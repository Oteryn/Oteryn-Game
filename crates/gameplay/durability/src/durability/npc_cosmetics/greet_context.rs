//! Retained data from a consumed genuine conversation-owner greeting witness.
//! This DTO is not a wire claim or a constructor for the dispatch witness.
#[derive(Clone, Debug)]
pub struct NpcSourceGreetContext {
    pub game_session_id: crate::foundation::GameSessionId,
    pub connection_generation: u64,
    pub command_id: u64,
    pub actor_identity: [u8; 16],
    pub npc_identity: [u8; 16],
    pub npc_generation: u64,
    pub catalogue_revision: String,
    pub conversation_generation: u64,
    pub movement_revision: u64,
}
impl NpcSourceGreetContext {
    pub(crate) fn observation(
        &self,
        fence: &crate::durability::character_progression::CurrentCharacterGameplayFence,
        request: &super::NpcCosmeticRequest,
    ) -> Result<serde_json::Value, crate::durability::DurabilityError> {
        use crate::durability::DurabilityError;
        if self.game_session_id != fence.game_session_id
            || self.game_session_id != request.command.game_session_id()
            || self.connection_generation != fence.connection_generation.get()
            || self.command_id != request.command.command_id().get()
            || self.catalogue_revision.is_empty()
            || self.catalogue_revision.len() > 256
            || self.catalogue_revision.chars().any(char::is_control)
            || self.movement_revision == 0
            || self.actor_identity == [0; 16]
            || self.npc_identity == [0; 16]
            || self.npc_generation == 0
            || self.conversation_generation == 0
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let hex = |bytes: &[u8]| {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        Ok(serde_json::json!({
            "game_session_id":sqlx::types::Uuid::from_bytes(*self.game_session_id.as_bytes()).to_string(),
            "connection_generation":self.connection_generation.to_string(),
            "command_id":self.command_id.to_string(),
            "actor_identity":hex(&self.actor_identity),
            "npc_identity":hex(&self.npc_identity),
            "npc_generation":self.npc_generation.to_string(),
            "catalogue_revision":self.catalogue_revision,
            "conversation_generation":self.conversation_generation.to_string(),
            "movement_revision":self.movement_revision.to_string(),
        }))
    }
}
