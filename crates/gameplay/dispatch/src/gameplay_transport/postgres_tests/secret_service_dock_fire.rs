//! Crystal 201189f8, data-global/scripts/quests/others/actions_fire_bug.lua:144-155.
//! The Fandom archive Secret Service Quest/Spoiler confirms the shared lockout.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
    authored_bindings::Bindings,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

pub(super) const NAME: &str = "qengine-rewards-secret-service-dock-fire";
const AT: Position = Position {
    x: 33294,
    y: 31677,
    floor: -15,
};
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const TRACK: &str = "oteryn:quest-progress/dock-fire-fixture/mission";
const REARM: &str = "oteryn:quest-transition/dock-fire-fixture/rearm";
const LIFETIME_MS: u32 = 420_000;

#[test]
fn dock_fire_blocks_eligible_use_until_its_exact_item_retires() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::qengine_rewards::scenario(NAME)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let mut callback = super::authored_world_rewards::placeholder(AT)?;
    callback.key = "oteryn:quest-callback/dock-fire-fixture/ignite".into();
    callback.requires_world_admission = true;
    callback.storage[0].track = TRACK.into();
    callback.storage[0].minimum = 1;
    callback.storage[0].maximum = 1;
    callback.storage[0].set = 2;
    callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_fire_bug.lua:144-155".into()];
    let mut callback = serde_json::to_value(callback)?;
    callback["item_on_target"] = json!({"item":"oteryn:item.tibia.i5467","count":1,"reusable":true,"replacement":null,"target_replacement":null});
    callback["effects"] =
        json!({"item_presence":{"position":AT,"appearance":2118,"lifetime_ms":LIFETIME_MS}});
    let document = json!({"schema":1,"quests":[{"quest":QUEST,
        "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
        "sources":["synthetic dock-fire engine fixture"],
        "tracks":[{"key":TRACK,"initial":1,"minimum":1,"maximum":2}],
        "stages":[{"key":"ignite","completes":false,"trigger":{"kind":"map_use","callback":callback}}]}]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("revision")?;
    let catalogue = Arc::new(
        bindings
            .attach(&base)?
            .with_fixture_transition(QuestTransition {
                key: REARM.into(),
                quest: QUEST.into(),
                effects: vec![QuestEffect {
                    track: TRACK.into(),
                    from: QuestComparison::Eq(2),
                    effect: QuestEffectKind::Set(1),
                }],
                completes: false,
                experience: None,
                world_change: None,
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: AT,
            item: "oteryn:item.tibia.i5467".into(),
            count: 1,
        },
        4600,
        true,
    )
    .await?;
    let ignition = owner.owner_now().get();
    assert_eq!(
        use_target(owner, admitted, AT).await?,
        UseDisposition::Committed
    );
    let pos = crate::map::overlay::TilePos {
        x: AT.x,
        y: AT.y,
        floor: AT.floor,
    };
    let due = {
        let map = owner.bundle_map().ok_or("map")?;
        let due = map
            .facts
            .next_movement_decay_time()
            .ok_or("fire deadline")?;
        let duration = u64::from(LIFETIME_MS) * 1_000;
        assert!(due >= ignition + duration && due <= owner.owner_now().get() + duration);
        assert_eq!(
            map.facts
                .movement_items(pos)
                .iter()
                .filter(|e| e.facts.appearance_id == 2118)
                .count(),
            1
        );
        assert_eq!(map.facts.expire_movement_items(due - 1), 0);
        due
    };
    // Restore eligibility through the ordinary fenced writer while the same shared fire burns.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    {
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: REARM.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(4690).map_err(debug)?,
                    )),
                },
                catalogue,
            )
            .await
            .map_err(debug)?;
        assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    }
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        use_target(owner, admitted, AT).await?,
        UseDisposition::Occupied
    );
    let stored: i64 =
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(TRACK)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(stored, 1);
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert!(map.facts.movement_items(pos).is_empty());
    assert!(map.facts.quest_callback_presence_available(pos));
    assert_eq!(map.facts.expire_movement_items(due), 0);
    assert_eq!(
        use_target(owner, admitted, AT).await?,
        UseDisposition::Committed
    );
    Ok(())
}

pub(super) async fn use_target(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, 1)
        .ok_or("source position")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let use_intent = encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let result =
        super::quest_world_callbacks::wire(owner, admitted, command, 2, use_intent).await?;
    decode_use_result(&result).map_err(|error| format!("{error:?}").into())
}
