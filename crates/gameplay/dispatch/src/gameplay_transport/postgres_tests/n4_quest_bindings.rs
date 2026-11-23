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

#[test]
fn n4_yalahar_mechanism_checks_durable_progress_without_rewriting_it() -> TestResult {
    scenario("n4_yalahar", false)
}

#[test]
fn n4_wrath_choice_claims_exactly_once_across_chests_and_owner_reload() -> TestResult {
    scenario("n4_wrath_choice", true)
}

fn scenario(name: &'static str, wrath: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run(name, move || {
        run(async |admin| {
            let harness = Harness::create_with_admission_seed(admin, name, false, false).await?;
            let result = exercise(&harness, wrath).await;
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

async fn exercise(h: &Harness, wrath: bool) -> TestResult {
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
    let crop = if wrath {
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
    let world = native_crop(
        generation.native_gameplay().ok_or("native item facts")?,
        crop,
    )?;
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
    let deps = Dependencies::with_content_and_capacity(
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
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = room.movement_cells().into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
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
    if wrath {
        return wrath_chest(h, &deps, &authority, &owner, admitted).await;
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
    let fence = admitted.item_fence.ok_or("item fence")?;
    let session = admitted.game_session_id;
    let map = owner.bundle_map().ok_or("native map")?;
    let selected = super::questpc7_hoist_story_fixtures::target(
        owner,
        Position {
            x: 33078,
            y: 31170,
            floor: -8,
        },
        2473,
    )?;
    let alternate = super::questpc7_hoist_story_fixtures::target(
        owner,
        Position {
            x: 33080,
            y: 31170,
            floor: -8,
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
        UseDisposition::Rejected
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
    let denied = wire(owner, &mut admitted, 3, 2, alternate).await?;
    assert_eq!(
        decode_use_result(&denied).map_err(debug)?,
        UseDisposition::NothingToUse
    );
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

    // Rebuild the server owner and use the durable receipt with a fresh command cache.
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
    assert_eq!(
        super::quest_callbacks_fixtures::wire_uses(&reloaded, replay, &[1, 2], &target).await?,
        [UseDisposition::NothingToUse, UseDisposition::Committed]
    );
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
