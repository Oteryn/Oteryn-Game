use super::*;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
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
            TeamFinderResult::Browse { listings, .. } => {
                listings.get(self.selected_row).map(|v| v.leader_handle)
            }
            TeamFinderResult::Applicants { members, .. } => {
                members.get(self.selected_row).map(|v| v.handle)
            }
        }
    }
}
