//! M39a: twelve physical source Tomes, incremental NPC topics and exact hand-ins.
//! Crystal9f5a72c cael.lua:84-133 sha256:4d4d0c4aa36722ebb85925c685b7631471889dd1f51e641711cee51568336dcd.
use super::fixture::{Harness, debug, id};
use super::m39_bone_journey::{carried, grant, stage, use_at};
use super::quest_world_callbacks::wire;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::{
    npc_service::NpcIntentDisposition,
    world_object::UseDisposition,
    world_spatial::{StepDirection, StepDisposition},
};
#[path = "m39_tomes_journey/extra.rs"]
mod extra;
#[path = "m39_tomes_journey/prerequisites.rs"]
mod prerequisites;
#[path = "m39_tomes_journey/world.rs"]
pub(crate) mod world;
const FRONTIER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/";
const COUNTER: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/tomeof_knowledge";

#[test]
fn m39a_tomes_twelve_source_books_and_incremental_handins() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m39-tomes")
}

#[test]
fn m39b_tomes_repeatable_paid_xp_and_no_free_reward() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m39-tomes-extra")
}

#[test]
fn m39_tomes_legacy_claim_blocks_a_second_world_reward() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m39-tomes-legacy-first")
}

#[test]
fn m39_tomes_world_claim_blocks_a_second_legacy_reward() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-m39-tomes-world-first")
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    println!("POSTGRES_PLAYER_JOURNEY m39a-tomes server_version={version}");
    assert_eq!(stage(h, COUNTER).await?, None);
    assert_eq!(carried(h, 10217).await?, 0, "no seeded books");
    prerequisites::install(owner, admitted).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 4, 6, 11, 13, 15, 18]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("Tomes capabilities")?;
    if name.ends_with("-legacy-first") || name.ends_with("-world-first") {
        return once_identity(owner, admitted, h, name.ends_with("-legacy-first")).await;
    }
    let [cael, curos, zalamon] = super::classic_quests::install_above(
        owner,
        admitted,
        ["oteryn:npc.cael", "oteryn:npc.curos", "oteryn:npc.zalamon"],
    )
    .await?;
    for (n, key) in ["frontier-ready", "curos-ready", "zalamon-ready"]
        .into_iter()
        .enumerate()
    {
        prerequisites::apply(owner, admitted, key, 18_000 + u64::try_from(n)?).await?;
    }
    assert_eq!(
        grant(owner, admitted, cael, "l2-cael-tome-1").await?.0,
        NpcIntentDisposition::Rejected,
        "no free counter progress"
    );
    assert_eq!(stage(h, COUNTER).await?, None);
    for (npc, key) in [(curos, "l2-alliance-finish"), (zalamon, "l2-zalamon-tome")] {
        assert_eq!(
            grant(owner, admitted, npc, key).await?.0,
            NpcIntentDisposition::Ok
        );
    }
    assert_eq!(carried(h, 10217).await?, 2);
    let mut books = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .filter(|c| c.key.starts_with("oteryn:quest-callback/l2-tome-"))
        .map(|c| c.selector.position.ok_or("book position"))
        .collect::<Result<Vec<_>, _>>()?;
    books.sort_by_key(|p| (p.floor, p.x, p.y));
    assert_eq!(books.len(), 10);
    for p in books {
        let (outcome, command, payload, result) = use_at(owner, admitted, p).await?;
        assert_eq!(outcome, UseDisposition::Committed);
        assert_eq!(
            wire(owner, admitted, command, 2, payload).await?,
            result,
            "literal chest replay"
        );
        assert_eq!(
            use_at(owner, admitted, p).await?.0,
            UseDisposition::Rejected,
            "a new claim cannot duplicate a source book"
        );
        let command = admitted.continuity.next_command_id;
        let result = wire(
            owner,
            admitted,
            command,
            1,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::South),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result_paced(&result, true)
                .map_err(debug)?,
            StepDisposition::Moved
        );
    }
    assert_eq!(
        carried(h, 10217).await?,
        12,
        "two NPC gifts and ten real chest claims"
    );
    for n in 1..=12 {
        if n == 7 {
            refuse_busy_offer(owner, admitted, cael).await?;
            assert_eq!(carried(h, 10217).await?, 6);
            prerequisites::apply(owner, admitted, "frontier-finished", 18_005).await?;
        }
        let key = format!("l2-cael-tome-{n}");
        let (result, command) = grant(owner, admitted, cael, &key).await?;
        assert_eq!(result, NpcIntentDisposition::Ok, "Tome {n}");
        assert_eq!(stage(h, COUNTER).await?, Some(n));
        assert_eq!(carried(h, 10217).await?, 12 - n);
        let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(
            &oteryn_protocol_oteryn::npc_service::NpcTalkIntent {
                npc_actor: cael,
                text: "yes".into(),
            },
        )
        .map_err(debug)?;
        let replay = wire(owner, admitted, command, 7, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&replay)
                .map_err(debug)?,
            NpcIntentDisposition::Ok
        );
        assert_eq!(stage(h, COUNTER).await?, Some(n));
        assert_eq!(carried(h, 10217).await?, 12 - n);
    }
    for flag in ["snake_head_teleport", "corruption_hole", "zao_palace_doors"] {
        assert_eq!(stage(h, &format!("{FRONTIER}{flag}")).await?, Some(1));
    }
    if name.ends_with("-extra") {
        extra::exercise(owner, admitted, h, cael).await?;
    }
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
struct TomeClaimSnapshot {
    receipts: Vec<(String, String, String, i64, bool)>,
    carried_instances: Vec<(String, String, i64)>,
    progress: Option<i64>,
    progress_receipts: i64,
    atomic_progress: bool,
}

async fn claim_snapshot(
    h: &Harness,
    claim: &str,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
) -> TestResult<TomeClaimSnapshot> {
    let receipts = sqlx::query_as(
        "SELECT r.claim_production_key,r.claim_revision_ref,r.item_instance_id::text,r.quantity, \
         c.claimed_transaction_id=r.transaction_id AND c.next_allowed_at IS NULL \
         AND c.claim_revision_ref=r.claim_revision_ref AND i.minted_transaction_id=r.transaction_id \
         FROM game_reward_claim_mint_receipts r \
         JOIN game_reward_claims c USING(character_id,claim_family,claim_production_key) \
         JOIN game_item_instances i USING(item_instance_id) \
         WHERE r.claim_production_key=$1 ORDER BY r.item_instance_id",
    )
    .bind(claim)
    .fetch_all(&h.pool)
    .await?;
    let carried_instances = sqlx::query_as(
        "SELECT i.item_instance_id::text,i.definition_production_key,i.quantity \
         FROM game_reward_claim_mint_receipts r JOIN game_item_instances i USING(item_instance_id) \
         JOIN game_item_container_entries e USING(item_instance_id) \
         WHERE r.claim_production_key=$1 AND i.lifecycle=1 ORDER BY i.item_instance_id",
    )
    .bind(claim)
    .fetch_all(&h.pool)
    .await?;
    let progress = stage(
        h,
        &callback.storage.first().ok_or("source book track")?.track,
    )
    .await?;
    let (progress_receipts, atomic_progress): (i64, bool) = sqlx::query_as(
        "SELECT count(*),coalesce(bool_and(EXISTS(SELECT 1 FROM game_reward_claim_mint_receipts r \
         WHERE r.character_id=q.character_id AND r.claim_production_key=$2 \
         AND r.game_session_id=q.cause_id AND r.command_id=q.cause_ordinal \
         AND r.created_xact_id=q.created_xact_id)),false) \
         FROM game_character_quest_receipts q WHERE q.transition_key=$1",
    )
    .bind(&callback.key)
    .bind(claim)
    .fetch_one(&h.pool)
    .await?;
    Ok(TomeClaimSnapshot {
        receipts,
        carried_instances,
        progress,
        progress_receipts,
        atomic_progress,
    })
}

/// Exercise the retained pre-M39 owner with a real bound source and current fences.
/// Separate command ids model the historical owner without changing wire continuity.
async fn legacy_use(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    position: Position,
    command_id: u64,
) -> TestResult<UseDisposition> {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_object::WorldObjectTarget;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("legacy source action clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "legacy source fence")?
        .ok_or("legacy source fence")?;
    let map = owner.bundle_map().ok_or("source map")?;
    let key = oteryn_world_bundle::bundle::placement_key(position.floor, position.x, position.y, 1)
        .ok_or("source placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let result = owner
        .use_bundle_chest(
            admitted.runtime_actor.ok_or("source actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence.item_fence()),
            },
            map,
            &target,
        )
        .await
        .ok_or("retained source claim did not bind")?;
    Ok(result.disposition)
}

async fn once_identity(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    legacy_first: bool,
) -> TestResult {
    use oteryn_gameplay_quests::quest::world_callbacks::{legacy_world_claim, load};
    let mut books = load()?
        .into_iter()
        .filter(|callback| callback.key.starts_with("oteryn:quest-callback/l2-tome-"))
        .map(|callback| {
            callback
                .selector
                .position
                .map(|position| (position, callback))
                .ok_or("book position")
        })
        .collect::<Result<Vec<_>, _>>()?;
    books.sort_by_key(|(position, _)| (position.floor, position.x, position.y));
    assert_eq!(books.len(), 10);
    let bound = crate::content::world_reward_claims::bind_embedded_reward_claims(
        owner
            .bundle_map()
            .ok_or("source map")?
            .overlay
            .base()
            .as_ref(),
        &|_| true,
    )?;
    let mut paired = 0;
    for (index, (position, callback)) in books.iter().enumerate() {
        let proof = legacy_world_claim(callback)?;
        if let Some(proof) = proof {
            paired += 1;
            let claim = bound
                .served
                .iter()
                .find(|claim| claim.key.as_str() == proof.key)
                .ok_or("legacy source claim with its real UID")?;
            assert_eq!(claim.revision.as_str(), proof.revision);
            assert_eq!(
                proof.authority_sha256,
                claim
                    .authority_digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
                "Content binder authority must agree"
            );
            assert!(
                claim
                    .placements
                    .iter()
                    .any(|placement| placement.cell.x == position.x
                        && placement.cell.y == position.y
                        && placement.cell.floor == position.floor)
            );
        } else {
            assert_eq!(callback.key, "oteryn:quest-callback/l2-tome-container");
        }
        let claim_key = proof.map_or(callback.key.as_str(), |proof| proof.key.as_str());
        let before = claim_snapshot(h, claim_key, callback).await?;
        assert!(before.receipts.is_empty() && before.carried_instances.is_empty());
        assert_eq!(before.progress, None);
        assert_eq!(before.progress_receipts, 0);
        assert!(!before.atomic_progress);
        let legacy_command = 90_000 + u64::try_from(index)?;
        let claimed_by_legacy = legacy_first && proof.is_some();
        if claimed_by_legacy {
            assert_eq!(
                legacy_use(owner, admitted, *position, legacy_command).await?,
                UseDisposition::Committed
            );
        } else {
            let (result, command, payload, bytes) = use_at(owner, admitted, *position).await?;
            assert_eq!(result, UseDisposition::Committed);
            assert_eq!(
                wire(owner, admitted, command, 2, payload).await?,
                bytes,
                "World claim literal replay"
            );
        }
        let granted = claim_snapshot(h, claim_key, callback).await?;
        assert_eq!(granted.receipts.len(), 1, "one exact once claim/MINT");
        let receipt = granted.receipts.first().ok_or("source MINT receipt")?;
        assert_eq!(receipt.0, claim_key);
        if let Some(proof) = proof {
            assert_eq!(receipt.1, proof.revision);
        }
        assert_eq!(
            (receipt.3, receipt.4),
            (1, true),
            "same immutable claim/item transaction"
        );
        assert_eq!(
            granted.carried_instances,
            [(receipt.2.clone(), "oteryn:item.tibia.i10217".into(), 1)]
        );
        assert_eq!(granted.progress, (!claimed_by_legacy).then_some(1));
        assert_eq!(granted.progress_receipts, i64::from(!claimed_by_legacy));
        assert_eq!(
            granted.atomic_progress, !claimed_by_legacy,
            "World progress and MINT share the physical transaction"
        );
        if claimed_by_legacy {
            assert_eq!(
                legacy_use(owner, admitted, *position, legacy_command).await?,
                UseDisposition::Committed,
                "historical claim replays its original MINT"
            );
            assert_eq!(claim_snapshot(h, claim_key, callback).await?, granted);
        } else if proof.is_some() {
            for _ in 0..2 {
                assert_eq!(
                    legacy_use(owner, admitted, *position, legacy_command).await?,
                    UseDisposition::NothingToUse,
                    "the old owner cannot issue a second source book"
                );
                assert_eq!(claim_snapshot(h, claim_key, callback).await?, granted);
            }
        }
        let (result, command, payload, bytes) = use_at(owner, admitted, *position).await?;
        assert_eq!(
            result,
            UseDisposition::Rejected,
            "the World owner cannot duplicate an existing source claim"
        );
        assert_eq!(
            wire(owner, admitted, command, 2, payload).await?,
            bytes,
            "refused source USE replays literally"
        );
        assert_eq!(
            claim_snapshot(h, claim_key, callback).await?,
            granted,
            "no second item or progress write"
        );
        assert_eq!(carried(h, 10217).await?, i64::try_from(index)? + 1);
        let command = admitted.continuity.next_command_id;
        let result = wire(
            owner,
            admitted,
            command,
            1,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::South),
        )
        .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_spatial::decode_step_result_paced(&result, true)
                .map_err(debug)?,
            StepDisposition::Moved
        );
    }
    assert_eq!(paired, 9);
    assert_eq!(carried(h, 10217).await?, 10, "one book per physical source");
    Ok(())
}

async fn refuse_busy_offer(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with("/l2-cael-tome-7"))
        .ok_or("seventh Tome")?;
    // A native disconnect ends NPC focus. Keep greeting and refusal together,
    // exactly as the successful hand-in conversations above do.
    let (result, _, _) = super::qengine_rewards::wire_conversation(
        owner,
        admitted,
        npc,
        &callback,
        false,
        vec!["hi", "tome"],
    )
    .await?;
    assert_eq!(result, NpcIntentDisposition::Rejected);
    Ok(())
}
