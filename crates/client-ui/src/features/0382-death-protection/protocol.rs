use super::DeathProtectionReply;
pub fn decode(payload: &[u8]) -> Option<DeathProtectionReply> {
    DeathProtectionReply::decode(payload)
}
