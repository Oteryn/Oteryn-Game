//! Shared reward mechanism proof; this fixture is not a source barrel binding.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{
    RewardClaimMintContent, RewardClaimMintError, RewardClaimMintRequest,
};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn l6_reward_container_preserves_mead_and_exact_replay() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED l6_reward_fluids");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::quest_world_callbacks::scenario(
                "l6-reward-fluids",
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|error| error.to_string())
        })?
        .join()
        .unwrap_or_else(|_| Err("filled reward fixture panicked".into()));
    eprintln!(
        "POSTGRES_CASE_COMPLETED l6_reward_fluids duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_reward_fluids version={version}");
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native policy")?;
    let facts = |id| {
        generation_item_facts(
            native,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{id}"),
                revision_ref: "definition-r1".into(),
            },
        )
        .map_err(debug)
    };
    let fence = admitted.item_fence.ok_or("current Item fence")?;
    let request = RewardClaimMintRequest {
        backpack_outputs: false,
        root_charges_remaining: None,
        root_fluid_subtype: None,
            additional_items: Vec::new(),
        command: CommandRef::new(
            admitted.game_session_id,
            CommandId::new(8000).map_err(debug)?,
        ),
        claim: TypedDefinitionRef {
            family: "RewardClaim".into(),
            production_key: "fixture:reward.filled-fluid-engine".into(),
            revision_ref: "definition-r1".into(),
        },
        source_placement: "fixture:placement.filled-fluid-engine".into(),
        item: facts(7343)?,
        quantity: 1,
        contents: [(2901, 1), (7439, 1), (3031, 82)]
            .into_iter()
            .map(|(id, quantity)| {
                Ok(RewardClaimMintContent {
                    charges_remaining: None,
                    fluid_subtype: (id == 2901).then_some(16),
                    item: facts(id)?,
                    quantity,
                    key_number: None,
                    text: None,
                })
            })
            .collect::<TestResult<Vec<_>>>()?,
        root_text: None,
        backpack: facts(2854)?,
        content_revision: format!(
            "sha256:{}",
            native
                .source_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
        achievement: None,
        quest_transition: None,
        key_number: None,
    };
    let weights = NativeItemWeights(native);
    let before = counts(h).await?;
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(matches!(
        h.root
            .freeze_reward_claim_mint_weighted(authority, &h.node, stale, request.clone(), &weights)
            .await,
        Err(RewardClaimMintError::AuthorityRejected)
    ));
    assert_eq!(counts(h).await?, before);
    let mut candidate = h
        .root
        .freeze_reward_claim_mint_weighted(authority, &h.node, fence, request.clone(), &weights)
        .await
        .map_err(debug)?;
    h.root
        .commit_reward_claim_mint_weighted_noticed(
            authority,
            &h.node,
            fence,
            &mut candidate,
            None,
            &weights,
        )
        .await
        .map_err(debug)?;
    // Crystal201189f src/utils/utils_definitions.hpp:456: FLUID_MEAD=16.
    // Keep this assertion in RED and GREEN; an empty waterskin is not mead.
    let fluid: Option<i16> = sqlx::query_scalar("SELECT f.fluid_subtype FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) LEFT JOIN game_item_fluid_states f USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i2901'")
        .bind(candidate.item_instance_id().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        fluid,
        Some(16),
        "the minted waterskin must contain source mead"
    );
    let committed = counts(h).await?;
    let mut changed = request.clone();
    changed.contents[0].fluid_subtype = Some(17);
    assert!(matches!(
        h.root
            .freeze_reward_claim_mint_weighted(authority, &h.node, fence, changed, &weights)
            .await,
        Err(RewardClaimMintError::ConflictingCause)
    ));
    assert_eq!(counts(h).await?, committed);
    let invalid = sqlx::query("INSERT INTO game_item_fluid_reward_mints(item_instance_id,transaction_id,root_transaction_id,fluid_subtype) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($2,'hex')::uuid,16)")
        .bind(candidate.item_instance_id().as_slice()).bind(candidate.transaction_id().as_slice()).execute(&h.pool).await;
    assert!(
        matches!(&invalid, Err(sqlx::Error::Database(error)) if error.code().as_deref()==Some("23514")),
        "unrelated root cannot acquire a fluid proof: {invalid:?}"
    );
    assert_eq!(counts(h).await?, committed);
    let mut replay = h
        .root
        .freeze_reward_claim_mint_weighted(authority, &h.node, fence, request.clone(), &weights)
        .await
        .map_err(debug)?;
    assert_eq!(candidate.item_instance_id(), replay.item_instance_id());
    h.root
        .commit_reward_claim_mint_weighted_noticed(
            authority,
            &h.node,
            fence,
            &mut replay,
            None,
            &weights,
        )
        .await
        .map_err(debug)?;
    assert_eq!(counts(h).await?, committed);
    let vessel: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i2901'")
        .bind(candidate.item_instance_id().as_slice()).fetch_one(&h.pool).await?;
    let transaction: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())")
        .fetch_one(&h.pool)
        .await?;
    let vessel: [u8; 16] = vessel.try_into().map_err(|_| "vessel identity")?;
    assert_eq!(
        h.root
            .commit_item_fluid(
                authority,
                &h.node,
                fence,
                crate::durability::item_use::fluid::ItemFluidRequest {
                    command_id: 8001,
                    transaction_id: transaction.try_into().map_err(|_| "fluid transaction")?,
                    change: crate::durability::item_use::fluid::ItemFluidChange {
                        item_instance_id: vessel,
                        definition: facts(2901)?.definition,
                        fluid_before: 16,
                        fluid_after: 0,
                    },
                    other: None,
                }
            )
            .await
            .map_err(debug)?,
        crate::durability::item_use::fluid::ItemFluidOutcome::Written
    );
    let after_drink = counts(h).await?;
    let mut after_use = h
        .root
        .freeze_reward_claim_mint_weighted(authority, &h.node, fence, request, &weights)
        .await
        .map_err(debug)?;
    h.root
        .commit_reward_claim_mint_weighted_noticed(
            authority,
            &h.node,
            fence,
            &mut after_use,
            None,
            &weights,
        )
        .await
        .map_err(debug)?;
    assert_eq!(
        counts(h).await?,
        after_drink,
        "historical MINT replay cannot refill a consumed vessel"
    );
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_fluid_states WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(vessel.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(remaining, 0);
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<serde_json::Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('instances',(SELECT count(*) FROM game_item_instances),'receipts',(SELECT count(*) FROM game_reward_claim_mint_receipts),'reservations',(SELECT count(*) FROM game_reward_claim_mint_reservations),'fluid_proofs',(SELECT count(*) FROM game_item_fluid_reward_mints),'fluids',(SELECT count(*) FROM game_item_fluid_states),'audits',(SELECT count(*) FROM game_item_audit_outbox))")
        .fetch_one(&h.pool).await?)
}
