use super::fixture::{Harness, debug, run};
use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop};
use super::quest_world_callbacks_fixtures::{local, wire};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::NativeEntryStart;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ChannelContentPin, CommandId, CommandRef};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::map_door::WORLD_ACTION_COOLDOWN;
use crate::gameplay_transport::quest_log::{QuestLogLines, QuestLogObservation, QuestLogState};
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::quest_log::{QuestLogQuery, QuestLogView, decode_quest_log};
use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};
use std::sync::Arc;

const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline";
// Palimuth's source-lowered questline update to 53: later progress must survive a threshold-21 gate.
const ADVANCE: &str = "oteryn:quest-transition/quest/u8_4/in_service_of_yalahar/questline/npc_23";
const SOURCE: Position = Position {
    x: 32743,
    y: 31161,
    floor: -5,
};
const START: Position = Position { x: 32742, ..SOURCE };
const DESTINATION: Position = Position {
    x: 32744,
    y: 31164,
    floor: -5,
};

const CHEST: &str = "oteryn:reward-claim.quest.u8_6.wrath_of_the_emperor.chest_items";
const CHOICES: [(u16, u16, u16, u32); 4] = [
    (33076, 31170, 6308, 11687),
    (33078, 31170, 6299, 11686),
    (33080, 31170, 6300, 11689),
    (33079, 31173, 7825, 50263),
];
const DISPLAYS: [(u16, u16, u8, u16); 4] = [
    (33076, 31169, 4, 11687),
    (33078, 31169, 3, 11686),
    (33080, 31169, 3, 11689),
    (33080, 31173, 2, 50263),
];

#[test]
fn n4_yalahar_mechanism_checks_durable_progress_without_rewriting_it() -> TestResult {
    scenario("n4_yalahar", Case::Guarded)
}

#[test]
fn n4_wrath_choice_claims_exactly_once_across_chests_and_owner_reload() -> TestResult {
    scenario("n4_wrath_choice", Case::Claim)
}

#[derive(Clone, Copy)]
enum Case {
    Guarded,
    Public(u16),
    Facing,
    Claim,
    Projection,
    Display,
    DisplayReplay,
    Capacity,
    Wheel,
}

#[test]
fn n4_wrath_chests_are_usable_from_projected_targets() -> TestResult {
    scenario("n4_wrath_projection", Case::Projection)
}

#[test]
fn n4_wrath_reward_displays_refuse_map_move() -> TestResult {
    scenario("n4_wrath_display", Case::Display)
}

#[test]
fn n4_wrath_reward_displays_resume_only_their_historical_mint() -> TestResult {
    scenario("n4_wrath_display_replay", Case::DisplayReplay)
}

#[test]
fn n4_wrath_reward_uses_live_wheel_capacity() -> TestResult {
    scenario("n4_wrath_wheel", Case::Wheel)
}

#[test]
fn n4_wrath_full_and_overweight_use_have_distinct_feedback() -> TestResult {
    scenario("n4_wrath_capacity", Case::Capacity)
}

#[test]
fn n4_yalahar_public_gates_keep_the_revision_for_two_players() -> TestResult {
    let first = scenario("n4_yalahar_public_1018", Case::Public(32775));
    let second = scenario("n4_yalahar_public_1019", Case::Public(32778));
    first.and(second)
}

#[test]
fn n4_yalahar_teleport_faces_from_the_actual_departure() -> TestResult {
    scenario("n4_yalahar_facing", Case::Facing)
}

fn scenario(name: &'static str, case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        run(async |admin| {
            let harness =
                Harness::create_with_admission_seed(admin, name, matches!(case, Case::Wheel), false)
                    .await?;
            let result = exercise(&harness, case).await;
            harness.cleanup().await?;
            result
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn exercise(h: &Harness, case: Case) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
    let (controller, activated) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("active generation")?;
    let public_x = match case {
        Case::Public(x) => Some(x),
        Case::Facing => Some(32775),
        _ => None,
    };
    let crop = if let Some(x) = public_x {
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x128-y121.b3"
            )),
            region: (7, 128, 121),
            bounds: (32772, 31115, 32781, 31121),
            start: (x, 31117),
            expected: vec![ExpectedTile {
                position: (x, 31118),
                ids: vec![if x == 32775 { 926 } else { 927 }, 8618],
                counted_entry: None,
            }],
        }
    } else if !matches!(case, Case::Guarded) {
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z08-x129-y121.b3"
            )),
            region: (8, 129, 121),
            bounds: (33073, 31169, 33081, 31175),
            start: (33079, 31171),
            expected: CHOICES
                .into_iter()
                .map(|(x, y, _, _)| ExpectedTile {
                    position: (x, y),
                    ids: vec![if y == 31173 { 11001 } else { 10997 }, 2473],
                    counted_entry: None,
                })
                .collect(),
        }
    } else {
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z05-x127-y121.b3"
            )),
            region: (5, 127, 121),
            bounds: (32742, 31161, 32745, 31165),
            start: (START.x, START.y),
            expected: vec![ExpectedTile {
                position: (SOURCE.x, SOURCE.y),
                ids: vec![923, 8618],
                counted_entry: None,
            }],
        }
    };
    let native = generation.native_gameplay().ok_or("native item facts")?;
    let world = if public_x.is_some() {
        use super::l6_world_pickups_fixtures::{ExpectedTeleport, native_scene};
        native_scene(
            native,
            vec![crop],
            &[
                ExpectedTeleport {
                    position: (32773, 31118, 7),
                    ordinal: 2,
                    destination: (32775, 31120, 7),
                },
                ExpectedTeleport {
                    position: (32780, 31118, 7),
                    ordinal: 1,
                    destination: (32778, 31120, 7),
                },
            ],
        )?
    } else {
        native_crop(native, crop)?
    };
    let origin = world.start();
    let base = activated.qualified_room().clone();
    let cells = world.movement_cells(base.movement_cells()).map_err(debug)?;
    let frame = world.frame_binding_digest(world_id);
    let start = local(Position {
        x: origin.x,
        y: origin.y,
        floor: origin.floor,
    });
    let room = base
        .bundle_view(
            cells,
            frame,
            world.map_revision_digest(),
            NativeEntryStart {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .ok_or("Yalahar room")?;
    let mut deps = Dependencies::with_content_and_capacity(
        ChannelContentPin::from_activation(
            world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (start.x, start.y, start.floor),
        ),
        base,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        2,
    )?;
    deps.combat_room = Some(room.clone());
    let premium = if matches!(case, Case::Wheel) {
        use crate::gameplay_transport::spell_premium_coordinator::{
            SpellPremiumCoordinator, purchase_fixture,
        };
        let (consumer, clock, activation, _) = purchase_fixture::current(super::fixture::id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        Some(SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        ))
    } else {
        None
    };
    let mut owner = deps.owner(h, &authority);
    owner.premium_coordinator = premium.as_ref();
    owner.active_generation = Some(generation);
    owner.movement_cells = room.movement_cells().into();
    let content = load_embedded_quest_state("content-1").map_err(debug)?;
    owner.quest_catalogue = Some(Arc::new(content.catalogue().clone()));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let session = admitted.game_session_id;
    let world = admitted.world_id.as_bytes().as_slice();
    let channel = admitted.channel_id.as_bytes().as_slice();
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(world).bind(channel).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(world).bind(channel)
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    admitted.item_fence = Some(fence.item_fence());
    deps.runtime
        .lock()
        .await
        .initialize_pinned_test_position(deps.actor, start)
        .map_err(debug)?;
    deps.initialize(&owner).await;
    owner.admit_progression_session(&mut admitted).await;
    match case {
        Case::Guarded => {}
        Case::Public(x) => return public_gate(h, &deps, &owner, admitted, x).await,
        Case::Facing => return public_gate_facing(&owner, admitted).await,
        Case::Claim => return wrath_chest(h, &deps, &authority, &owner, admitted).await,
        Case::Projection => return wrath_projection(h, &owner, admitted).await,
        Case::Display => return wrath_displays(h, &owner, admitted).await,
        Case::DisplayReplay => return wrath_display_replay(h, &owner, admitted).await,
        Case::Capacity => return wrath_capacity(h, &owner, admitted).await,
        Case::Wheel => return wrath_wheel(h, &owner, admitted).await,
    }
    owner.quest_log_content = Some(
        QuestLogLines::new(
            oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&content)?.content,
        )
        .ok_or("journal lines")?,
    );
    owner
        .states
        .chat
        .admit(session, "Traveller", owner.owner_now());
    owner.admit_quest_session(&admitted).await;
    let before = owner.observe_quest_log(deps.actor, session, None).await;
    let mut continuity = admitted.continuity.quest_log;
    let (mut journal, _) =
        QuestLogState::snapshot(&mut continuity, before.clone(), false).map_err(debug)?;
    let listed = journal
        .query(&mut continuity, &QuestLogQuery::List, before)
        .ok_or("journal list")?
        .map_err(debug)?;
    let target = super::questpc7_hoist_story_fixtures::target(&owner, SOURCE, 8618)?;
    let position = async || -> TestResult<_> {
        Ok(deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position())
    };
    let denied = wire(&owner, &mut admitted, 1, 2, target.clone()).await?;
    assert_eq!(
        decode_use_result(&denied).map_err(debug)?,
        UseDisposition::Rejected
    );
    assert_eq!(position().await?, start);
    assert!(effects(&owner)?.is_empty());
    assert!(
        owner
            .states
            .chat
            .drain(session)
            .iter()
            .any(|line| matches!(line,
        oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. }
            if text == "The gate mechanism won't move. You probably have to find a way around until you figure out how to operate the gate."))
    );
    let catalogue = owner.quest_catalogue.clone().ok_or("quest catalogue")?;
    let request = QuestTransitionRequest {
        transition_key: ADVANCE.into(),
        cause: QuestCause::Command(CommandRef::new(
            session,
            CommandId::new(900).map_err(debug)?,
        )),
    };
    assert!(matches!(
        h.root
            .commit_character_quest_transition(&authority, &h.node, fence, request, catalogue)
            .await?,
        QuestTransitionOutcome::Committed(_)
    ));
    owner.refresh_quest_session(session, false).await;
    let changed = owner
        .observe_quest_log(deps.actor, session, journal.version())
        .await;
    let delta = journal
        .refresh(&mut continuity, changed)
        .map_err(debug)?
        .ok_or("journal delta")?;
    assert_eq!(delta.to, listed.to + 1);
    let QuestLogObservation::Changed(source) =
        owner.observe_quest_log(deps.actor, session, None).await
    else {
        return Err("journal source".into());
    };
    let (index, _) = source
        .titles()
        .into_iter()
        .find(|(_, title)| title == "In Service of Yalahar Quest")
        .ok_or("Yalahar journal entry")?;
    let Some(QuestLogView::List(entries)) = decode_quest_log(&delta.payload).map_err(debug)?.view
    else {
        return Err("journal delta did not carry a quest list".into());
    };
    assert!(entries.iter().any(|entry| entry.quest_index == index));
    let before = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await?;
    tokio::time::sleep(WORLD_ACTION_COOLDOWN).await;
    let accepted = wire(&owner, &mut admitted, 2, 2, target.clone()).await?;
    assert_eq!(
        decode_use_result(&accepted).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(position().await?, local(DESTINATION));
    let projected = effects(&owner)?;
    assert_eq!(projected.len(), 2);
    for ((_, position, effect), expected) in projected.iter().zip([START, DESTINATION]) {
        assert_eq!(
            (position.x, position.y, position.floor, *effect),
            (expected.x, expected.y, expected.floor, 11)
        );
    }
    assert_eq!(wire(&owner, &mut admitted, 2, 2, target).await?, accepted);
    assert_eq!(effects(&owner)?, projected);
    assert_eq!(position().await?, local(DESTINATION));
    owner.refresh_quest_session(session, false).await;
    let after = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await?;
    assert_eq!(after.tracks().get(QUESTLINE), Some(&53));
    assert_eq!(before, after);
    let observation = owner
        .observe_quest_log(deps.actor, session, journal.version())
        .await;
    assert!(
        journal
            .refresh(&mut continuity, observation)
            .map_err(debug)?
            .is_none()
    );
    Ok(())
}

fn effects(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Vec<(u64, crate::map::overlay::TilePos, u16)>> {
    Ok(owner
        .bundle_map()
        .ok_or("native map")?
        .facts
        .movement_effects_since(0))
}

async fn wrath_chest(
    h: &Harness,
    deps: &Dependencies,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;

    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("world interaction capability")?;
    let fence = admitted.item_fence.ok_or("item fence")?;
    let session = admitted.game_session_id;
    let map = owner.bundle_map().ok_or("native map")?;
    let at = Position {
        x: 33079,
        y: 31171,
        floor: -8,
    };
    let selected = projected_target(
        owner,
        at,
        Position {
            x: 33078,
            y: 31170,
            ..at
        },
        2473,
    )?;
    let alternate = projected_target(
        owner,
        at,
        Position {
            x: 33080,
            y: 31170,
            ..at
        },
        2473,
    )?;
    let ledger = async || -> TestResult<(i64, i64)> {
        Ok(sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2), (SELECT count(*) FROM game_reward_claim_mint_receipts)",
        ).bind(fence.character_id.as_bytes().as_slice()).bind(CHEST).fetch_one(&h.pool).await?)
    };
    let denied = wire(owner, &mut admitted, 1, 2, selected.clone()).await?;
    assert_eq!(
        decode_use_result(&denied).map_err(debug)?,
        UseDisposition::NoRoom
    );
    assert_eq!(
        ledger().await?,
        (0, 0),
        "no backpack leaves the entire claim unspent"
    );
    super::item_look_fixtures::seed_main_backpack(h, &[]).await?;
    tokio::time::sleep(WORLD_ACTION_COOLDOWN).await;
    let claimed = wire(owner, &mut admitted, 2, 2, selected.clone()).await?;
    assert_eq!(
        decode_use_result(&claimed).map_err(debug)?,
        UseDisposition::Committed
    );
    assert_eq!(
        wire(owner, &mut admitted, 2, 2, selected.clone()).await?,
        claimed
    );
    tokio::time::sleep(WORLD_ACTION_COOLDOWN).await;
    assert_eq!(ledger().await?, (1, 1));
    let backpack = async || -> TestResult<_> {
        Ok(h.root
            .read_character_backpack(authority, fence.character_id)
            .await?
            .ok_or("equipped backpack")?)
    };
    let before = backpack().await?;
    assert_eq!(before.entries.len(), 1);
    assert_eq!(before.entries[0].item.quantity, 1);
    assert_eq!(
        before.entries[0].item.definition.production_key,
        "oteryn:item.tibia.i11686"
    );

    // Remove the original bag from the equipped slot, leaving its instance and contents intact.
    replace_backpack(h, None).await?;
    let denied = wire(owner, &mut admitted, 3, 2, alternate).await?;
    let spent = decode_use_result(&denied).map_err(debug)?;
    super::item_look_fixtures::seed_character_backpack(h, 41, 72, &[]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i2853' WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(super::fixture::id(72).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;

    // Rebuild the server owner and replay against a different equipped bag.
    let mut reloaded = deps.owner(h, authority);
    reloaded.active_generation = owner.active_generation;
    reloaded.movement_cells = owner.movement_cells.clone();
    reloaded.quest_catalogue = owner.quest_catalogue.clone();
    deps.initialize(&reloaded).await;
    reloaded.refresh_quest_session(session, false).await;
    let mut replay = deps.admitted;
    replay.first_entry = FirstEntryOutcome::Positioned;
    replay.item_fence = Some(fence);
    let target =
        oteryn_protocol_oteryn::world_object::decode_use_intent(&selected).map_err(debug)?;
    // Fresh ingress starts at one. The first attempt has no mint receipt and now
    // sees the spent claim. The second returns its original durable mint receipt.
    let replayed =
        super::quest_callbacks_fixtures::wire_uses(&reloaded, replay, &[1, 2], &target).await?;
    assert_eq!(
        (spent, replayed),
        (
            UseDisposition::NothingToUse,
            vec![UseDisposition::NothingToUse, UseDisposition::Committed]
        ),
        "spent claim precedes the equipped slot and a replay uses the original destination binding"
    );
    assert!(backpack().await?.entries.is_empty());
    replace_backpack(h, Some(70)).await?;
    let after = backpack().await?;
    assert_eq!(after.entries.len(), 1);
    assert_eq!(after.entries[0].item, before.entries[0].item);
    assert_eq!(ledger().await?, (1, 1));
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        map.overlay.base().as_ref(),
        &|_| true,
    )?;
    let choices = &report
        .served
        .iter()
        .find(|row| row.key.as_str() == CHEST)
        .ok_or("shared claim")?
        .placements;
    assert_eq!(choices.len(), CHOICES.len());
    for (x, y, _, item) in CHOICES {
        let row = choices
            .iter()
            .find(|row| (row.cell.x, row.cell.y) == (x, y))
            .ok_or("native choice")?;
        assert_eq!(row.placement.items.len(), 1);
        assert_eq!(
            row.placement.items[0].item.key().as_str(),
            format!("oteryn:item.tibia.i{item}")
        );
    }
    Ok(())
}

async fn replace_backpack(h: &Harness, bag: Option<u8>) -> TestResult {
    use super::fixture::id;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    if let Some(bag) = bag {
        sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(bag).as_slice()).bind(id(42).as_slice()).bind(id(74).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

fn projected_item(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    at: Position,
    target: Position,
    appearance: u16,
) -> TestResult<oteryn_protocol_oteryn::world_map::MapOrigin> {
    use crate::gameplay_transport::item_view::{ItemViewContinuity, SessionItemView};
    use crate::gameplay_transport::world_map::{MapViewSource, SessionMapView};
    let map = owner.bundle_map().ok_or("native map")?;
    let source = MapViewSource {
        overlay: &map.overlay,
        facts: &map.facts,
        content_generation: owner
            .active_generation
            .ok_or("active generation")?
            .identity()
            .client_artifact_digest(),
        reset_epoch: owner.reset_epoch,
    };
    let mut items = SessionItemView::resume(ItemViewContinuity::default()).with_map_view();
    let snapshot = SessionMapView::default()
        .snapshot(
            &mut items,
            &source,
            oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: i32::from(at.x),
                y: i32::from(at.y),
                floor: i16::from(at.floor),
            },
        )
        .map_err(debug)?;
    let snapshot = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(debug)?;
    let tile = snapshot
        .tiles
        .iter()
        .find(|t| {
            (t.position.x, t.position.y, t.position.floor)
                == (
                    i32::from(target.x),
                    i32::from(target.y),
                    -i16::from(target.floor),
                )
        })
        .ok_or("projected tile")?;
    let item = tile
        .items
        .iter()
        .find(|i| i.appearance_id == appearance)
        .ok_or("projected item")?;
    Ok(item.origin)
}

fn projected_target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    at: Position,
    target: Position,
    appearance: u16,
) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::world_map::MapOrigin;
    use oteryn_protocol_oteryn::world_object::{WorldObjectTarget, encode_use_intent};
    let origin = projected_item(owner, at, target, appearance)?;
    let MapOrigin::BaseOrdinal {
        ordinal,
        object_revision,
    } = origin
    else {
        return Err(
            format!("object must project a base ordinal at {target:?}, got {origin:?}").into(),
        );
    };
    let map = owner.bundle_map().ok_or("native map")?;
    let key = oteryn_world_bundle::bundle::placement_key(target.floor, target.x, target.y, ordinal)
        .ok_or("placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    encode_use_intent(&WorldObjectTarget {
        placement,
        expected_revision: object_revision,
    })
    .map_err(debug)
}

async fn relocate(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    destination: Position,
) -> TestResult {
    let _permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut runtime = owner.runtime.lock().await;
    let before = runtime.read_actor_position(actor).map_err(debug)?;
    if before.position() == local(destination) {
        return Ok(());
    }
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport_facing(
        &runtime,
        owner.movement_cells(),
        actor,
        admitted.game_session_id,
        before,
        local(destination),
        false,
        &doors,
        None,
    )
    .map_err(|error| format!("relocate to {destination:?}: {error:?}"))?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

async fn wrath_projection(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
) -> TestResult {
    super::item_look_fixtures::seed_main_backpack(h, &[]).await?;
    for (i, (x, y, _, _)) in CHOICES.into_iter().enumerate() {
        let target = Position { x, y, floor: -8 };
        let at = target;
        relocate(owner, admitted, at).await?;
        let payload = projected_target(owner, at, target, 2473)?;
        tokio::time::sleep(WORLD_ACTION_COOLDOWN).await;
        let id = admitted.continuity.next_command_id;
        let result = wire(owner, &mut admitted, id, 2, payload).await?;
        assert_eq!(
            decode_use_result(&result).map_err(debug)?,
            if i == 0 {
                UseDisposition::Committed
            } else {
                UseDisposition::NothingToUse
            }
        );
    }
    Ok(())
}

async fn wrath_displays(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{
        LoopMove, LoopMoveRefusal, LoopMoveResult, LoopTarget,
    };
    use crate::gameplay_transport::item_view::ItemKey;
    use oteryn_protocol_oteryn::world_map::MapOrigin;
    super::item_look_fixtures::seed_main_backpack(h, &[]).await?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut results = Vec::new();
    for (x, y, ordinal, id) in DISPLAYS {
        let target = Position { x, y, floor: -8 };
        let at = if y == 31169 {
            Position { y: y + 1, ..target }
        } else {
            Position { x: x - 1, ..target }
        };
        relocate(owner, admitted, at).await?;
        let origin = projected_item(owner, at, target, id)?;
        let key =
            oteryn_world_bundle::bundle::placement_key(-8, x, y, ordinal).ok_or("display key")?;
        let moved = owner
            .move_loop_item(
                admitted.runtime_actor.ok_or("actor")?,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 100 + u64::from(id),
                    item_fence: admitted.item_fence,
                },
                LoopMove {
                    item: ItemKey::MapBase {
                        bundle_digest: map.overlay.base().digest(),
                        placement_key: key,
                        reset_epoch: owner.reset_epoch,
                    },
                    count: None,
                    target: LoopTarget::MainBackpack,
                    slot: None,
                },
            )
            .await;
        results.push((origin, moved, ordinal));
    }
    for (origin, moved, ordinal) in results {
        assert_eq!(moved, LoopMoveResult::Refused(LoopMoveRefusal::Stale));
        assert_eq!(
            origin,
            MapOrigin::BaseOrdinal {
                ordinal,
                object_revision: 0
            }
        );
    }
    let minted: i64 = sqlx::query_scalar("SELECT count(*) FROM game_map_item_mint_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(minted, 0);
    Ok(())
}

async fn wrath_display_replay(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::durability::map_item_mint::{MapItemMintOutcome, MapItemMintRequest};
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::item_move::{
        LoopMove, LoopMoveRefusal, LoopMoveResult, LoopTarget,
    };
    use crate::gameplay_transport::item_ref_admission::generation_item_facts;
    use crate::gameplay_transport::item_view::ItemKey;

    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("item fence")?;
    let map = owner.bundle_map().ok_or("native map")?;
    let digest = map.overlay.base().digest();
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native items")?;
    let content_revision = format!(
        "sha256:{}",
        native
            .source_digest()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    super::item_look_fixtures::seed_main_backpack(h, &[]).await?;
    let mut expected = Vec::new();
    let mut results = Vec::new();
    for (x, y, ordinal, id) in DISPLAYS {
        let at = Position {
            x,
            y: if y == 31169 { y + 1 } else { y },
            floor: -8,
        };
        let at = if y == 31169 {
            at
        } else {
            Position { x: x - 1, ..at }
        };
        relocate(owner, admitted, at).await?;
        let key =
            oteryn_world_bundle::bundle::placement_key(-8, x, y, ordinal).ok_or("display key")?;
        let command_id = 100 + u64::from(id);
        let command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: Some(fence),
        };
        let mint = MapItemMintRequest {
            flat_crate: None,
            key_number: None,
            command: CommandRef::new(
                admitted.game_session_id,
                CommandId::new(command_id).map_err(debug)?,
            ),
            base_bundle_digest: digest,
            placement_key: key,
            reset_epoch: owner.reset_epoch,
            item: generation_item_facts(
                native,
                &crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: format!("oteryn:item.tibia.i{id}"),
                    revision_ref: "definition-r1".into(),
                },
            )
            .map_err(debug)?,
            quantity: 1,
            content_revision: content_revision.clone(),
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        };
        // Commit only the first phase of a pre-upgrade MOVE, then retry via the current owner.
        let committed = {
            let permit = owner.spell_lane_permit().await.ok_or("spell lane")?;
            let mut frozen = h
                .root
                .freeze_map_item_mint(owner.character, owner.holder, fence, mint)
                .await
                .map_err(debug)?;
            h.root
                .commit_map_item_mint(&permit, owner.character, owner.holder, fence, &mut frozen)
                .await
                .map_err(debug)?
        };
        let MapItemMintOutcome::Committed(minted) = committed else {
            return Err("historical mint".into());
        };
        expected.push(minted.item_instance_id);
        owner.states.loop_ground.invalidate();
        let request = LoopMove {
            item: ItemKey::MapBase {
                bundle_digest: digest,
                placement_key: key,
                reset_epoch: owner.reset_epoch,
            },
            count: None,
            target: LoopTarget::MainBackpack,
            slot: None,
        };
        let fresh = owner
            .move_loop_item(
                actor,
                UseCommand {
                    command_id: command_id + 1,
                    ..command
                },
                request,
            )
            .await;
        assert_eq!(fresh, LoopMoveResult::Refused(LoopMoveRefusal::Stale));
        let resumed = owner.move_loop_item(actor, command, request).await;
        results.push((id, resumed));
    }
    for (id, resumed) in results {
        assert_eq!(
            resumed,
            LoopMoveResult::Committed,
            "historical MOVE of display {id}"
        );
    }
    let bag = h
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await?
        .ok_or("backpack")?;
    let mut carried: Vec<_> = bag
        .entries
        .iter()
        .map(|entry| entry.item.item_instance_id)
        .collect();
    carried.sort_unstable();
    expected.sort_unstable();
    assert_eq!(carried, expected);
    let receipt_counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_map_item_mint_receipts), (SELECT count(*) FROM game_item_loop_move_receipts)")
        .fetch_one(&h.pool).await?;
    assert_eq!(receipt_counts, (4, 4));
    Ok(())
}

async fn wrath_wheel(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
) -> TestResult {
    use super::atelier_fixtures::fixture as atelier;
    use super::fixture::id;
    use crate::durability::character_wheel::{
        WheelChangeFacts, WheelChangeRequest, WheelCommitOutcome, WheelOccurrence, WheelRuleset,
        WheelSlots,
    };
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::ItemWeightLookup;
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
    use oteryn_protocol_oteryn::refusal_detail::split_refusal_detail;

    let premium = owner.premium_coordinator.ok_or("Premium coordinator")?;
    let session = admitted.game_session_id;
    owner.admit_premium(id(40), session);
    atelier::eligible(&h.pool).await?;
    let purchase = atelier::prepare(h).await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("promotion fence")?;
    let promoted = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_npc_purchase(
            &h.root,
            owner.character,
            owner.holder,
            fence,
            purchase,
            premium,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(
            promoted,
            crate::durability::npc_purchase::PurchaseOutcome::Committed(_)
        ),
        "{promoted:?}"
    );

    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native items")?;
    let weights = NativeItemWeights(native);
    let weight = |key: &str| -> TestResult<u64> {
        Ok(u64::from(
            weights
                .weight_of(&TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: "definition-r1".into(),
                })
                .map_err(debug)?,
        ))
    };
    let (bag, spear, coin) = (
        "oteryn:item.tibia.i2854",
        "oteryn:item.tibia.i3277",
        "oteryn:item.tibia.i3031",
    );
    let base =
        oteryn_gameplay_items::item_capacity::project_global_capacity_centi_oz(60, "sorcerer")
            .map_err(debug)?;
    let to_fill = base - 12_900 - weight(bag)?;
    let (spear_weight, coin_weight) = (weight(spear)?, weight(coin)?);
    let (spears, remainder) = (to_fill / spear_weight, to_fill % spear_weight);
    assert_eq!(remainder % coin_weight, 0);
    let mut entries = Vec::new();
    for (key, mut quantity) in [(spear, spears), (coin, remainder / coin_weight)] {
        while quantity != 0 {
            let stack = quantity.min(100);
            entries.push((key, i64::try_from(stack)?));
            quantity -= stack;
        }
    }
    replace_backpack(h, None).await?;
    super::item_look_fixtures::seed_main_backpack(h, &entries).await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("allocation fence")?;
    admitted.item_fence = Some(fence.item_fence());
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18, 66, 508])
            .ok_or("capacity capabilities")?;
    assert_eq!(
        owner
            .observe_own_capacity(admitted)
            .await
            .ok_or("capacity projection")?
            .free,
        12_900
    );
    let target = Position {
        x: 33078,
        y: 31170,
        floor: -8,
    };
    relocate(owner, admitted, target).await?;
    let payload = projected_target(owner, target, target, 2473)?;
    let denied = wire(owner, &mut admitted, 1, 2, payload.clone()).await?;
    assert_eq!(
        split_refusal_detail(&denied).map_err(debug)?.1,
        Some(oteryn_protocol_oteryn::refusal_detail::RefusalDetail::NotEnoughCapacity)
    );

    let data = WheelGemData::embedded().map_err(debug)?;
    let rules =
        Arc::new(WheelRuleset::from_catalogue(data.revision(), data.wheel()).map_err(debug)?);
    let mut points = [0; 36];
    points[14] = 3; // Sorcerer's 2 oz per point grants 6 oz at this allocation.
    let allocated = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_wheel(
            &h.root,
            owner.character,
            owner.holder,
            fence,
            WheelChangeRequest {
                occurrence: WheelOccurrence::from_bytes(id(188)).map_err(debug)?,
                expected_wheel_revision: 0,
                slots: WheelSlots::new(points).map_err(debug)?,
                vessels: None,
            },
            rules,
            WheelChangeFacts { at_temple: false },
            premium,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(allocated, WheelCommitOutcome::Committed(_)),
        "{allocated:?}"
    );
    admitted.item_fence = Some(
        owner
            .current_quest_fence(session)
            .await
            .map_err(debug)?
            .ok_or("reward fence")?
            .item_fence(),
    );
    assert_eq!(
        owner
            .observe_own_capacity(admitted)
            .await
            .ok_or("Wheel capacity projection")?
            .free,
        13_500
    );
    tokio::time::sleep(WORLD_ACTION_COOLDOWN).await;
    let claimed = wire(owner, &mut admitted, 2, 2, payload).await?;
    let (result, detail) = split_refusal_detail(&claimed).map_err(debug)?;
    assert_eq!(
        (decode_use_result(result).map_err(debug)?, detail),
        (UseDisposition::Committed, None)
    );
    assert_eq!(
        owner
            .observe_own_capacity(admitted)
            .await
            .ok_or("capacity after reward")?
            .free,
        500
    );
    Ok(())
}

async fn wrath_capacity(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::refusal_detail::{RefusalDetail, split_refusal_detail};
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18, 66])
            .ok_or("refusal detail capability")?;
    assert!(
        admitted
            .continuity
            .selected_capabilities
            .contains(oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,)
    );
    let mut results = Vec::new();
    for (count, detail) in [
        (0, RefusalDetail::NotEnoughRoom),
        (20, RefusalDetail::NotEnoughRoom),
        (12, RefusalDetail::NotEnoughCapacity),
    ] {
        if count == 20 {
            super::item_look_fixtures::seed_main_backpack(h, &[("oteryn:item.tibia.i3031", 1); 20])
                .await?;
        }
        if count == 12 {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM game_item_container_entries WHERE placement_ordinal > 12")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i11686' WHERE item_instance_id IN (SELECT item_instance_id FROM game_item_container_entries)").execute(&mut *tx).await?;
            tx.commit().await?;
        }
        for (x, y, uid, _) in CHOICES {
            let target = Position { x, y, floor: -8 };
            let at = target;
            relocate(owner, admitted, at).await?;
            let payload = projected_target(owner, at, target, 2473)?;
            tokio::time::sleep(WORLD_ACTION_COOLDOWN).await;
            let id = admitted.continuity.next_command_id;
            let response = wire(owner, &mut admitted, id, 2, payload).await?;
            let (result, actual) = split_refusal_detail(&response).map_err(debug)?;
            results.push((uid, count, decode_use_result(result).map_err(debug)?, actual, detail));
        }
    }
    for (uid, count, result, actual, expected) in results {
        assert_eq!(actual, Some(expected), "chest {uid}, {count} carried items");
        let expected_disposition = match expected {
            RefusalDetail::NotEnoughRoom => UseDisposition::NoRoom,
            _ => UseDisposition::Rejected,
        };
        assert_eq!(result, expected_disposition, "chest {uid}, {count} carried items");
    }
    let ledger: (i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_reward_claims), (SELECT count(*) FROM game_reward_claim_mint_reservations), (SELECT count(*) FROM game_reward_claim_mint_receipts)",
    ).fetch_one(&h.pool).await?;
    assert_eq!(ledger, (0, 0, 0));
    Ok(())
}

async fn public_gate(
    h: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut first: AdmittedSession,
    x: u16,
) -> TestResult {
    use super::appearance_change_postgres_fixtures::observer_visibility_tests as observer;
    use super::fixture::id;
    h.seed_pvp_target().await?;
    observer::admit(h).await?;
    let second_session = crate::foundation::GameSessionId::decode(&id(110)).map_err(debug)?;
    let second_at = Position {
        x: x + 1,
        y: 31117,
        floor: -7,
    };
    let second_actor = {
        let mut runtime = deps.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(second_session)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, local(second_at))
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                second_session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(101)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        actor
    };
    deps.initialize_player(owner, second_actor, second_session)
        .await;
    let mut second = AdmittedSession {
        game_session_id: second_session,
        runtime_actor: Some(second_actor),
        controller: Some(crate::gameplay_transport::connection::ControllerBinding {
            transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[8; 16])
                .map_err(debug)?,
            account_id: id(100),
        }),
        continuity: crate::gameplay_transport::connection::SessionContinuity::FRESH,
        item_fence: Some(
            owner
                .current_quest_fence(second_session)
                .await
                .map_err(|()| "second fence")?
                .ok_or("second fence")?
                .item_fence(),
        ),
        ..first
    };
    owner.admit_progression_session(&mut second).await;
    let target = Position {
        x,
        y: 31118,
        floor: -7,
    };
    let at = Position { y: 31117, ..target };
    let first_target = projected_target(owner, at, target, 8618)?;
    let second_target = projected_target(owner, second_at, target, 8618)?;
    assert_eq!(first_target, second_target);
    let used = wire(owner, &mut first, 1, 2, first_target.clone()).await?;
    assert_eq!(
        decode_use_result(&used).map_err(debug)?,
        UseDisposition::Committed
    );
    let used = wire(owner, &mut second, 1, 2, second_target).await?;
    assert_eq!(
        decode_use_result(&used).map_err(debug)?,
        UseDisposition::Committed
    );
    let destination = if x == 32775 {
        Position {
            x: 32773,
            y: 31116,
            floor: -7,
        }
    } else {
        Position {
            x: 32780,
            y: 31115,
            floor: -7,
        }
    };
    let runtime = owner.runtime.lock().await;
    for actor in [first.runtime_actor.ok_or("first actor")?, second_actor] {
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            local(destination)
        );
    }
    assert_eq!(projected_target(owner, at, target, 8618)?, first_target);
    Ok(())
}

async fn public_gate_facing(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
) -> TestResult {
    use crate::foundation::MovementFacing;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let at = Position {
        x: 32775,
        y: 31117,
        floor: -7,
    };
    let target = Position { y: 31118, ..at };
    {
        let mut runtime = owner.runtime.lock().await;
        let before = runtime.read_actor_position(actor).map_err(debug)?;
        assert_eq!(before.position(), local(at));
        runtime
            .borrow_movement_position()
            .commit_turn(before, MovementFacing::East)
            .map_err(debug)?;
    }
    let payload = projected_target(owner, at, target, 8618)?;
    let used = wire(owner, &mut admitted, 1, 2, payload).await?;
    assert_eq!(
        decode_use_result(&used).map_err(debug)?,
        UseDisposition::Committed
    );
    let arrived = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        arrived.position(),
        local(Position {
            x: 32773,
            y: 31116,
            floor: -7
        })
    );
    assert_eq!(arrived.facing(), Some(MovementFacing::West));
    Ok(())
}
