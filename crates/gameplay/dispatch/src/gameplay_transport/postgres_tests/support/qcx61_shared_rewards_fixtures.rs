use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::{
    arena::{ArenaAction, ArenaDisposition},
    world_object::{UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent},
};

pub(super) async fn greenhorn_door(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    actor: ExactActorRef,
) -> TestResult<UseDisposition> {
    super::qcx61_halvar_fixtures::use_sealed_door(owner, admitted, actor, 32227, 31066).await
}

pub(super) async fn after_first_rank(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    actor: ExactActorRef,
    leave_before_use: bool,
) -> TestResult {
    if leave_before_use {
        // Crystal movements_arena_pit.lua:26-46 leaves the rank's door unlocked.
        let left =
            super::arena_match_fixtures::command(owner, admitted, ArenaAction::Leave).await?;
        assert_eq!(left.disposition, ArenaDisposition::Applied);
        assert_eq!(left.state.phase, 0);
        assert_eq!(left.state.completed_rank, 1);
        assert_eq!(
            greenhorn_door(owner, admitted, actor).await?,
            UseDisposition::Committed
        );
    }
    let session = admitted.game_session_id;
    let reward_room = {
        let mut runtime = owner.runtime.lock().await;
        let previous = runtime.read_actor_position(actor).map_err(debug)?;
        runtime
            .relocate_arena_player(
                actor,
                session,
                previous,
                MovementLocalPosition {
                    x: 32235,
                    y: 31067,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        previous.position()
    };
    let map = owner.bundle_map().ok_or("arena map")?;
    let placement_key = oteryn_world_bundle::bundle::placement_key(-7, 32235, 31068, 1)
        .ok_or("source chest placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&placement_key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(placement_key),
    })
    .map_err(debug)?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let id = admitted.continuity.next_command_id;
    let result =
        super::quest_world_callbacks_fixtures::wire(owner, admitted, id, 2, payload.clone())
            .await?;
    assert_eq!(
        decode_use_result(&result).map_err(debug)?,
        UseDisposition::Committed
    );
    if leave_before_use {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let id = admitted.continuity.next_command_id;
        let repeat =
            super::quest_world_callbacks_fixtures::wire(owner, admitted, id, 2, payload).await?;
        assert_eq!(
            decode_use_result(&repeat).map_err(debug)?,
            UseDisposition::NothingToUse
        );
    } else {
        {
            let mut runtime = owner.runtime.lock().await;
            let previous = runtime.read_actor_position(actor).map_err(debug)?;
            runtime
                .relocate_arena_player(actor, session, previous, reward_room)
                .map_err(debug)?;
        }
        // Choosing a different weapon through the wire command still sees the chest receipt.
        let result = super::arena_match_fixtures::command(
            owner,
            admitted,
            ArenaAction::ClaimReward {
                rank: 1,
                category: 3,
                choice: 2,
            },
        )
        .await?;
        assert_eq!(result.disposition, ArenaDisposition::AlreadyClaimed);
        assert_eq!(result.state.claimed_masks[0] & 4, 4);
    }
    let claims: Vec<String> = sqlx::query_scalar(
        "SELECT claim_production_key FROM game_reward_claims ORDER BY claim_production_key",
    )
    .fetch_all(&harness.pool)
    .await?;
    assert_eq!(claims, ["oteryn:reward.svargrond-arena.1.3"]);
    Ok(())
}
