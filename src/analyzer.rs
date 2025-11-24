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
    pub total_fum_count: usize,
    pub quorum_limit: usize,
}

pub fn analyze_voting_groups<'a>(fum: &Board, all_boards: &'a [&'a Board]) -> AnalysisResult<'a> {
    let fum_names: HashSet<String> = fum.members.iter().map(|m| m.name.clone()).collect();
    let total_fum_count = fum_names.len();

    let quorum_limit = (total_fum_count as f64 / 2.0).ceil() as usize;

    let mut voting_groups: Vec<VotingGroup> = Vec::new();
    let mut impossible_boards: Vec<&Board> = Vec::new();

    let targets: Vec<&&Board> = all_boards
        .into_iter()
        .filter(|b| b.name != fum.name)
        .collect();

    for target_board in targets {
        let conflicts: HashSet<String> = target_board
            .members
            .iter()
            .filter(|m| fum_names.contains(&m.name))
            .map(|m| m.name.clone())
            .collect();

        let remaining_if_alone = total_fum_count - conflicts.len();
        if remaining_if_alone < quorum_limit {
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
            let remaining_voters = total_fum_count - union_conflicts.len();

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
        total_fum_count,
        quorum_limit,
    }
}
