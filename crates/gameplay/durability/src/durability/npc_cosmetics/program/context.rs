//! Closed, already admitted source parent for the same-transaction FIRE child.
pub(crate) struct NpcProgramContext<'a> {
    pub fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    pub command: crate::foundation::CommandRef,
    pub npc_key: &'a str,
    pub source_key: &'a str,
    pub program_sha256: [u8; 32],
    pub plan_sha256: [u8; 32],
    pub fee_transaction_id: Option<[u8; 16]>,
    pub npc_transaction: [u8; 16],
    pub next_revision: u64,
    pub native: &'a oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
}
