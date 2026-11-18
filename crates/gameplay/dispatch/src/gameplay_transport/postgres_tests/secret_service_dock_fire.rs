//! Crystal 201189f8, data-global/scripts/quests/others/actions_fire_bug.lua:144-155.
//! The Fandom archive Secret Service Quest/Spoiler confirms the shared lockout.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug, id},
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::starter_grant::{StarterGrantOutcome, StarterTemplate};
use crate::foundation::GameSessionId;
use crate::gameplay_transport::connection::{
    ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, SessionContinuity,
};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent, encode_use_intent,
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
const FIRE_BUG: &str = "oteryn:item.tibia.i5467";
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
    callback["item_on_target"] = json!({
        "item":FIRE_BUG,"count":1,"reusable":true,"selected_instance":true,
        "replacement":null,"target_replacement":null
    });
    callback["effects"] =
        json!({"item_presence":{"position":AT,"appearance":2118,"lifetime_ms":LIFETIME_MS}});
    let document = json!({"schema":1,"quests":[{"quest":QUEST,
        "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
        "sources":["synthetic dock-fire engine fixture"],
        "tracks":[{"key":TRACK,"initial":1,"minimum":1,"maximum":2}],
        "stages":[{"key":"ignite","completes":false,"trigger":{"kind":"map_use","callback":callback}}]}]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
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
            item: FIRE_BUG.into(),
            count: 1,
        },
        4600,
        true,
    )
    .await?;
    let mut peer = admit_peer(owner, admitted, h).await?;
    assert_bare_use_refused(owner, admitted, AT).await?;
    let ignition = owner.owner_now().get();
    assert_eq!(
        use_selected_tool(owner, admitted, AT, FIRE_BUG).await?,
        UseDisposition::Committed
    );
    assert_eq!(super::qengine_voucher_stories::quantity(h, 5467).await?, 2);
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
    assert_eq!(mission_stage(h, &id(41)).await?, 2);
    let peer_character = *peer.item_fence.ok_or("peer item fence")?.character_id.as_bytes();
    assert_eq!(mission_stage(h, &peer_character).await?, 1);
    assert_eq!(
        use_selected_tool(owner, &mut peer, AT, FIRE_BUG).await?,
        UseDisposition::Occupied
    );
    assert_eq!(mission_stage(h, &peer_character).await?, 1);
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(map.facts.next_movement_decay_time(), Some(due));
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_millis(u64::from(LIFETIME_MS)))
        .ok_or("clock")?;
    let map = owner.bundle_map().ok_or("map")?;
    let stop = oteryn_foundation::CancellationToken::new();
    let observe = async {
        while !map.facts.quest_callback_presence_available(pos) {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        stop.cancel();
    };
    tokio::time::timeout(
        Duration::from_secs(10),
        super::fixture::join_two(owner.run_source_owner_cycles(&stop), observe),
    )
    .await?;
    assert!(map.facts.movement_items(pos).is_empty());
    assert_eq!(map.facts.next_movement_decay_time(), None);
    assert_eq!(
        use_selected_tool(owner, &mut peer, AT, FIRE_BUG).await?,
        UseDisposition::Committed
    );
    assert_eq!(mission_stage(h, &id(41)).await?, 2);
    assert_eq!(mission_stage(h, &peer_character).await?, 2);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 5467).await?, 2);
    Ok(())
}

async fn mission_stage(h: &Harness, character: &[u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT COALESCE((SELECT value FROM game_character_quest_tracks
          WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),1)",
    )
    .bind(character.as_slice())
    .bind(TRACK)
    .fetch_one(&h.pool)
    .await?)
}

async fn admit_peer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    first: &AdmittedSession,
    h: &Harness,
) -> TestResult<AdmittedSession> {
    h.seed_pvp_target().await?;
    let operation =
        super::appearance_change_postgres::observer_visibility_tests::admit_with_operation(h)
            .await?;
    let session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session(session)
        .await
        .map_err(debug)?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("peer fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, super::quest_world_callbacks::local(AT))
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize(
                &runtime,
                actor,
                session,
                crate::gameplay_transport::actor_spell::tests::FACTS,
                (3, 7_000_000),
                owner.owner_now(),
            )
            .ok_or("peer spell state")?;
        actor
    };
    let mut peer = AdmittedSession {
        game_session_id: session,
        world_id: first.world_id,
        channel_id: first.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("peer transport")?,
            account_id: id(100),
        }),
        continuity: SessionContinuity::FRESH,
        item_fence: Some(fence),
    };
    let generation = owner.active_generation.ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native")?;
    h.root
        .initialize_admission_equipment(
            owner.character,
            owner.holder,
            &fence,
            generation.identity().server_artifact_digest(),
            &operation,
        )
        .await
        .map_err(debug)?;
    let grant = h
        .root
        .grant_starter_items(
            owner.character,
            owner.holder,
            &fence,
            &NativeItemWeights(native),
            &StarterTemplate {
                definition: crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..StarterTemplate::main()
            },
        )
        .await
        .map_err(debug)?;
    assert!(!matches!(grant, StarterGrantOutcome::NotAdmitted));
    owner.admit_progression_session(&mut peer).await;
    owner.admit_quest_session(&peer).await;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "peer quest fence")?
        .ok_or("peer quest fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: AT,
            item: FIRE_BUG.into(),
            count: 1,
        },
        4601,
        true,
    )
    .await?;
    Ok(peer)
}

pub(super) async fn assert_bare_use_refused(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
) -> TestResult {
    let target = world_target(owner, at)?;
    let use_intent = encode_use_intent(&target).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let result =
        super::quest_world_callbacks::wire(owner, admitted, command, 2, use_intent).await?;
    assert_ne!(decode_use_result(&result).map_err(debug)?, UseDisposition::Committed);
    Ok(())
}

pub(super) async fn use_selected_tool(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
    tool: &str,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("clock")?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(), &[4, 6, 11, 15, 18],
        )
        .ok_or("selected tool capabilities")?;
    let reference = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", tool, "definition-r1"))
        .ok_or("tool definition")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(world_target(owner, at)?)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (next, result, _) =
        super::item_craft::exchange(owner, *admitted, command, &payload, Some(reference)).await?;
    *admitted = next;
    Ok(result)
}

fn world_target(owner: &ComposedFreshAdmission<'_, '_, '_>, at: Position) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, 1)
        .ok_or("source position")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}
