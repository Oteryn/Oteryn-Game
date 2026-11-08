//! Test-support visibility for the unchanged, transaction-bound Ground matcher.
pub async fn matches(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &crate::durability::spell_item_transaction::SpellItemScopeAuthority,
    target: &crate::durability::spell_items_abi::SpellGroundTarget,
    expected: &[oteryn_gameplay_quests::quest::world_callbacks::GroundStackEntry],
) -> Result<bool, crate::durability::DurabilityError> {
    super::ground_stack::matches(tx, authority, target, expected).await
}
