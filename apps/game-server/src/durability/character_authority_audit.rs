//! Registered Character Authority durable-audit encoding.

use prost::Message;

pub const EVENT_TYPE_CHARACTER_AUTHORITY_BOOTSTRAPPED: u32 = 1;
pub const EVENT_SCHEMA_REVISION: u32 = 1;
pub const RETENTION_PROFILE: &str = "CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1";

#[derive(Clone, PartialEq, Message)]
pub struct CharacterAuthorityBootstrappedV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub account_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub world_id: Vec<u8>,
    #[prost(enumeration = "CharacterLifecycleV1", tag = "4")]
    pub lifecycle: i32,
    #[prost(uint64, tag = "5")]
    pub character_revision: u64,
    #[prost(enumeration = "CharacterAuthorityTransitionV1", tag = "6")]
    pub transition: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum CharacterLifecycleV1 {
    Unspecified = 0,
    Active = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum CharacterAuthorityTransitionV1 {
    Unspecified = 0,
    InitialBootstrapCreate = 1,
}

pub fn encode_bootstrap(
    account_id: &[u8; 16],
    character_id: &[u8; 16],
    world_id: &[u8; 16],
) -> Vec<u8> {
    CharacterAuthorityBootstrappedV1 {
        account_id: account_id.to_vec(),
        character_id: character_id.to_vec(),
        world_id: world_id.to_vec(),
        lifecycle: CharacterLifecycleV1::Active as i32,
        character_revision: 1,
        transition: CharacterAuthorityTransitionV1::InitialBootstrapCreate as i32,
    }
    .encode_to_vec()
}
