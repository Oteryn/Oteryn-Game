use super::super::admission_authority_guards::uuid_text as text;
use super::*;
use crate::durability::bank::new_entry_id;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::CommandRef;
use crate::foundation::WorldId;
use oteryn_gameplay_housing::house_auction::value::ValuePlan;
use serde_json::json;

pub(in crate::durability) struct ValueWrite<'a> {
    pub operation: [u8; 16],
    pub auction: [u8; 16],
    pub world: WorldId,
    pub house: &'a str,
    pub account: [u8; 16],
    pub character: [u8; 16],
    pub kind: i16,
    pub binding: [u8; 32],
    pub command: Option<CommandRef>,
    pub fence: Option<&'a super::super::item_transfer::CurrentCharacterItemFence>,
    pub maximum: u64,
    pub first_rent: u64,
    pub price: u64,
    pub reached: u64,
    pub ends_at_us: i64,
    pub now_us: i64,
    pub plan: &'a ValuePlan,
    pub eligibility: &'a [super::eligibility::Candidate],
    pub ownership_before: u64,
}

pub(super) async fn apply_value(
    tx: &mut Tx<'_>,
    node: &NodeIncarnationProof,
    write: ValueWrite<'_>,
) -> Result<()> {
    let mut lines = Vec::with_capacity(write.plan.lines.len());
    for line in &write.plan.lines {
        let entry = new_entry_id(tx).await?;
        lines.push(
            json!({"entry":text(&entry),"kind":line.kind as i16,"amount":line.amount,
            "balance_before":line.balance_before,"balance_after":line.balance_after,
            "escrow_before":line.escrow_before,"escrow_after":line.escrow_after}),
        );
    }
    let event = new_entry_id(tx).await?;
    let envelope = super::super::house_ownership_audit::encode(&write, event)?;
    let p = json!({"operation":text(&write.operation),"auction":text(&write.auction),
        "world":text(write.world.as_bytes()),"house":write.house,"account":text(&write.account),
        "character":text(&write.character),"kind":write.kind,"binding":write.binding.as_slice(),
        "session":write.command.map(|c|text(c.game_session_id().as_bytes())),
        "command":write.command.map(|c|c.command_id().get().to_string()),
        "connection_generation":write.fence.map(|f|f.connection_generation.get().to_string()),
        "lease_generation":write.fence.map(|f|f.character_lease_generation.to_string()),
        "scope_generation":write.fence.map(|f|f.scope_ownership_generation.get().to_string()),
        "maximum":write.maximum,"first_rent":write.first_rent,"price":write.price,"reached":write.reached,
        "ends_at_us":write.ends_at_us,"now_us":write.now_us,"lines":lines,
        "balance_before":write.plan.lines.first().map_or(write.plan.balance,|l|l.balance_before),
        "balance_after":write.plan.balance,"escrow_before":write.plan.lines.first().map_or(write.plan.escrow,|l|l.escrow_before),
        "escrow_after":write.plan.escrow,"event":text(&event)});
    let fact = node.fact();
    sqlx::query("SELECT game_house_value_apply($1,encode($2,'hex')::uuid,$3::text::numeric,$4,$5)")
        .bind(p)
        .bind(fact.node_id().as_bytes().as_slice())
        .bind(fact.registration_revision().to_string())
        .bind(node.sql_possession_secret())
        .bind(envelope)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
