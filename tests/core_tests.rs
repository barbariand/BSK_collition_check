use containtment_check::analyzer::analyze_voting_groups;
use containtment_check::parser::parse_html_boards;
use containtment_check::types::{Board, Member};

fn make_board(name: &str, year: &str, member_names: Vec<&str>) -> Board {
    Board {
        name: name.to_string(),
        year: year.to_string(),
        members: member_names
            .into_iter()
            .map(|n| Member {
                position: "Ledamot".to_string(),
                name: n.to_string(),
            })
            .collect(),
    }
}

#[test]
fn test_algorithm_basic_packing() {
    let fum = make_board(
        "FUM",
        "24/25",
        vec!["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"],
    );
    let b1 = make_board("KIDS", "24/25", vec!["A", "B", "C"]);
    let b2 = make_board("SIT", "24/25", vec!["D", "E", "F"]);

    let all = vec![&fum, &b1,& b2];
    let res = analyze_voting_groups(&fum, &all);

    assert_eq!(res.groups.len(), 2);
}

#[test]
fn test_html_parsing_snippet() {
    let html = r#"
    <section>
        <h2>Förtroendevalda 2025/2026</h2>
        <div class="elementor-tabs-wrapper">
            <div class="elementor-tab-desktop-title">TestStyrelse</div>
        </div>
        <div class="elementor-tabs-content-wrapper">
            <div class="elementor-tab-content">
                <p>Ordförande: Cindy Nilsson<br />Ledamot: Winnow Master</p>
            </div>
        </div>
    </section>
    "#;

    let boards = parse_html_boards(html).expect("Should parse");
    assert_eq!(boards.len(), 1);
    assert_eq!(boards[0].name, "TestStyrelse");
    assert_eq!(boards[0].year, "2025/2026");
    assert_eq!(boards[0].members.len(), 2);
    assert_eq!(boards[0].members[0].name, "Cindy Nilsson");
    assert_eq!(boards[0].members[1].name, "Winnow Master");
}
