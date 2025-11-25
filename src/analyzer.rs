use crate::types::Board;
use std::collections::HashSet;

#[derive(Debug)]
pub struct VotingGroup<'a> {
    pub boards: Vec<&'a Board>,
    pub conflicted_members: HashSet<String>,
}

#[derive(Debug)]
pub struct AnalysisResult<'a> {
    pub groups: Vec<VotingGroup<'a>>,
    pub impossible: Vec<&'a Board>,
    pub total_seats: usize,
    pub present_count: usize,
    pub quorum_limit: usize,
}

pub fn analyze_voting_groups<'a>(
    fum: &Board,
    all_boards: &'a [&'a Board],
    absent_members: &HashSet<String>,
) -> AnalysisResult<'a> {
    let fum_names_all: HashSet<String> = fum.members.iter().map(|m| m.name.clone()).collect();
    let total_seats = fum_names_all.len();

    let quorum_limit = (total_seats as f64 / 2.0).ceil() as usize;

    let present_fum_names: HashSet<String> =
        fum_names_all.difference(absent_members).cloned().collect();

    let present_count = present_fum_names.len();

    let mut voting_groups: Vec<VotingGroup> = Vec::new();
    let mut impossible_boards: Vec<&Board> = Vec::new();

    let targets: Vec<&&Board> = all_boards.into_iter().collect();

    for target_board in targets {
        let conflicts: HashSet<String> = target_board
            .members
            .iter()
            .filter(|m| fum_names_all.contains(&m.name))
            .map(|m| m.name.clone())
            .collect();

        let conflicts_present: HashSet<_> = conflicts.intersection(&present_fum_names).collect();
        let eligible_voters = present_count - conflicts_present.len();

        if eligible_voters < quorum_limit {
            impossible_boards.push(target_board);
            continue;
        }

        let mut placed = false;
        for group in &mut voting_groups {
            let union_conflicts: HashSet<_> = group
                .conflicted_members
                .union(&conflicts)
                .cloned()
                .collect();

            let union_conflicts_present: HashSet<_> =
                union_conflicts.intersection(&present_fum_names).collect();
            let remaining_voters = present_count - union_conflicts_present.len();

            if remaining_voters >= quorum_limit {
                group.boards.push(target_board);
                group.conflicted_members = union_conflicts;
                placed = true;
                break;
            }
        }

        if !placed {
            voting_groups.push(VotingGroup {
                boards: vec![target_board],
                conflicted_members: conflicts,
            });
        }
    }

    AnalysisResult {
        groups: voting_groups,
        impossible: impossible_boards,
        total_seats,
        present_count,
        quorum_limit,
    }
}
