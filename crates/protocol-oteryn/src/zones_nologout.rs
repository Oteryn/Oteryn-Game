//! ZONES-NOLOGOUT-1: World login text and canonical logout refusal notices.
//! Domain 57 reuses `TutorialNoticeV1` status-text encoding; an empty snapshot is `[8, 1]`.
pub const CAPABILITY_ZONES_NOLOGOUT_V1: u32 = 92;
pub const STATE_DOMAIN_SERVER_MESSAGES: u32 = 57;
pub const MAX_SERVER_MOTD_BYTES: usize = 255;
pub const NO_LOGOUT_MESSAGE: &str = "You can not logout here.";
pub const IN_FIGHT_MESSAGE: &str = "You may not logout during or immediately after a fight!";
