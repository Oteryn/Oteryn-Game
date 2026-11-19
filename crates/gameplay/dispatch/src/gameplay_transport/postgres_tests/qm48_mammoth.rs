//! Crystal 201189f8 dankwart.lua:60-66, fluids.lua:1-2,120-122 and action_horn.lua:34-46.
//! Buying and drinking Dankwart's mead must enable the native mammoth callback.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, barbarian_mead,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde_json::json;

const AT: Position = Position {
    x: 32154,
    y: 31028,
    floor: -5,
};
const MISSION: &str = "oteryn:quest-progress/quest/u8_0/barbarian_test/mission03";
pub(super) const NAME: &str = "qm48/mammoth";
const KEY: &str = "oteryn:quest-callback/test/mammoth-live-drunk";
const MUG: &str = "oteryn:item.tibia.i2880";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/barbarian_test/action_horn.lua:34-46";

#[test]
fn qm48_mammoth_live_drunk_refusal_expiry_cleanse_commit_replay() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qm48-mammoth", || {
        super::quest_world_callbacks::scenario(NAME, AT, false)
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qm48_mammoth duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    native_scene_with_appearances(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z05-x125-y121.b3"
            )),
            region: (5, 125, 121),
            bounds: (32152, 31026, 32156, 31030),
            start: (32154, 31027),
            expected: vec![ExpectedTile {
                position: (32154, 31028),
                ids: vec![6592, 7176],
                counted_entry: None,
            }],
        }],
        &[],
        &[7177],
    )
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(json!({
        "key": KEY,
        "quest": barbarian_mead::QUEST,
        "trigger": "use",
        "requires_world_admission": true,
        "selector": {"native": {"position": AT, "ordinal": 1, "appearance": 7176}},
        "reversible": false,
        "passages": [],
        "storage": [
            {"track": barbarian_mead::TRACKS[0], "minimum": 6, "maximum": 6, "set": 7},
            {"track": MISSION, "minimum": 1, "maximum": 1, "set": 2}
        ],
        "effects": {"requires_live_drunk": true, "transform": 7177, "reset_after_ms": 5000},
        "offerings": [],
        "return_on_failure": false,
        "sources": [SOURCE]
    }))?)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_gameplay_domain::domain::condition::{ConditionType, ConditionValues, StatusKind};
    use oteryn_gameplay_foundation::foundation::condition::{
        ApplicationFacts, ConditionDefinition, ConditionSourceKind,
    };
    use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY qm48_mammoth version={version}");
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1).ok_or("mammoth key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: 0,
    })
    .map_err(debug)?;
    let apply = |now, duration_ms| -> TestResult<_> {
        Ok((
            ConditionDefinition::new(
                "qm48.mead.drunk",
                1,
                ConditionValues::TimedStatus {
                    kind: StatusKind::Drunk,
                    duration_ms,
                },
            )
            .ok_or("definition")?,
            now,
        ))
    };
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(3))
        .ok_or("clock")?;
    for command in [1, 2, 3, 5] {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("world action clock")?;
        if command == 5 {
            drink_mead(owner, admitted, h).await?;
        } else if command > 1 {
            let now = owner.owner_now().get();
            let (definition, at) = apply(
                if command == 2 {
                    now.saturating_sub(2_000_000)
                } else {
                    now
                },
                if command == 2 { 1_000 } else { 10_000 },
            )?;
            let root = GameplayDecisionRoot::from_bytes([7; 32]);
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let conditions = &mut states
                .get_mut(&runtime, actor, session)
                .ok_or("state")?
                .conditions;
            conditions
                .apply(
                    &definition,
                    Some("mead".into()),
                    ConditionSourceKind::SelfUse,
                    &[],
                    &ApplicationFacts {
                        now: at,
                        base_speed: 220,
                        mana_shield_capacity: 0,
                        target_reentry_protected: false,
                        source_reentry_protected: false,
                        target_is_player: true,
                        decision_root: &root,
                        occurrence: DecisionOccurrenceId::from_bytes([u8::try_from(command)?; 16]),
                    },
                )
                .map_err(debug)?;
            if command == 3 {
                conditions.remove_type(ConditionType::Status(StatusKind::Drunk));
            }
        }
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone())
                .await?;
        let result =
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?;
        assert_eq!(
            result,
            if command == 5 {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            },
            "command {command}"
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(KEY)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(receipts, i64::from(command == 5));
    }
    let rows: Vec<(String, i64)> = sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key IN ($2,$3) ORDER BY track_key")
        .bind(super::fixture::id(41).as_slice()).bind(MISSION).bind(barbarian_mead::TRACKS[0]).fetch_all(&h.pool).await?;
    assert_eq!(
        rows,
        vec![(MISSION.into(), 2), (barbarian_mead::TRACKS[0].into(), 7)]
    );
    let runtime = owner.runtime.lock().await;
    owner
        .states
        .spell_states
        .lock()
        .await
        .get_mut(&runtime, actor, session)
        .ok_or("state")?
        .conditions
        .cleanse_negative();
    drop(runtime);
    let replay = super::quest_world_callbacks::wire(owner, admitted, 5, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed
    );
    let map = owner.bundle_map().ok_or("map")?;
    assert_eq!(map.facts.object_revision(key), 1);
    assert_eq!(map.facts.expire_movement_items(owner.owner_now().get()), 0);
    assert_eq!(
        map.facts
            .expire_movement_items(owner.owner_now().get() + 5_000_000),
        1
    );
    Ok(())
}

async fn buy_mead(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use super::classic_quests;
    use crate::content::ProjectV2ServiceOfferDirection;
    use crate::gameplay_transport::{
        connection::{FreshAdmissionAuthority, UseCommand},
        npc_trade_command::NpcTradeStep,
    };
    use oteryn_gameplay_quests::quest::world_callbacks::Offering;
    use oteryn_protocol_oteryn::npc_service::{
        NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, NpcTradeSide,
    };
    const DANKWART: &str = "oteryn:npc.dankwart";

    let fence = admitted.item_fence.ok_or("item fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence,
        &Offering {
            position: AT,
            item: "oteryn:item.tibia.i3031".into(),
            count: 5,
        },
        951,
        true,
    )
    .await?;
    let [npc] = classic_quests::install_above(owner, admitted, [DANKWART]).await?;
    let book = owner.states.chat.npc_talk().ok_or("NPC catalogue")?;
    let entry = book.get(DANKWART).ok_or("Dankwart catalogue entry")?;
    let offer = entry
        .offers()
        .iter()
        .find(|offer| {
            offer.item.key == MUG
                && offer.sub_type == Some(16)
                && offer.direction == ProjectV2ServiceOfferDirection::SellToPlayer
        })
        .ok_or("Dankwart's mug of mead")?;
    assert_eq!((offer.count, offer.unit_price), (Some(1), 5));
    let mug_ref = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", MUG, &offer.item.revision))
        .ok_or("mug definition")?;
    classic_quests::talk(owner, admitted, npc, 20501, "hi").await?;
    let actor = admitted.runtime_actor.ok_or("buyer")?;
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let open = owner
        .npc_talk(
            actor,
            command(20502),
            NpcTalkIntent { npc_actor: npc, text: "trade".into() },
        )
        .await;
    assert_eq!(open.disposition, NpcIntentDisposition::Ok);
    let window = open.window.ok_or("Dankwart's shop window")?;
    let (shop_npc, catalogue_revision) = window.open.ok_or("open shop")?;
    assert_eq!(shop_npc, npc);
    let offer_index = window
        .offers
        .iter()
        .position(|shown| {
            shown.item_definition_ref == mug_ref.get()
                && shown.side == NpcTradeSide::Buy
                && shown.count == 1
                && shown.unit_price == 5
        })
        .ok_or("mead in the trade window")?;
    assert!(window.offers.iter().any(|shown| {
        shown.item_definition_ref == mug_ref.get()
            && shown.side == NpcTradeSide::Buy
            && shown.count == 1
            && shown.unit_price == 3
    }));
    let purchase = owner
        .npc_trade(
            actor,
            command(20503),
            NpcTradeIntent {
                npc_actor: npc,
                catalogue_revision,
                offer_index: u32::try_from(offer_index)?,
                side: NpcTradeSide::Buy,
                quantity: 1,
                expected_unit_price: 5,
            },
        )
        .await;
    assert!(matches!(purchase, NpcTradeStep::Traded));
    let fluids: Vec<(i64, i16)> = sqlx::query_as(
        "SELECT i.quantity, s.fluid_subtype FROM game_item_instances i
         JOIN game_item_fluid_states s USING(item_instance_id)
         WHERE i.definition_production_key=$1 AND i.lifecycle=1",
    )
    .bind(MUG)
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(fluids, [(1, 16)]);
    Ok(())
}

async fn drink_mead(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::FreshAdmissionAuthority;
    use oteryn_gameplay_domain::domain::condition::{ConditionType, StatusKind};
    use oteryn_protocol_oteryn::world_object::encode_use_item_intent;
    buy_mead(owner, admitted, h).await?;
    let reference = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", MUG, "definition-r1"))
        .ok_or("mug definition")?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    let payload = encode_use_item_intent(std::num::NonZeroU64::MIN);
    let (next, disposition, _) =
        super::item_craft::exchange(owner, *admitted, 4, &payload, Some(reference)).await?;
    *admitted = next;
    assert_eq!(disposition, UseDisposition::Committed);
    assert_eq!(h.count("game_item_fluid_states").await?, 0);
    let actor = admitted.runtime_actor.ok_or("actor after drinking")?;
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let conditions = states
        .get(&runtime, actor, admitted.game_session_id)
        .ok_or("spell state after drinking")?
        .owned_conditions();
    let drunk = conditions
        .get(ConditionType::Status(StatusKind::Drunk).conflict_key())
        .ok_or("mead must apply Drunk")?;
    // Crystal fluids.lua:1-2 fixes the duration at 60 seconds. Use the
    // committed start time so dispatch and database latency cannot affect it.
    assert_eq!(drunk.remaining_duration_ms(drunk.started_at()), Some(60_000));
    Ok(())
}

pub(super) fn catalogue(callback: &Callback) -> TestResult<QuestStateCatalogue> {
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({
        "schema": 1,
        "quests": [{
            "quest": barbarian_mead::QUEST,
            "recipe_sha256": "ee5a983a8eec21cb3332f908b8baf0e66311797615eae25e103518e1583a197f",
            "tracks": [
                {"key": barbarian_mead::TRACKS[0], "initial": 6, "minimum": -1, "maximum": 8},
                {"key": MISSION, "initial": 1, "minimum": -1, "maximum": 2},
                {"key": barbarian_mead::TRACKS[2], "initial": 9, "minimum": 0, "maximum": 9}
            ],
            "stages": [{
                "key": "test-mammoth",
                "completes": false,
                "trigger": {"kind": "map_use", "callback": callback}
            }],
            "sources": [SOURCE]
        }]
    }))?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("quest catalogue")?)?)
}
