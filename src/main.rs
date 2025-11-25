use clap::Parser;
use colored::*;
use containtment_check::analyzer::{AnalysisResult, analyze_voting_groups};
use containtment_check::parser::parse_html_boards;
use containtment_check::types::{Board, Member};
use std::collections::HashSet;
use std::fs;
use tracing::{error, info, warn};
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "https://bthstudent.se/studentkaren/fortroendevalda/"
    )]
    source: String,

    #[arg(short, long, default_value = "Fullmäktige")]
    base_board: String,

    #[arg(short, long)]
    voting_year: Option<String>,

    #[arg(long, value_delimiter = ',')]
    absent: Vec<String>,

    #[arg(long, default_value_t = 3)]
    le_threshold: usize,

    #[arg(long, value_delimiter = ',')]
    priority: Vec<String>,
}

fn main() {
    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .init();

    let args = Args::parse();

    let content = fetch_content(&args.source);
    info!("Parsar HTML-innehåll...");

    let mut boards = match parse_html_boards(&content) {
        Ok(b) => {
            info!("Hittade {} styrelser totalt.", b.len());
            b
        }
        Err(e) => {
            error!("Kritisk fel vid parsning: {}", e);
            return;
        }
    };

    let (fum_name, fum_year, fum_member_names) = {
        let candidates: Vec<&Board> = boards
            .iter()
            .filter(|b| b.name == args.base_board)
            .collect();

        let fum_opt = if let Some(y) = &args.voting_year {
            candidates.into_iter().find(|b| &b.year == y)
        } else {
            candidates.into_iter().max_by_key(|b| b.year.clone())
        };

        match fum_opt {
            Some(b) => (
                b.name.clone(),
                b.year.clone(),
                b.members.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
            ),
            None => {
                error!("Kunde inte hitta huvudstyrelsen '{}'.", args.base_board);
                return;
            }
        }
    };

    let absent_set = process_absences(&fum_member_names, &args.absent);
    print_fum_info(&fum_name, &fum_year, &fum_member_names, &absent_set);

    let target_year = match get_previous_year(&fum_year) {
        Ok(y) => y,
        Err(e) => {
            error!("Fel vid årsberäkning: {}", e);
            return;
        }
    };

    println!(
        "\n{}",
        format!("GRANSKAR VERKSAMHETSÅRET: {}", target_year)
            .bold()
            .underline()
    );
    println!("------------------------------------------------");

    apply_fuzzy_corrections(
        &mut boards,
        &fum_member_names,
        &target_year,
        args.le_threshold,
    );

    let fum_ref = boards
        .iter()
        .find(|b| b.name == fum_name && b.year == fum_year)
        .expect("FUM borde finnas kvar");

    let mut boards_to_audit: Vec<_> = boards.iter().filter(|b| b.year == target_year).collect();

    if boards_to_audit.is_empty() {
        warn!("Hittade inga styrelser för året {}.", target_year);
        return;
    }

    if !args.priority.is_empty() {
        let priority_set: HashSet<String> = args
            .priority
            .iter()
            .map(|s| s.trim().to_lowercase())
            .collect();

        boards_to_audit.sort_by_key(|b| !priority_set.contains(&b.name.to_lowercase()));

        println!("{}", "PRIORITERING AKTIVERAD".blue().bold());
        println!("Följande styrelser behandlas först:");
        for p in &args.priority {
            println!("  -> {}", p.yellow());
        }
        println!("------------------------------------------------\n");
    }

    let analysis = analyze_voting_groups(fum_ref, &boards_to_audit, &absent_set);
    print_analysis_results(&analysis, &absent_set);
}

fn apply_fuzzy_corrections(
    boards: &mut [Board],
    correct_names: &[String],
    target_year: &str,
    threshold: usize,
) {
    println!(
        "{}",
        "ANALYS OCH KORRIGERING AV NAMN (Fuzzy Match)".blue().bold()
    );
    println!("Jämför styrelsemedlemmar mot FUM-listan för att hitta stavfel...");
    println!("------------------------------------------------");

    let mut corrections_made = false;

    for board in boards.iter_mut() {
        if board.year != target_year {
            continue;
        }
        for member in &mut board.members {
            if correct_names
                .iter()
                .any(|n| n.eq_ignore_ascii_case(&member.name))
            {
                continue;
            }
            let mut best_match: Option<&String> = None;
            let mut best_dist = usize::MAX;
            for correct in correct_names {
                let dist =
                    strsim::levenshtein(&member.name.to_lowercase(), &correct.to_lowercase());
                if dist < best_dist {
                    best_dist = dist;
                    best_match = Some(correct);
                }
            }

            if let Some(correct) = best_match {
                if best_dist > 0 && best_dist <= threshold {
                    println!("{}", "[KORRIGERING]".yellow().bold());
                    println!("  Plats:    {} ({})", board.name.cyan(), board.year.cyan());
                    println!("  Hittade:  '{}'", member.name.red());
                    println!("  Ändrar till: '{}' (FUM-ledamot)", correct.green());
                    println!("  Avstånd:  {} tecken", best_dist);
                    println!();

                    member.name = correct.clone();
                    corrections_made = true;
                }
            }
        }
    }

    if !corrections_made {
        println!("{}", "[OK] Inga namn behövde korrigeras.".green());
    }
    println!("------------------------------------------------\n");
}

fn fetch_content(source: &str) -> String {
    if source.starts_with("http") {
        info!("Laddar ner HTML från URL: {}", source);
        reqwest::blocking::get(source)
            .and_then(|r| r.text())
            .expect("Kunde inte läsa svaret från servern")
    } else {
        info!("Läser fil: {}", source);
        fs::read_to_string(source).expect("Kunde inte läsa filen")
    }
}

fn find_voting_board<'a>(
    boards: &'a [Board],
    base_name: &str,
    requested_year: Option<&str>,
) -> Option<&'a Board> {
    let candidates: Vec<_> = boards.iter().filter(|b| b.name == base_name).collect();

    if let Some(y) = requested_year {
        candidates.into_iter().find(|b| b.year == y)
    } else {
        candidates.into_iter().max_by_key(|b| b.year.clone())
    }
}

fn process_absences(valid_fum_names: &[String], requested_absent: &[String]) -> HashSet<String> {
    let mut absent_set = HashSet::new();
    let valid_names_set: HashSet<&String> = valid_fum_names.iter().collect();

    if !requested_absent.is_empty() {
        println!("{}", "FRÅNVAROHANTERING".blue().bold());
        for name in requested_absent {
            let trimmed = name.trim();
            if let Some(real_name) = valid_names_set
                .iter()
                .find(|n| n.eq_ignore_ascii_case(trimmed))
            {
                println!("  [INFO] {} markeras som frånvarande.", real_name.yellow());
                absent_set.insert(real_name.to_string());
            } else {
                println!(
                    "  {}",
                    format!(
                        "[FEL] Kunde inte hitta '{}' i FUM-listan. Kontrollera stavning!",
                        trimmed
                    )
                    .red()
                );
            }
        }
        println!();
    }
    absent_set
}

fn print_fum_info(name: &str, year: &str, members: &[String], absent_set: &HashSet<String>) {
    println!("------------------------------------------------");
    println!("RÖSTANDE ORGAN: {} ({})", name.green().bold(), year.green());

    let mut sorted_members = members.to_vec();
    sorted_members.sort();

    println!("Ledamöter (Totalt {}):", sorted_members.len());
    for name in sorted_members {
        if absent_set.contains(&name) {
            println!("  - {} {}", name.dimmed(), "(FRÅNVARANDE)".red());
        } else {
            println!("  * {}", name);
        }
    }
}

fn print_analysis_results(analysis: &AnalysisResult, absent_set: &HashSet<String>) {
    println!("{}", "ANALYSRESULTAT".blue().bold());
    println!("Mandat i FUM: {}", analysis.total_seats);
    println!(
        "Närvarande på mötet: {}",
        analysis.present_count.to_string().bold()
    );
    println!(
        "Kvorumgräns (krävs för beslut): {}\n",
        analysis.quorum_limit.to_string().bold()
    );

    if analysis.present_count < analysis.quorum_limit {
        println!("{}", "!!! MÖTET EJ BESLUTSMÄSSIGT !!!".red().bold().blink());
        println!(
            "För få närvarande ledamöter ({}) för att nå kvorum ({}).",
            analysis.present_count, analysis.quorum_limit
        );
        return;
    }

    if !analysis.impossible.is_empty() {
        println!(
            "{}",
            "!!! VARNING: Följande kan INTE tas upp (för få röstberättigade kvar) !!!"
                .red()
                .bold()
        );

        let mut sorted_impossible = analysis.impossible.clone();
        sorted_impossible.sort_by(|a, b| a.name.cmp(&b.name));

        for b in &sorted_impossible {
            println!("  - {} ({})", b.name.red(), b.year.red());
        }
        println!();
    }

    for (i, group) in analysis.groups.iter().enumerate() {
        let conflicts_present_count = group
            .conflicted_members
            .iter()
            .filter(|n| !absent_set.contains(*n))
            .count();

        let eligible = analysis.present_count - conflicts_present_count;
        let group_header = format!("GRUPP {}: ({} röstberättigade)", i + 1, eligible);

        println!("{}", group_header.green().bold());
        println!("  (Krav för beslut: {} st)", analysis.quorum_limit);

        let mut conflicts: Vec<_> = group.conflicted_members.iter().collect();
        conflicts.sort();

        println!("  Jäviga ledamöter i denna grupp:");
        if conflicts.is_empty() {
            println!("    (Inga)");
        } else {
            for name in conflicts {
                if absent_set.contains(name) {
                    println!("    - {} {}", name.dimmed(), "(Frånvarande)".italic());
                } else {
                    println!("    - {} {}", name.red(), "(Närvarande, får ej rösta)");
                }
            }
        }
        println!();
        println!("  Styrelser:");

        let mut sorted_boards = group.boards.clone();
        sorted_boards.sort_by(|a, b| a.name.cmp(&b.name));

        for b in &sorted_boards {
            println!("    * {} ({})", b.name.cyan(), b.year.white().dimmed());
            let board_conflicts: Vec<_> = b
                .members
                .iter()
                .filter(|m| group.conflicted_members.contains(&m.name))
                .map(|m| m.name.as_str())
                .collect();
            if !board_conflicts.is_empty() {
                let conflict_str = board_conflicts.join(", ");
                println!("      -> Jäv: {}", conflict_str.red());
            }
        }
        println!("------------------------------------------------");
    }
}

fn get_previous_year(current_year: &str) -> Result<String, String> {
    let parts: Vec<&str> = current_year.split('/').collect();
    if parts.len() != 2 {
        return Err("Fel format på årtal".to_string());
    }
    let s: i32 = parts[0].parse().map_err(|_| "Fel startår")?;
    let e: i32 = parts[1].parse().map_err(|_| "Fel slutår")?;
    Ok(format!("{}/{}", s - 1, e - 1))
}
