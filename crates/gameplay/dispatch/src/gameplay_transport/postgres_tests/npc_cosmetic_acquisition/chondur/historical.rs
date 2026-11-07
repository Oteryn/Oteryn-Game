//! Explicit historical owned-material fixture; never a natural Stampor drop claim.
use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::RewardClaimMintRequest;
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::generation_item_facts;
pub(super) async fn mint_materials(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("actual native source")?;
    let definition = |key: &str| TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    };
    for (ordinal, item, quantity) in [(0, 12312, 50), (1, 12314, 30), (2, 12313, 100)] {
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("current historical operand fence")?;
        let request = RewardClaimMintRequest {
            command: CommandRef::new(
                admitted.game_session_id,
                CommandId::new(1000 + ordinal).map_err(debug)?,
            ),
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: format!(
                    "fixture:reward.srv7-historically-owned-stampor-material-{item}"
                ),
                revision_ref: "definition-r1".into(),
            },
            source_placement: "fixture:historically-owned-stampor-material-not-natural-producer"
                .into(),
            item: generation_item_facts(native, &definition(&format!("oteryn:item.tibia.i{item}")))
                .map_err(debug)?,
            quantity,
            contents: Vec::new(),
            root_text: None,
            backpack: generation_item_facts(native, &definition("oteryn:item.tibia.i2854"))
                .map_err(debug)?,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        };
        // Genuine mature Root MINT + audit/custody proof. Only its historical
        // acquisition provenance is isolated; no donor drop odds are asserted.
        let mut candidate = owner
            .root
            .freeze_reward_claim_mint(owner.character, owner.holder, fence.item_fence(), request)
            .await
            .map_err(debug)?;
        let uuid = *candidate.item_instance_id();
        let mint = *candidate.transaction_id();
        owner
            .root
            .commit_reward_claim_mint(
                owner.character,
                owner.holder,
                fence.item_fence(),
                &mut candidate,
            )
            .await
            .map_err(debug)?;
        let proven:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_reward_claim_mint_receipts r JOIN game_item_instances i USING(item_instance_id) JOIN game_item_audit_outbox a USING(transaction_id,event_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND i.minted_transaction_id=encode($2,'hex')::uuid AND i.definition_production_key=$3 AND i.lifecycle=1 AND i.quantity=$4 AND r.envelope_sha256=a.envelope_sha256 AND a.envelope_sha256=sha256(a.envelope))").bind(uuid.as_slice()).bind(mint.as_slice()).bind(format!("oteryn:item.tibia.i{item}")).bind(i64::from(quantity)).fetch_one(&h.pool).await?;
        assert!(
            proven,
            "historical material has a genuine Root MINT/audit receipt"
        );

        admitted.item_fence = Some(
            owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(debug)?
                .ok_or("after material MINT fence")?
                .item_fence(),
        );
    }
    Ok(())
}
