//! Real world USE joins an existing source wardrobe to the fenced cosmetic owner.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};

#[test]
fn questpc7_world_wardrobe_grant_is_atomic_and_replay_safe() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let start = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc7-world-outfit", || {
        super::quest_world_callbacks::scenario(
            "l1-warmaster-wardrobe",
            Position {
                x: 33053,
                y: 31020,
                floor: -7,
            },
            false,
        )
    });
    eprintln!(
        "QUESTPC7_WORLD_OUTFIT POSTGRES_CASE_COMPLETED duration={:?} success={}",
        start.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("QUESTPC7_WORLD_OUTFIT POSTGRES_STARTED version={version}");
    // The other lane's completed mission is a fenced fixture precondition, not a
    // qualification of that quest. No subject progress or grants are inserted by SQL.
    let seed = "oteryn:quest-transition/fixture/prior-wardrobe-mission";
    let mut transitions = vec![
        owner
            .quest_catalogue
            .as_ref()
            .and_then(|c| c.transition(&callback.key))
            .ok_or("wardrobe transition")?
            .clone(),
    ];
    transitions.push(QuestTransition {
        key: seed.into(),
        quest: callback.quest.clone(),
        effects: callback
            .storage
            .iter()
            .map(|s| QuestEffect {
                track: s.track.clone(),
                from: QuestComparison::Between(-1, s.maximum),
                effect: QuestEffectKind::Set(s.minimum),
            })
            .collect(),
        completes: false,
        experience: None,
        world_change: None,
    });
    let catalogue = std::sync::Arc::new(
        QuestStateCatalogue::new(
            "content-1",
            callback
                .storage
                .iter()
                .map(|s| QuestTrack {
                    key: s.track.clone(),
                    quest: callback.quest.clone(),
                    initial: -1,
                    min: -1,
                    max: s.set.max(s.maximum),
                })
                .collect(),
            transitions,
        )
        .map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: seed.into(),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(900).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    drop(slot);
    // Legacy callers cannot omit the admitted generation to earn a fresh outfit.
    let context = {
        let runtime = owner.runtime.lock().await;
        let expected = runtime.read_actor_position(actor).map_err(debug)?;
        let map = owner.bundle_map().ok_or("map")?;
        crate::durability::quest_world_callback::QuestWorldContext {
            party: Vec::new(),
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_revision: format!(
                "sha256:{}",
                runtime
                    .content_pin()
                    .server_artifact_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
            reset_epoch: owner.reset_epoch,
            actor_placement: actor.placement_identity(),
            movement_revision: expected.revision(),
        }
    };
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let request = QuestTransitionRequest {
        transition_key: callback.key.clone(),
        cause: QuestCause::Use(CommandRef::new(
            admitted.game_session_id,
            CommandId::new(901).map_err(debug)?,
        )),
    };
    assert!(
        slot.commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request.clone(),
            owner.quest_catalogue.clone().ok_or("catalogue")?
        )
        .await
        .is_err(),
        "generic writer cannot bypass the world participant"
    );
    let denied = slot
        .commit_world_callback(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            owner.quest_catalogue.clone().ok_or("catalogue")?,
            &permit,
            context,
            None,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(denied, QuestTransitionOutcome::Refused(_)),
        "missing admitted generation must refuse: {denied:?}"
    );
    drop(slot);
    let before: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_account_appearance_unlock_receipts),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(before, (0, 1), "refusal cannot grant or advance the quest");
    drop(permit);
    let p = callback.selector.position.ok_or("position")?;
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1).ok_or("key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await?;
    assert_eq!(
        decode_use_result(&reply).map_err(debug)?,
        UseDisposition::Committed
    );
    let count: i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key='oteryn:outfit.warmaster/quest-grant/0'").fetch_one(&h.pool).await?;
    assert_eq!(
        count, 1,
        "a committed world wardrobe must grant its compatible base in the same transaction"
    );
    let replay = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
    assert_eq!(replay, reply);
    let count: i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts WHERE unlock_key='oteryn:outfit.warmaster/quest-grant/0'").fetch_one(&h.pool).await?;
    assert_eq!(count, 1);
    Ok(())
}
