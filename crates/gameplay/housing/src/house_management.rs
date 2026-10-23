//! Native management values. Transport never supplies current ownership or access.

pub const HOUSE_ACL_MEMBERS_MAX: usize = 200;
pub const DAY_MS: u64 = 86_400_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseManagementList {
    Guest,
    Subowner,
    Door { x: u16, y: u16, floor: i8 },
}
impl HouseManagementList {
    pub fn key(self) -> Option<String> {
        match self {
            Self::Guest => Some("guest".into()),
            Self::Subowner => Some("subowner".into()),
            Self::Door { x, y, floor } if (-15..=0).contains(&floor) => {
                Some(format!("door:{x}:{y}:{floor}"))
            }
            Self::Door { .. } => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseManagementAction {
    ReplaceAcl {
        list: HouseManagementList,
        expected_revision: u64,
        members: Vec<[u8; 16]>,
    },
    MoveOut {
        due_at_unix_ms: u64,
    },
    CancelMoveOut,
}
/// OWN0 §6: absolute dates, one through thirty days from the durable clock.
pub fn valid_move_out_date(now_ms: u64, due_ms: u64) -> bool {
    due_ms
        .checked_sub(now_ms)
        .is_some_and(|delay| (DAY_MS..=30 * DAY_MS).contains(&delay))
}
#[cfg(test)]
#[path = "house_management_tests.rs"]
mod tests;
