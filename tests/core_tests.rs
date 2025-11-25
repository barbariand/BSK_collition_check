use containtment_check::analyzer::analyze_voting_groups;
use containtment_check::parser::parse_html_boards;
use containtment_check::types::{Board, Member};
use std::collections::HashSet;

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

// --- LOGIKTESTER MED FRÅNVARO ---

#[test]
fn test_voting_logic_no_absence() {
    // Scenario: 5 ledamöter. Kvorum = 3.
    // Styrelse A har 2 jäviga (P1, P2).
    // 3 opartiska kvar. 3 >= 3. Borde gå bra.
    let fum = make_board("FUM", "24/25", vec!["P1", "P2", "P3", "P4", "P5"]);
    let b1 = make_board("StyrelseA", "23/24", vec!["P1", "P2"]);

    let all = vec![&fum, &b1];
    let absent = HashSet::new();

    let res = analyze_voting_groups(&fum, &all, &absent);

    assert_eq!(res.total_seats, 5);
    assert_eq!(res.present_count, 5);
    assert_eq!(res.quorum_limit, 3);
    assert_eq!(res.groups.len(), 1);
    assert!(res.impossible.is_empty());
}

#[test]
fn test_voting_logic_absence_causes_failure() {
    // Scenario: 5 ledamöter. Kvorum = 3.
    // Styrelse A har 2 jäviga (P1, P2).
    // P3 är frånvarande.
    // Närvarande: {P1, P2, P4, P5} (4 st).
    // Jäviga närvarande: {P1, P2}.
    // Röstberättigade: {P4, P5} (2 st).
    // 2 < 3. Styrelse A går inte att ansvarsbefria.
    let fum = make_board("FUM", "24/25", vec!["P1", "P2", "P3", "P4", "P5"]);
    let b1 = make_board("StyrelseA", "23/24", vec!["P1", "P2"]);

    let all = vec![&fum, &b1];
    let mut absent = HashSet::new();
    absent.insert("P3".to_string());

    let res = analyze_voting_groups(&fum, &all, &absent);

    assert_eq!(res.present_count, 4);
    assert_eq!(res.groups.len(), 0);
    assert_eq!(res.impossible.len(), 1); // Den hamnar i impossible
    assert_eq!(res.impossible[0].name, "StyrelseA");
}

#[test]
fn test_voting_logic_conflicted_person_is_absent() {
    // Scenario: 5 ledamöter. Kvorum = 3.
    // Styrelse A har 2 jäviga (P1, P2).
    // P1 är frånvarande.
    // Närvarande: {P2, P3, P4, P5} (4 st).
    // Jäviga närvarande: {P2} (P1 är ju borta).
    // Röstberättigade: {P3, P4, P5} (3 st).
    // 3 >= 3. Det funkar! (Eftersom den jäviga ändå inte är där).
    let fum = make_board("FUM", "24/25", vec!["P1", "P2", "P3", "P4", "P5"]);
    let b1 = make_board("StyrelseA", "23/24", vec!["P1", "P2"]);

    let all = vec![&fum, &b1];
    let mut absent = HashSet::new();
    absent.insert("P1".to_string()); // En av de jäviga är borta

    let res = analyze_voting_groups(&fum, &all, &absent);

    assert_eq!(res.present_count, 4);
    assert_eq!(res.groups.len(), 1); // Nu går det!
    assert!(res.impossible.is_empty());
}

#[test]
fn test_voting_logic_grouping_split_by_absence() {
    // Scenario:
    // FUM: 10 pers. Kvorum 5.
    // B1 jäv: {A, B, C}
    // B2 jäv: {D, E, F}
    // Om alla är där:
    // Grupp 1 (B1+B2): Jäv {A,B,C,D,E,F} (6 st). Röstberättigade: 10-6=4. 4 < 5. Fail.
    // Måste delas upp.

    // MEN, om A, B, C är frånvarande?
    // Närvarande: 7 st.
    // B1+B2 jäv närvarande: {D, E, F} (3 st). (A,B,C är borta).
    // Röstberättigade: 7 - 3 = 4. 4 < 5. Fail ändå.

    // Låt oss testa ett fall där frånvaro GÖR att de kan grupperas.
    // FUM: 6 pers {A,B,C,D,E,F}. Kvorum 3.
    // B1 jäv: {A}
    // B2 jäv: {B}
    // Båda närvarande: Jäv {A,B}. Röst: 6-2=4. 4 >= 3. Kan grupperas.

    // Fall: "Marginalen".
    // FUM: 5 pers {A,B,C,D,E}. Kvorum 3.
    // B1 jäv: {A}
    // B2 jäv: {B}
    // Om alla närvarande: Jäv {A,B}. Röst: 5-2=3. 3>=3. Kan grupperas.

    // Om C är borta (Opartisk borta):
    // Närvarande: 4 {A,B,D,E}.
    // Jäv {A,B}. Röst: 4-2=2. 2 < 3. Kan INTE grupperas. Måste splittas.

    let fum = make_board("FUM", "24/25", vec!["A", "B", "C", "D", "E"]);
    let b1 = make_board("B1", "23/24", vec!["A"]);
    let b2 = make_board("B2", "23/24", vec!["B"]);
    let all = vec![&fum, &b1, &b2];

    // Fall 1: Ingen frånvaro -> 1 grupp
    let res1 = analyze_voting_groups(&fum, &all, &HashSet::new());
    assert_eq!(res1.groups.len(), 1);

    // Fall 2: C borta -> 2 grupper (för att klara kvorum var för sig)
    // B1 ensam: Närvarande 4. Jäv {A}. Röst 3. OK.
    // B2 ensam: Närvarande 4. Jäv {B}. Röst 3. OK.
    // B1+B2:    Närvarande 4. Jäv {A,B}. Röst 2. FAIL.
    let mut absent = HashSet::new();
    absent.insert("C".to_string());
    let res2 = analyze_voting_groups(&fum, &all, &absent);

    assert_eq!(res2.groups.len(), 2);
}

// ... (behåll parsing-testerna om du vill, men de behöver också uppdateras för att använda den nya funktionen som kräver wrapper)
