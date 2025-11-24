use clap::Parser;
use containtment_check::analyzer::analyze_voting_groups;
use containtment_check::parser::parse_html_boards;
use containtment_check::types::Board;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file: PathBuf,

    #[arg(short, long, default_value = "FUM")]
    base_board: String,

    #[arg(short, long)]
    voting_year: Option<String>,
}

fn main() {
    let args = Args::parse();

    println!("Läser fil: {:?}", args.file);
    let content = fs::read_to_string(&args.file).expect("Kunde inte läsa filen");

    println!("Parsar HTML...");
    let boards = match parse_html_boards(&content) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Fel vid parsning: {}", e);
            return;
        }
    };

    println!("Hittade {} styrelser totalt.", boards.len());

    let voting_fum_candidates: Vec<_> = boards
        .iter()
        .filter(|b| b.name == args.base_board)
        .collect();

    let voting_fum = if let Some(y) = &args.voting_year {
        voting_fum_candidates.into_iter().find(|b| &b.year == y)
    } else {
        voting_fum_candidates.first().cloned()
    };

    if let Some(fum) = voting_fum {
        println!("------------------------------------------------");
        println!("RÖSTANDE ORGAN: {} ({})", fum.name, fum.year);

        let target_year = match get_previous_year(&fum.year) {
            Ok(y) => y,
            Err(e) => {
                eprintln!(
                    "Kunde inte räkna ut föregående år från '{}': {}",
                    fum.year, e
                );
                return;
            }
        };

        println!("GRANSKAR VERKSAMHETSÅRET: {}", target_year);
        println!("------------------------------------------------");

        let boards_to_audit: Vec<_> = boards
            .iter()
            .filter(|b| b.year == target_year && b.name != args.base_board)
            .collect();

        if boards_to_audit.is_empty() {
            println!("Hittade inga styrelser för året {}.", target_year);
            return;
        }

        let analysis = analyze_voting_groups(fum, &boards_to_audit);

        println!(
            "Antal FUM-ledamöter (röstberättigade totalt): {}",
            analysis.total_fum_count
        );
        println!("Kvorumgräns (50%): {}\n", analysis.quorum_limit);

        if !analysis.impossible.is_empty() {
            println!(
                "!!! VARNING: Följande styrelser kan inte ansvarsbefrias pga för stort jäv (över 50% av FUM sitter i dem) !!!"
            );
            for b in &analysis.impossible {
                println!("  - {}", b.name);
            }
            println!();
        }

        for (i, group) in analysis.groups.iter().enumerate() {
            let eligible = analysis.total_fum_count - group.conflicted_members.len();
            println!("GRUPP {}: ({} röstberättigade kvar)", i + 1, eligible);
            println!("  Styrelser att besluta om:");
            for b in &group.boards {
                println!("    - {}", b.name);
            }

            let mut conflicts: Vec<_> = group.conflicted_members.iter().collect();
            conflicts.sort();
            println!("  Jäviga (måste lämna rummet): {:?}", conflicts);
            println!("------------------------------------------------");
        }
    } else {
        eprintln!("Kunde inte hitta huvudstyrelsen '{}'.", args.base_board);
        eprintln!("Hittade styrelser '{:?}'", boards);
    }
}

fn get_previous_year(current_year: &str) -> Result<String, String> {
    let parts: Vec<&str> = current_year.split('/').collect();
    if parts.len() != 2 {
        return Err("Felaktigt årsformat, förväntade YYYY/YYYY".to_string());
    }

    let start_year: i32 = parts[0].parse().map_err(|_| "Kunde inte parsa startår")?;
    let end_year: i32 = parts[1].parse().map_err(|_| "Kunde inte parsa slutår")?;

    Ok(format!("{}/{}", start_year - 1, end_year - 1))
}
