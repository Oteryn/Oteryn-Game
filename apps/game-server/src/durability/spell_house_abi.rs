//! Neutral House owner contract; no gameplay or spell implementation dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum HouseAccess {
    NotInvited,
    Guest,
    Subowner,
    Owner,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HouseList {
    Guest,
    Subowner,
    Door(u32),
}
pub(crate) mod house_presence_seal {
    pub(crate) trait Sealed {}
}
/// Produced only by the current physical World-House owner. A definition key,
/// persisted ACL or client editor token alone cannot implement live presence.
pub(crate) trait HousePresenceProof: house_presence_seal::Sealed {
    fn world(&self) -> crate::foundation::WorldId;
    fn actor_session(&self) -> crate::foundation::GameSessionId;
    fn scope_generation(&self) -> u64;
    fn content_digest(&self) -> [u8; 32];
    fn house_key(&self) -> &str;
    fn has_door(&self, id: u32) -> bool;
}
