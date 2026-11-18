//! Fenced PostgreSQL USE and STEP through the world callback dispatcher.
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8, data-global/scripts/quests:
//! the_pits_of_inferno_quest/actions_fountain.lua:3-15;
//! lions_rock/actions_lions_rock.lua:115-139;
//! the_first_dragon/actions_rewards.lua:43-72;
//! the_great_dragon_hunt_quest/actions_treasure.lua:19-40 and
//! the_great_dragon_hunt_quest/creaturescripts_the_great_dragon_hunt.lua:12-25.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::UseCommand;
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(super) const PREFIX: &str = "qengine-rewards-qonce-";
const WELL: &str = "qengine-rewards-qonce-well";
const WELL_LOST_ACK: &str = "qengine-rewards-qonce-well-lost-ack";
const DAILY: &str = "qengine-rewards-qonce-daily";
const FOUNTAIN_TRANSFORM: &str = "qengine-rewards-qonce-fountain-transform";
const ANNUAL: &str = "qengine-rewards-qonce-annual";
const FULL_BACKPACK: &str = "qengine-rewards-qonce-full-backpack";
const HOARD: &str = "qengine-rewards-qonce-hoard";
const STEP: &str = "qengine-rewards-qonce-step";
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const CLAIM: &str = "oteryn:quest-callback/qonce-dispatch-claim";
const LEDGER: &str = "oteryn:quest-progress/qonce-dispatch/ledger";
const CHOICE: &str = "oteryn:quest-progress/qonce-dispatch/choice";
const HUNT_GATE: &str = "oteryn:quest-progress/qonce-dispatch/hunt-gate";
const HUNT_KILL: &str = "oteryn:quest-transition/native-kills/qonce-hoard-dragon";
const DRAGON: &str = "oteryn:creature.dragon";
const LION: &str = "oteryn:creature.noble_lion";
const LION_MOUNT: &str = "oteryn:content.mount.noble_lion";
const LIONS_HEART: u16 = 21439;
const HOLY_AREA_EFFECT: u16 = 50;
const LIONS_FOUNTAIN: u16 = 6389;
const LIONS_SANCTUARY_ROCK: u16 = 1852;
const DAILY_PERIOD: i64 = 86_401;
const COUNT: &str = "oteryn:quest-progress/qonce-dispatch/count";
const WELL_ACHIEVEMENT: &str = "oteryn:achievement/fountain_of_life";
const HOARD_ACHIEVEMENT: &str = "oteryn:achievement/hoard_of_the_dragon";
const DAILY_REFUSAL: &str = "At the moment there is neither a treasure nor anything else in the fountain. Perhaps you might return later.";
const FOUNTAIN_RED_GEM: &str = "Something sparkles in the fountain's water. You draw out a red gem.";
const ANNUAL_REFUSAL: &str = "The chest is empty.";
const ANNUAL_INELIGIBLE: &str = "You are not eligible to claim these rewards yet.";
const FEATHERS: &str = "oteryn:quest-progress/qonce-dispatch/feathers";
const HOARD_REFUSAL: &str = "You try to pick a treasure, but you hear further dragons approaching. You should kill some more before picking out something.";
const AT: Position = Position { x: 33294, y: 31677, floor: -15 };

#[test]
fn once_only_well_restores_saved_and_live_vitals_and_grants_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(WELL)
}

#[test]
fn daily_claim_serializes_double_use_and_refuses_until_period_expires_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(DAILY)
}

#[test]
fn annual_claim_delivers_backpack_and_refuses_until_period_expires_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(ANNUAL)
}

#[test]
fn fountain_replay_projects_the_rock_and_refuses_it_after_cooldown_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(FOUNTAIN_TRANSFORM)
}

pub(super) fn map_fixture(name: &str, callback: &mut Callback) {
    if name == FOUNTAIN_TRANSFORM {
        callback.selector.appearance = Some(LIONS_FOUNTAIN);
        callback.effects.transform = Some(LIONS_SANCTUARY_ROCK);
    }
}

#[test]
fn full_backpack_rolls_back_bundle_and_cooldown_until_claim_retry_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(FULL_BACKPACK)
}

#[test]
fn hoard_bundle_resets_eligibility_and_awards_tenth_claim_achievement_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(HOARD)
}

#[test]
fn interrupted_well_use_recovers_committed_heal_without_second_credit_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(WELL_LOST_ACK)
}

#[test]
fn step_claim_credits_the_same_achievement_only_once_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(STEP)
}

fn storage(track: &str, minimum: i64, maximum: i64, set: i64) -> Value {
    json!({"track": track, "minimum": minimum, "maximum": maximum, "set": set})
}

fn track(key: &str, initial: i64, minimum: i64, maximum: i64) -> Value {
    json!({"key": key, "initial": initial, "minimum": minimum, "maximum": maximum})
}

async fn track_value(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(key).fetch_one(&h.pool).await?)
}

async fn achievements(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key=$1")
        .bind(key).fetch_one(&h.pool).await?)
}

async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1")
        .bind(CLAIM).fetch_one(&h.pool).await?)
}

fn advance_interaction(owner: &mut ComposedFreshAdmission<'_, '_, '_>) -> TestResult {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(1))
        .ok_or("world interaction clock")?;
    owner.states.world_gate.lock().map_err(|_| "world interaction gate")?.begin_turn();
    Ok(())
}

fn claim_target(owner: &ComposedFreshAdmission<'_, '_, '_>) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("claim map")?;
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
        .ok_or("claim placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget { placement, expected_revision: map.facts.object_revision(key) })
}

async fn use_claim(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
) -> TestResult<UseDisposition> {
    let actor = admitted.runtime_actor.ok_or("admitted actor")?;
    let map = owner.bundle_map().ok_or("claim map")?;
    let target = claim_target(owner)?;
    Ok(owner.use_quest_callback(
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: admitted.item_fence,
        },
        map,
        &target,
    ).await.ok_or("claim callback did not dispatch")?.disposition)
}

async fn contend_for_daily_claim(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    h: &Harness,
) -> TestResult<[UseDisposition; 2]> {
    let first_command = 21_u64;
    let mut cause = admitted.game_session_id.as_bytes().to_vec();
    cause.extend_from_slice(&first_command.to_be_bytes());
    let mut blocker = h.pool.begin().await?;
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:reward-claim-mint:' || encode($1, 'hex'), 0))",
    )
    .bind(cause)
    .execute(&mut *blocker)
    .await?;
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await?;

    let first = use_claim(owner, admitted, first_command);
    tokio::pin!(first);
    // Observe the first USE inside the durable writer before dispatching the second.
    let writer_waiting = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM pg_stat_activity \
                 WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(blocker_pid)
            .fetch_one(&h.pool)
            .await?;
            if blocked {
                return Ok::<(), sqlx::Error>(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    });
    tokio::select! {
        result = &mut first => return Err(format!("first claim finished before acquiring the reward cause: {result:?}").into()),
        waiting = writer_waiting => waiting??,
    }

    // Poll the second USE while the first is still waiting on the reward cause.
    let (first, second, released) = tokio::join!(biased;
        first, use_claim(owner, admitted, 22), blocker.rollback(),
    );
    released?;
    let first = first?;
    assert_eq!(first, UseDisposition::Committed);
    Ok([first, second?])
}

async fn tame_noble_lion(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget};
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

    let mut ordinal = 88_100;
    super::qengine_voucher_stories::mint(owner, admitted, LIONS_HEART, 1, &mut ordinal).await?;
    let source = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i21439", "definition-r1"))
        .ok_or("Lion's Heart definition")?;
    let actor = admitted.runtime_actor.ok_or("admitted lion tamer")?;
    let entity = {
        let mut runtime = owner.runtime.lock().await;
        let from = runtime.read_actor_position(actor).map_err(debug)?.position();
        let target = runtime.admit_source_pinned_lab_creature(
            crate::foundation::MovementLocalPosition { x: from.x + 1, ..from },
            LION, 100,
        ).map_err(debug)?;
        let visible = runtime.visible_entities();
        let lion = visible.creatures.iter().find(|creature| creature.actor == target)
            .ok_or("visible Noble Lion")?;
        EntityRef { identity: target.placement_identity(), generation: lion.generation }
    };
    crate::gameplay_transport::mount_taming::test_draw::set(0);
    let result = owner.tame_mount_impl(
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 88_101,
            item_fence: admitted.item_fence,
        },
        UseIntent {
            target: UseTarget::ItemByDefinition(source),
            use_with: Some(entity),
            use_with_map_item: None,
            use_with_position: None,
        },
        None,
        None,
        true,
    ).await.map_err(|()| "Noble Lion taming dispatch")?.ok_or("Noble Lion taming result")?;
    assert_eq!(result.disposition, UseDisposition::Committed);
    let entitlement: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_account_appearance_unlocks WHERE unlock_key='oteryn:taming.noble_lion')",
    ).fetch_one(&h.pool).await?;
    assert!(entitlement, "the normal taming owner grants the Noble Lion before the fountain");
    Ok(())
}

async fn full_backpack_retry(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::foundation::{CommandId, CommandRef};
    use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
    use oteryn_protocol_oteryn::world_object::{decode_use_result, encode_use_intent};

    let native = owner.active_generation.and_then(|generation| generation.native_gameplay())
        .ok_or("native item policies")?;
    let bag = generation_item_facts(native, &TypedDefinitionRef {
        family:"Item".into(), production_key:"oteryn:item.tibia.i2854".into(),
        revision_ref:"definition-r1".into(),
    }).map_err(debug)?;
    let capacity = i64::from(bag.container_capacity.ok_or("backpack capacity")?);
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
        .fetch_one(&h.pool).await?;
    assert!(occupied < capacity);
    let mut ordinal = 89_000;
    for _ in occupied..capacity {
        super::qengine_voucher_stories::mint(owner, admitted, 3031, 100, &mut ordinal).await?;
    }
    let occupied: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries")
        .fetch_one(&h.pool).await?;
    assert_eq!(occupied, capacity);
    let before = super::qengine_voucher_stories::quantity(h, 2854).await?;
    let before_red = super::qengine_voucher_stories::quantity(h, 3039).await?;
    let before_yellow = super::qengine_voucher_stories::quantity(h, 3037).await?;
    let payload = encode_use_intent(&claim_target(owner)?).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Rejected);
    assert_eq!(receipts(h).await?, 0);
    let (tracks, mints): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_tracks WHERE track_key IN ($1,$2)),                 (SELECT count(*) FROM game_reward_claim_mint_receipts                  WHERE game_session_id=encode($3,'hex')::uuid AND command_id=1)",
    ).bind(LEDGER).bind(CHOICE).bind(admitted.game_session_id.as_bytes().as_slice())
        .fetch_one(&h.pool).await?;
    assert_eq!((tracks, mints), (0, 0), "failed delivery leaves no claim or cooldown");
    assert_eq!(super::qengine_voucher_stories::quantity(h, 2854).await?, before);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3039).await?, before_red);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3037).await?, before_yellow);

    let filler: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT i.item_instance_id FROM game_item_container_entries e          JOIN game_item_instances i USING (item_instance_id)          WHERE i.definition_production_key='oteryn:item.tibia.i3031' AND i.quantity=100          ORDER BY i.item_instance_id LIMIT 1",
    ).fetch_one(&h.pool).await?;
    let fence = admitted.item_fence.ok_or("item fence")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (runtime.content_pin().server_artifact_digest(), runtime.content_pin().frame_binding_digest())
    };
    let revision: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    let map = owner.bundle_map().ok_or("claim map")?;
    let permit = owner.spell_lane_permit().await.ok_or("item lane")?;
    let moved = owner.root.commit_item_loop_move(
        &permit, owner.character, owner.holder, &fence,
        CommandRef::new(admitted.game_session_id, CommandId::new(89_990).map_err(debug)?),
        &NativeItemWeights(native),
        LoopMoveRequest {
            item:*filler.as_bytes(), count:None, pickup_cell:None,
            destination:LoopDestination::Ground(LoopGround {
                spatial_position:crate::durability::map_item_mint_audit::map_item_spatial_position(
                    AT.x - 1, AT.y, AT.floor,
                ),
                map_revision:crate::map::overlay::map_revision(map.overlay.base()),
                content_revision:format!("sha256:{revision}"),
                placement_context:frame.to_vec(),
            }),
        },
    ).await.map_err(debug)?;
    assert!(moved.is_ok(), "the production move frees a backpack slot: {moved:?}");
    drop(permit);

    advance_interaction(owner)?;
    let payload = encode_use_intent(&claim_target(owner)?).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(track_value(h, LEDGER).await?, 100_000_000);
    assert_eq!(track_value(h, CHOICE).await?, 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 2854).await?, before + 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3039).await?, before_red + 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3037).await?, before_yellow + 1);
    let children: Vec<(String, i64, bool)> = sqlx::query_as(
        "SELECT child.definition_production_key, child.quantity,
                 claim.created_xact_id=quest.created_xact_id
                 AND child.xmin::text=claim.xmin::text AND nested.xmin::text=claim.xmin::text
         FROM game_reward_claim_mint_receipts claim
         JOIN game_character_quest_receipts quest ON quest.character_id=claim.character_id
           AND quest.cause_id=claim.game_session_id AND quest.cause_ordinal=claim.command_id
         JOIN game_item_nested_entries nested ON nested.parent_item_instance_id=claim.item_instance_id
         JOIN game_item_instances child ON child.item_instance_id=nested.item_instance_id
         WHERE quest.transition_key=$1 AND claim.command_id=2
         ORDER BY nested.placement_ordinal",
    ).bind(CLAIM).fetch_all(&h.pool).await?;
    assert_eq!(children, [
        ("oteryn:item.tibia.i3039".to_owned(), 1_i64, true),
        ("oteryn:item.tibia.i3037".to_owned(), 1_i64, true),
    ], "the retry places both children in the claimed backpack in the claim transaction");
    Ok(())
}

async fn fountain_transform(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::encode_use_intent;
    let source_appearance = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<u16> {
        let map = owner.bundle_map().ok_or("fountain map")?;
        let tile = map.overlay.base().tile(AT.x, AT.y, AT.floor).ok_or("fountain tile")?;
        let (id, _) = tile.ids().iter().zip(tile.depths())
            .filter(|(_, depth)| **depth == 0).nth(1).ok_or("fountain stack entry")?;
        Ok(map.facts.base_entry(
            crate::map::overlay::TilePos { x: AT.x, y: AT.y, floor: AT.floor },
            1,
            *id,
        ).ok_or("fountain source facts")?.appearance_id)
    };
    let holy_area_count = |owner: &ComposedFreshAdmission<'_, '_, '_>| -> TestResult<usize> {
        let map = owner.bundle_map().ok_or("fountain map")?;
        Ok(map.facts.movement_effects_since(0).into_iter()
            .filter(|(_, _, effect)| *effect == HOLY_AREA_EFFECT).count())
    };
    assert_eq!(source_appearance(owner)?, LIONS_FOUNTAIN);
    let before = super::qengine_voucher_stories::quantity(h, 3039).await?;
    let original = claim_target(owner)?;
    let payload = encode_use_intent(&original).map_err(debug)?;
    owner.states.quest_callbacks.interrupt_after_commit
        .store(true, std::sync::atomic::Ordering::Release);
    assert!(super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await.is_err());
    assert!(owner.has_pending_quest_callback(admitted.game_session_id));
    assert_eq!(source_appearance(owner)?, LIONS_FOUNTAIN);
    assert_eq!(holy_area_count(owner)?, 0);
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(track_value(h, CHOICE).await?, 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3039).await?, before + 1);

    assert!(owner.reconcile_quest_callbacks(admitted.game_session_id).await);
    assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
    assert_eq!(source_appearance(owner)?, LIONS_SANCTUARY_ROCK);
    assert_eq!(claim_target(owner)?.expected_revision, original.expected_revision + 1);
    assert_eq!(holy_area_count(owner)?, 1);
    assert!(owner.reconcile_quest_callbacks(admitted.game_session_id).await);
    h.root.advance_test_quest_clock(DAILY_PERIOD).map_err(debug)?;
    advance_interaction(owner)?;
    assert_eq!(use_claim(owner, *admitted, 2).await?, UseDisposition::NothingToUse);
    assert_eq!(receipts(h).await?, 1, "an expired personal clock cannot reopen the shared rock");
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3039).await?, before + 1);
    assert_eq!(holy_area_count(owner)?, 1);
    let reward_lines = owner.states.chat.drain(admitted.game_session_id).into_iter()
        .filter(|line| matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. }
            if text == FOUNTAIN_RED_GEM)).count();
    assert_eq!(reward_lines, 1, "recover the selected reward message once");
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

    // The source tile is installed by the common dispatch fixture. Build the binding
    // independently so its guards and effects come entirely from this scenario.
    let mut callback = json!({
        "key":CLAIM,"quest":QUEST,"trigger":"use",
        "selector":{"position":AT,"unique":63001,"appearance":100},
        "requires_world_admission":true,"reversible":false,"passages":[],
        "storage":[],"offerings":[],"return_on_failure":false,
        "sources":[
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_pits_of_inferno_quest/actions_fountain.lua:3-15",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/lions_rock/actions_lions_rock.lua:115-139",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/actions_rewards.lua:43-72",
            "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_great_dragon_hunt_quest/actions_treasure.lua:19-40"
        ]
    });
    let well = matches!(name, WELL | WELL_LOST_ACK);
    let mut tracks = vec![];
    let mut effects;
    let period = match name {
        DAILY | FOUNTAIN_TRANSFORM => Some((DAILY_PERIOD, DAILY_REFUSAL)),
        ANNUAL | FULL_BACKPACK => Some((31_536_001_i64, ANNUAL_REFUSAL)),
        _ => None,
    };
    if let Some((seconds, refusal)) = period {
        tracks.push(track(LEDGER, -1, -1, i64::MAX));
        let choices = if name == DAILY { 2 } else { 1 };
        tracks.push(track(CHOICE, 0, 0, choices));
        callback["storage"] = json!([storage(CHOICE, 0, choices, 0)]);
        callback["state_operations"] = json!([{
            "track": LEDGER, "from": {"kind":"elapsed_at_least","seconds":seconds},
            "effect":{"kind":"set_now"}
        }]);
        callback["failure_message"] = json!(refusal);
        let give = match name {
            ANNUAL => json!({"item":"oteryn:item.tibia.i2854","count":1,
                "contents":[{"item":"oteryn:item.tibia.i3031","count":2}]}),
            FOUNTAIN_TRANSFORM => json!({"item":"oteryn:item.tibia.i3039","count":1}),
            FULL_BACKPACK => json!({"item":"oteryn:item.tibia.i2854","count":1,"contents":[
                {"item":"oteryn:item.tibia.i3039","count":1},
                {"item":"oteryn:item.tibia.i3037","count":1}
            ]}),
            _ => json!({"item":"oteryn:item.tibia.i3031","count":2}),
        };
        let mut rewards = vec![json!({"give":give})];
        if name == DAILY {
            rewards.insert(0, json!({
                "give":{"item":"oteryn:item.tibia.i21439","count":1},
                "exclude_owned_mount":LION_MOUNT
            }));
        }
        if name == FOUNTAIN_TRANSFORM {
            rewards[0]["message"] = json!(FOUNTAIN_RED_GEM);
        }
        effects = json!({"reset":{"kind":"elapsed","track":LEDGER,"seconds":seconds},
            "selection_track":CHOICE,"rewards":rewards});
        if matches!(name, DAILY | FOUNTAIN_TRANSFORM) {
            effects["actor_magic_effect"] = json!(HOLY_AREA_EFFECT);
        }
        if name == ANNUAL {
            tracks.push(track(FEATHERS, 1, 1, 2));
            let mut feathers = storage(FEATHERS, 2, 2, 2);
            feathers["preserve"] = json!(true);
            callback["storage"] = json!([storage(CHOICE, 0, choices, 0), feathers]);
            effects["eligibility"] = json!({"track":FEATHERS,"message":ANNUAL_INELIGIBLE});
        }
    } else if name == HOARD {
        tracks.extend([
            track(LEDGER, 49, 0, 50), track(COUNT, 9, 0, 100),
            track(CHOICE, 0, 0, 2), track(HUNT_GATE, 0, 0, 0),
        ]);
        let mut count = storage(COUNT, 0, 99, 1);
        count["increment"] = json!(true);
        callback["storage"] = json!([storage(LEDGER, 50, 50, 0), count, storage(CHOICE, 0, 2, 0)]);
        callback["failure_message"] = json!(HOARD_REFUSAL);
        effects = json!({"reset":{"kind":"progress","track":LEDGER,"minimum":50},
            "selection_track":CHOICE,
            "milestone":{"track":COUNT,"count":10,"achievement":HOARD_ACHIEVEMENT},
            "rewards":[
                {"give":{"item":"oteryn:item.tibia.i3031","count":2,"additional":[{"item":"oteryn:item.tibia.i3039","count":1}]}},
                {"give":{"item":"oteryn:item.tibia.i3031","count":2,"additional":[{"item":"oteryn:item.tibia.i3039","count":2}]}}
            ]});
    } else {
        assert!(well || name == STEP);
        tracks.push(track(LEDGER, 0, 0, 1));
        callback["storage"] = json!([storage(LEDGER, 0, 0, 1)]);
        callback["achievements"] = json!([WELL_ACHIEVEMENT]);
        callback["failure_message"] = json!("Sorry, not possible.");
        effects = json!({"reset":{"kind":"once","track":LEDGER}});
        if well {
            effects["restore_vitals"] = json!(true);
        } else {
            callback["trigger"] = json!("step_in");
            callback["selector"]["unique"] = Value::Null;
            callback["selector"]["appearance"] = Value::Null;
        }
    }
    callback["effects"]["claim"] = effects;
    if name == FOUNTAIN_TRANSFORM {
        callback["selector"]["appearance"] = json!(LIONS_FOUNTAIN);
        callback["effects"]["transform"] = json!(LIONS_SANCTUARY_ROCK);
    }
    let recipe_sha256: String = Sha256::digest(serde_json::to_vec(&callback)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let trigger = if name == STEP { "step_in" } else { "map_use" };
    let mut stages = vec![json!({
        "key":"claim","completes":false,"trigger":{"kind":trigger,"callback":callback}
    })];
    if name == HOARD {
        stages.push(json!({
            "key":"kill-dragon","completes":false,"trigger":{"kind":"kill","counter":{
                "key":HUNT_KILL,"creature":DRAGON,"track":LEDGER,
                "gate":HUNT_GATE,"gate_value":0,"maximum":50,
                "death_area":{"minimum":[AT.x-2, AT.y-2, -AT.floor],
                              "maximum":[AT.x+2, AT.y+2, -AT.floor]}
            }}
        }));
    }
    if name == ANNUAL {
        stages.push(json!({
            "key":"feathers","completes":false,"trigger":{"kind":"step_in","callback":{
                "key":"oteryn:quest-callback/qonce-dispatch-feathers","quest":QUEST,
                "trigger":"step_in","selector":{"position":AT},
                "requires_world_admission":true,"reversible":false,"passages":[],
                "storage":[storage(FEATHERS,1,1,2)],"offerings":[],
                "return_on_failure":false,"sources":[
                    "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_first_dragon/actions_rewards.lua:43-45"
                ]
            }}
        }));
    }
    let bindings = Bindings::parse(&serde_json::to_vec(&json!({"schema":1,"quests":[{
        "quest":QUEST,
        "recipe_sha256":recipe_sha256,
        "sources":["synthetic dispatch tests exercising the Crystal claim shapes"],
        "tracks":tracks,
        "stages":stages
    }]}))?)?;
    let revision = owner.quest_catalogue.as_deref().ok_or("catalogue")?.content_revision();
    let base = QuestStateCatalogue::empty(revision).ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    owner.states.chat.admit(admitted.game_session_id, "Claimant", owner.owner_now());

    if name == STEP {
        for (command, direction) in [(1, StepDirection::East), (2, StepDirection::West), (3, StepDirection::East)] {
            let reply = super::quest_world_callbacks::wire(owner, admitted, command, 1,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(direction)).await?;
            assert_eq!(oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
        }
        assert_eq!(receipts(h).await?, 1);
        assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 1);
        return Ok(());
    }

    if well {
        use oteryn_protocol_oteryn::world_object::{decode_use_result, encode_use_intent};

        let actor = admitted.runtime_actor.ok_or("well actor")?;
        let session = admitted.game_session_id;
        let payload = encode_use_intent(&claim_target(owner)?).map_err(debug)?;
        let full = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("well vitals")?;
            let full = state.vitals();
            assert!(full.health > 1 && full.mana > 0);
            state.restore_saved_vitals(1, 0);
            (full.health, full.mana)
        };
        if name == WELL_LOST_ACK {
            owner.states.quest_callbacks.interrupt_after_commit
                .store(true, std::sync::atomic::Ordering::Release);
            assert!(super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await.is_err());
            assert!(owner.states.quest_callbacks.has_pending_claim_vitals(session));
            assert_eq!(receipts(h).await?, 1);
            assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 1);
            let saved: (i32, i32) = sqlx::query_as("SELECT health,mana FROM game_character_last_positions")
                .fetch_one(&h.pool).await?;
            assert_eq!(saved, (i32::try_from(full.0)?, i32::try_from(full.1)?));
            {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let vitals = states.get(&runtime, actor, session).ok_or("interrupted actor")?.vitals();
                assert_eq!((vitals.health, vitals.mana), (1, 0));
            }
            assert!(owner.reconcile_quest_callbacks(session).await);
            assert!(!owner.states.quest_callbacks.has_pending_claim_vitals(session));
        } else {
            let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload.clone()).await?;
            assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
        }
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("healed actor")?;
            let vitals = state.vitals();
            assert_eq!((vitals.health, vitals.mana), full);
            let (damaged, _) = state.after_creature_damage(full.0 - 1).ok_or("well damage")?;
            *state = damaged;
        }
        if name == WELL {
            let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
            assert_eq!(decode_use_result(&reply).map_err(debug)?, UseDisposition::Committed);
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let vitals = states.get(&runtime, actor, session).ok_or("replayed actor")?.vitals();
            assert_eq!((vitals.health, vitals.mana), (1, full.1));
        }
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 2).await?, UseDisposition::Rejected);
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let vitals = states.get(&runtime, actor, session).ok_or("refused actor")?.vitals();
            assert_eq!((vitals.health, vitals.mana), (1, full.1));
        }
        let saved: (i32, i32) = sqlx::query_as("SELECT health,mana FROM game_character_last_positions")
            .fetch_one(&h.pool).await?;
        assert_eq!(saved, (i32::try_from(full.0)?, i32::try_from(full.1)?));
        assert_eq!(receipts(h).await?, 1);
        assert_eq!(achievements(h, WELL_ACHIEVEMENT).await?, 1);
        return Ok(());
    }

    h.root.set_test_quest_clock(100_000_000).map_err(debug)?;
    if name == FOUNTAIN_TRANSFORM {
        return Box::pin(fountain_transform(owner, admitted, h)).await;
    }
    if name == FULL_BACKPACK {
        return Box::pin(full_backpack_retry(owner, admitted, h)).await;
    }
    if name == DAILY {
        tame_noble_lion(owner, admitted, h).await?;
    }
    if name == HOARD {
        owner.active_generation.and_then(|generation| generation.native_gameplay())
            .ok_or("native creature policies")?
            .install_companion_policies(&mut *owner.runtime.lock().await).map_err(debug)?;
        super::qengine_kill_stories::death(owner, admitted, DRAGON, 88_001).await?;
        assert_eq!(track_value(h, LEDGER).await?, 50, "the death opens the hoard");
    }
    let before = super::qengine_voucher_stories::quantity(h, 3031).await?;
    if name == ANNUAL {
        assert_eq!(use_claim(owner, *admitted, 20).await?, UseDisposition::Rejected);
        assert_eq!(receipts(h).await?, 0);
        assert_eq!(super::qengine_voucher_stories::quantity(h, 3031).await?, before);
        let chat = owner.states.chat.drain(admitted.game_session_id);
        assert!(chat.iter().any(|line| matches!(line,
            oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. }
                if text == ANNUAL_INELIGIBLE
        )));
        let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 1,
            oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::East)).await?;
        assert_eq!(oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?, StepDisposition::Moved);
        assert_eq!(track_value(h, FEATHERS).await?, 2);
        advance_interaction(owner)?;
    }
    let before_hearts = super::qengine_voucher_stories::quantity(h, LIONS_HEART).await?;
    let before_red_gems = super::qengine_voucher_stories::quantity(h, 3039).await?;
    let outcomes = if name == DAILY {
        Box::pin(contend_for_daily_claim(owner, *admitted, h)).await?
    } else {
        let (first, second) = tokio::join!(use_claim(owner, *admitted, 21), use_claim(owner, *admitted, 22));
        [first?, second?]
    };
    let committed = outcomes.into_iter().filter(|outcome| *outcome == UseDisposition::Committed).count();
    assert_eq!(committed, 1);
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 3031).await?, before + 2);
    advance_interaction(owner)?;
    assert_eq!(use_claim(owner, *admitted, 23).await?, UseDisposition::Rejected);
    let refusal = period.map_or(HOARD_REFUSAL, |(_, refusal)| refusal);
    let chat = owner.states.chat.drain(admitted.game_session_id);
    assert!(chat.iter().any(|line| matches!(
        line,
        oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == refusal
    )));
    if let Some((seconds, _)) = period {
        h.root.advance_test_quest_clock(seconds - 1).map_err(debug)?;
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 24).await?, UseDisposition::Rejected);
        h.root.advance_test_quest_clock(1).map_err(debug)?;
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 25).await?, UseDisposition::Committed);
        advance_interaction(owner)?;
        assert_eq!(use_claim(owner, *admitted, 26).await?, UseDisposition::Rejected);
        assert_eq!(receipts(h).await?, 2);
        assert_eq!(super::qengine_voucher_stories::quantity(h, 3031).await?, before + 4);
    } else {
        assert_eq!(achievements(h, HOARD_ACHIEVEMENT).await?, 1);
        let (ledger, count): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT value FROM game_character_quest_tracks WHERE track_key=$1), \
                    (SELECT value FROM game_character_quest_tracks WHERE track_key=$2)",
        ).bind(LEDGER).bind(COUNT).fetch_one(&h.pool).await?;
        assert_eq!((ledger, count), (0, 10));
        assert!((1..=2).contains(&(super::qengine_voucher_stories::quantity(h, 3039).await? - before_red_gems)));
        super::qengine_kill_stories::death(owner, admitted, DRAGON, 88_002).await?;
        assert_eq!(track_value(h, LEDGER).await?, 1, "the next hunt can earn credit again");
        let deaths: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        ).bind(HUNT_KILL).fetch_one(&h.pool).await?;
        assert_eq!(deaths, 2, "duplicate death delivery earns one credit per death");
    }
    if name == DAILY {
        assert_eq!(track_value(h, CHOICE).await?, 2);
        assert_eq!(super::qengine_voucher_stories::quantity(h, LIONS_HEART).await?, before_hearts);
        let actor = admitted.runtime_actor.ok_or("fountain claimant")?;
        let cell = owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position();
        let claimant_cell = crate::map::overlay::TilePos {
            x: cell.x.try_into()?, y: cell.y.try_into()?, floor: (-cell.floor).try_into()?,
        };
        let holy_area: Vec<_> = owner.bundle_map().ok_or("fountain map")?.facts
            .movement_effects_since(0).into_iter()
            .filter_map(|(_, position, effect)| (effect == HOLY_AREA_EFFECT).then_some(position))
            .collect();
        assert_eq!(holy_area, [claimant_cell, claimant_cell]);

    }
    let atomic: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts m \
         JOIN game_character_quest_receipts q ON q.character_id=m.character_id \
          AND q.cause_id=m.game_session_id AND q.cause_ordinal=m.command_id \
          AND q.created_xact_id=m.created_xact_id WHERE q.transition_key=$1",
    ).bind(CLAIM).fetch_one(&h.pool).await?;
    assert_eq!(atomic, receipts(h).await?);
    Ok(())
}
