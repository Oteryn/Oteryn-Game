use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub filter: TeamFilter,
    pub selected: bool,
    pub open: bool,
    pub pending: Option<u32>,
    pub applicants: bool,
    pub result: Option<TeamFinderResult>,
    pub selected_row: usize,
    pub feedback: String,
}
impl Panel {
    pub fn next(&self) -> Option<u64> {
        match self.result.as_ref()? {
            TeamFinderResult::Browse { next, .. } | TeamFinderResult::Applicants { next, .. } => {
                *next
            }
        }
    }
    pub fn handle(&self) -> Option<u64> {
        match self.result.as_ref()? {
            TeamFinderResult::Browse { listings, .. } => self
                .visible_indices()
                .get(self.selected_row)
                .and_then(|index| listings.get(*index))
                .map(|v| v.leader_handle),
            TeamFinderResult::Applicants { members, .. } => {
                members.get(self.selected_row).map(|v| v.handle)
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeamFilter {
    pub name: String,
    pub level: Option<u16>,
    /// Zero includes every vocation/target.
    pub vocation_mask: u8,
    pub target: u8,
    pub only_open: bool,
}
impl Panel {
    pub fn visible_indices(&self) -> Vec<usize> {
        match &self.result {
            Some(TeamFinderResult::Browse { listings, .. }) => {
                let name = self.filter.name.to_lowercase();
                listings
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| {
                        use crate::team_finder_leader::TeamTarget;
                        row.name.to_lowercase().contains(&name)
                            && self.filter.level.is_none_or(|level| {
                                (row.listing.min_level..=row.listing.max_level).contains(&level)
                            })
                            && (self.filter.vocation_mask == 0
                                || self.filter.vocation_mask & row.listing.vocation_mask != 0)
                            && (!self.filter.only_open || row.occupied_slots < row.listing.slots)
                            && (self.filter.target == 0
                                || self.filter.target
                                    == match row.listing.target {
                                        TeamTarget::Boss(_) => 1,
                                        TeamTarget::Hunt { .. } => 2,
                                        TeamTarget::Quest(_) => 3,
                                    })
                    })
                    .map(|(index, _)| index)
                    .collect()
            }
            Some(TeamFinderResult::Applicants { members, .. }) => (0..members.len()).collect(),
            None => Vec::new(),
        }
    }
    pub fn visible_names(&self) -> Vec<String> {
        let indices = self.visible_indices();
        match &self.result {
            Some(TeamFinderResult::Browse { listings, .. }) => indices
                .into_iter()
                .filter_map(|index| listings.get(index))
                .map(|row| {
                    lfmt!(
                        "{} — {}–{}; {} free; {:?}; {:?}",
                        row.name,
                        row.listing.min_level,
                        row.listing.max_level,
                        row.listing.slots.saturating_sub(row.occupied_slots),
                        row.listing.target,
                        row.status
                    )
                })
                .collect(),
            Some(TeamFinderResult::Applicants { members, .. }) => members
                .iter()
                .map(|row| lfmt!("{} — level {}; {:?}", row.name, row.level, row.status))
                .collect(),
            None => Vec::new(),
        }
    }
}
