//! Reserved raid schedule wire identifiers. Raid execution and announcements are
//! server driven; these commands remain behind the unoffered capability.
pub const CAPABILITY_RAID_SCHEDULE_V1: u32 = 149;
pub const COMMAND_TYPE_RAID_SCHEDULE_QUERY: u32 = 171;
pub const COMMAND_TYPE_RAID_SUBSCRIBE: u32 = 172;
pub const COMMAND_TYPE_RAID_NOTICE_ACK: u32 = 173;
