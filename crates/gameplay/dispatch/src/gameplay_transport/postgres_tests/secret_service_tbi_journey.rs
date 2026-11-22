//! The chosen TBI route, using the embedded binding and native World selectors.
use super::fixture::{Harness, debug, id, run};
use super::quest_world_callbacks_fixtures::{input_item, local, wire};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::authored_bindings::{BoundQuest, Trigger};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.secret_service_quest";
const PHASE: &str = "oteryn:quest-progress/secret-service/tbi-phase";
const CHESTER: Position = Position {
    x: 32349,
    y: 32183,
    floor: -6,
};
const START: Position = Position {
    x: 32348,
    ..CHESTER
};
const LOOT: [(&str, Position, u16); 3] = [
    (
        "lottery_ticket",
        Position {
            x: 32872,
            y: 31958,
            floor: -11,
        },
        2469,
    ),
    (
        "poem_scroll",
        Position {
            x: 32643,
            y: 32733,
            floor: -7,
        },
        2472,
    ),
    (
        "intelligence_reports",
        Position {
            x: 32773,
            y: 31582,
            floor: -11,
        },
        4240,
    ),
];

#[test]
fn secret_service_tbi_full_journey_earns_final_package() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    super::kill_reward_e2e_fixtures::scenario_thread::run("secret-service-tbi", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "secret_service_tbi", true, false)
                .await?;
            let result = journey(&h).await;
            let cleanup = h.cleanup().await;
            result.and(cleanup)
        })
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED secret_service_tbi_full_journey_earns_final_package {:?}",
        started.elapsed()
    );
    Ok(())
}

async fn journey(h: &Harness) -> TestResult {
    let initial = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let catalogue = Arc::new(initial.catalogue().clone());
    let binding = catalogue
        .authored_bindings()
        .and_then(|bindings| bindings.quests.iter().find(|quest| quest.quest == QUEST));
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, active) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("active content generation")?;
    let world = world(
        generation
            .native_gameplay()
            .ok_or("native item catalogue")?,
    )?;
    assert!(world.boss_spawns().iter().any(|(_, species, at, _)| {
        species == "oteryn:creature.black_knight" && *at == (32874, 31946, 11)
    }));
    let cells = world
        .movement_cells(active.qualified_room().movement_cells())
        .map_err(debug)?;
    let pin = ChannelContentPin::from_activation(
        world_id,
        active.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        active.frame_binding().digest(),
        world.map_revision_digest(),
        (i32::from(START.x), i32::from(START.y), 6),
    );
    let mut deps = Dependencies::with_content_and_capacity(
        pin,
        crate::content::qualify_native_entry_room(world_id).map_err(debug)?,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
        64,
    )?;
    let room = active
        .qualified_room()
        .bundle_view(
            cells.clone(),
            active.frame_binding().digest(),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: i32::from(START.x),
                y: i32::from(START.y),
                floor: 6,
            },
        )
        .ok_or("qualified TBI World room")?;
    deps.combat_room = Some(room);
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    owner.quest_log_content = Some(
        crate::gameplay_transport::quest_log::QuestLogLines::new(
            oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&initial)?.content,
        )
        .ok_or("TBI log bounds")?,
    );
    let (consumer, clock, activation, _) =
        crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium =
        crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );
    owner.premium_coordinator = Some(&premium);
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            &offered,
            &[3, 4, 6, 11, 12, 13, 14, 15, 16, 17, 18, 25],
        )
        .ok_or("TBI journey capabilities")?;
    position(&owner, &admitted, START).await?;
    deps.initialize(&owner).await;
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_premium(id(40), admitted.game_session_id);
    owner.admit_quest_session(&admitted).await;
    super::qengine_outfit_stories_fixtures::earn_level(&owner, &admitted, &authority, 50).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("TBI quest fence")?
        .item_fence();
    admitted.item_fence = Some(fence);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(active.frame_binding().digest().as_slice()).execute(&h.pool).await?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence,
            generation.identity().server_artifact_digest(),
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence,
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(
                generation
                    .native_gameplay()
                    .ok_or("native item catalogue")?,
            ),
            &crate::durability::starter_grant::StarterTemplate::main(),
        )
        .await
        .map_err(debug)?;
    for (n, item) in [(1, 5467), (2, 3304)] {
        input_item(
            &owner,
            fence,
            &Offering {
                position: START,
                item: item_key(item),
                count: 1,
            },
            n,
            true,
        )
        .await?;
    }
    let [chester] = super::classic_quests_fixtures::install_positions(
        &owner,
        &admitted,
        ["oteryn:npc.chester_kahs"],
        [CHESTER],
    )
    .await?;
    assert_phase(h, 0).await?;
    let binding = binding.ok_or("Chester cannot recruit: missing production TBI binding")?;
    let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
        world.base().as_ref(),
        &|_| true,
    )?;
    for (suffix, _, _) in LOOT {
        assert!(report.served.iter().any(|claim| claim.key.as_str()
            == format!("oteryn:reward-claim.quest.u8_1.secret_service.{suffix}")));
    }
    exercise(&mut owner, &mut admitted, binding, chester, h).await
}

async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    binding: &BoundQuest,
    chester: EntityRef,
    h: &Harness,
) -> TestResult {
    let mut held = [1, 1, 0, 0, 0, 0, 0, 0, 0];
    let mut phase = 0;
    let mut offer = None;
    let mut terminal = None;
    for stage in &binding.stages {
        match &stage.trigger {
            Trigger::NpcDialogue { callback, .. } => {
                if callback.next_topic != 0 {
                    offer = Some(callback.keywords[0].as_str());
                    continue;
                }
                position(owner, admitted, START).await?;
                let words = std::iter::once("hi")
                    .chain(offer.take())
                    .chain(std::iter::once(callback.keywords[0].as_str()))
                    .collect();
                let (outcome, _, _) = super::qengine_rewards_fixtures::wire_conversation(
                    owner, admitted, chester, callback, false, words,
                )
                .await?;
                assert_eq!(outcome, NpcIntentDisposition::Ok, "{}", stage.key);
                for amount in &callback.take {
                    change(&mut held, &amount.item, -i64::from(amount.count))?;
                }
                if let Some(reward) = &callback.give {
                    change(&mut held, &reward.item, i64::from(reward.count))?;
                }
                phase += 1;
                if stage.completes {
                    terminal = Some(callback);
                }
            }
            Trigger::MapUse { callback } => {
                let at = callback.selector.position.ok_or("TBI map position")?;
                position(owner, admitted, adjacent(owner, at)?).await?;
                if let Some(tool) = &callback.item_on_target {
                    let target = target(
                        owner,
                        at,
                        callback.selector.appearance.ok_or("tool target")?,
                    )?;
                    use_item(owner, admitted, &tool.item, Some(target)).await?;
                    if !tool.reusable {
                        change(&mut held, &tool.item, -1)?;
                    }
                    phase += 1;
                } else {
                    let target = target(owner, at, 2772)?;
                    confirmed_use(owner, admitted, encode_use_intent(&target).map_err(debug)?)
                        .await?;
                }
            }
            Trigger::CarriedUse { callback } => {
                use_item(
                    owner,
                    admitted,
                    callback.carried_item.as_deref().ok_or("disguise")?,
                    None,
                )
                .await?;
                change(&mut held, &item_key(397), -1)?;
                phase += 1;
            }
            Trigger::Collect { .. } => {
                chest(owner, admitted, LOOT[0]).await?;
                change(&mut held, &item_key(5956), 1)?;
                phase += 1;
            }
            Trigger::StepIn { callback } => {
                let at = callback
                    .selector
                    .position
                    .ok_or("reconnaissance position")?;
                step_to(owner, admitted, at).await?;
                phase += 1;
                assert_progress(owner, admitted, h, phase, &held).await?;
                let (chest_index, item) = if stage.key == "s4" {
                    (1, 5952)
                } else {
                    (2, 348)
                };
                chest(owner, admitted, LOOT[chest_index]).await?;
                change(&mut held, &item_key(item), 1)?;
            }
            Trigger::Kill { counter } => {
                let (x, y, floor) = if stage.key == "s8" {
                    (32873, 31946, -11)
                } else {
                    (32570, 31859, -14)
                };
                position(owner, admitted, Position { x, y, floor }).await?;
                kill(owner, admitted, &counter.creature).await?;
                phase += 1;
                assert_progress(owner, admitted, h, phase, &held).await?;
                if stage.key == "technomancer" {
                    loot_beard(owner, admitted, h).await?;
                    change(&mut held, &item_key(396), 1)?;
                }
            }
            _ => return Err(format!("unexpected TBI stage {}", stage.key).into()),
        }
        assert_progress(owner, admitted, h, phase, &held).await?;
    }
    assert_eq!(phase, 24);
    assert_eq!(held[8], 1);
    let terminal = terminal.ok_or("terminal Chester hand-in")?;
    // Every wire_conversation replays its last command. This new connection
    // offers a fresh command after the reward has already been committed.
    let (refusal, _, _) = super::qengine_rewards_fixtures::wire_conversation(
        owner,
        admitted,
        chester,
        terminal,
        false,
        vec!["hi", "mission", "yes"],
    )
    .await?;
    assert_eq!(refusal, NpcIntentDisposition::Rejected);
    assert_progress(owner, admitted, h, phase, &held).await?;
    let completions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1 AND completes",
    )
    .bind(QUEST)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(completions, 1);
    let title: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/top_tbi_agent'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(title, 1);
    use oteryn_protocol_oteryn::quest_log::{
        QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query,
    };
    let query = encode_quest_log_query(&QuestLogQuery::ListOnly).map_err(debug)?;
    let log = decode_quest_log(&command(owner, admitted, 22, query).await?).map_err(debug)?;
    let Some(QuestLogView::List(entries)) = log.view else {
        return Err("TBI quest list unavailable".into());
    };
    let journal = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let journal = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&journal)?;
    let quest_index = u32::try_from(
        journal
            .content
            .active()
            .quests()
            .iter()
            .position(|quest| quest.key == QUEST)
            .ok_or("TBI journal index")?,
    )?;
    assert!(
        entries
            .iter()
            .any(|entry| entry.quest_index == quest_index && entry.completed)
    );
    Ok(())
}

fn item_key(item: u16) -> String {
    format!("oteryn:item.tibia.i{item}")
}

const ITEMS: [u16; 9] = [5467, 3304, 5956, 5952, 348, 349, 397, 396, 897];

fn change(held: &mut [i64; 9], key: &str, delta: i64) -> TestResult {
    let index = ITEMS
        .iter()
        .position(|item| item_key(*item) == key)
        .ok_or("unexpected TBI item")?;
    held[index] += delta;
    Ok(())
}

async fn assert_phase(h: &Harness, expected: i64) -> TestResult {
    let phase: Option<i64> = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 AND track_key=$2",
    )
    .bind(QUEST)
    .bind(PHASE)
    .fetch_optional(&h.pool)
    .await?;
    assert_eq!(phase.unwrap_or_default(), expected);
    Ok(())
}

async fn assert_progress(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    phase: i64,
    expected: &[i64; 9],
) -> TestResult {
    assert_phase(h, phase).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("current inventory fence")?
        .item_fence();
    let inventory = h
        .root
        .read_item_use_inventory(owner.character, &h.node, fence)
        .await
        .map_err(debug)?;
    let mut actual = [0; 9];
    for entry in inventory
        .backpack
        .iter()
        .flat_map(|bag| &bag.entries)
        .chain(inventory.nested.iter().map(|(_, entry)| entry))
    {
        if let Some(index) = ITEMS
            .iter()
            .position(|item| item_key(*item) == entry.item.definition.production_key)
        {
            actual[index] += i64::from(entry.item.quantity);
        }
    }
    assert_eq!(&actual, expected, "TBI custody after phase {phase}");
    Ok(())
}

async fn command(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    kind: u32,
    payload: Vec<u8>,
) -> TestResult<Vec<u8>> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("pacing clock")?;
    let next = admitted.continuity.next_command_id;
    wire(owner, admitted, next, kind, payload).await
}

async fn confirmed_use(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    payload: Vec<u8>,
) -> TestResult {
    assert_eq!(
        decode_use_result(&command(owner, admitted, 2, payload).await?).map_err(debug)?,
        UseDisposition::Committed
    );
    Ok(())
}

async fn use_item(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    item: &str,
    target: Option<WorldObjectTarget>,
) -> TestResult {
    let definition = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", item, "definition-r1"))
        .ok_or("qualified TBI item")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: target.map(MapUseTarget::Object),
        use_with_position: None,
    })
    .map_err(debug)?;
    confirmed_use(owner, admitted, payload).await
}

async fn chest(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    (_, at, appearance): (&str, Position, u16),
) -> TestResult {
    position(owner, admitted, adjacent(owner, at)?).await?;
    let payload = encode_use_intent(&target(owner, at, appearance)?).map_err(debug)?;
    confirmed_use(owner, admitted, payload).await
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: Position,
) -> TestResult {
    let mut runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let Ok(expected) = runtime.read_actor_position(actor) else {
        runtime
            .initialize_pinned_test_position(actor, local(at))
            .map_err(debug)?;
        return Ok(());
    };
    if expected.position() != local(at) {
        // Fixture travel positions isolate the mission interactions from cross-map travel.
        let map = owner.bundle_map().ok_or("TBI map")?;
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            owner.movement_cells(),
            actor,
            admitted.game_session_id,
            expected,
            local(at),
            false,
            &closed,
        )
        .map_err(debug)?;
        runtime
            .commit_quest_relocation(vec![proof], || true)
            .map_err(debug)?;
    }
    Ok(())
}

fn adjacent(owner: &ComposedFreshAdmission<'_, '_, '_>, target: Position) -> TestResult<Position> {
    let map = owner.bundle_map().ok_or("TBI map")?;
    for (x, y) in [
        (target.x - 1, target.y),
        (target.x, target.y - 1),
        (target.x + 1, target.y),
        (target.x, target.y + 1),
    ] {
        if map
            .overlay
            .base()
            .tile(x, y, target.floor)
            .is_some_and(|tile| tile.ground().is_some())
        {
            return Ok(Position { x, y, ..target });
        }
    }
    Err(format!("no adjacent source ground at {target:?}").into())
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    at: Position,
    appearance: u16,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("TBI map")?;
    let tile = TilePos {
        x: at.x,
        y: at.y,
        floor: at.floor,
    };
    let ordinal = (0..64)
        .find(|ordinal| {
            map.facts
                .base_entry(tile, *ordinal, 0)
                .is_some_and(|entry| entry.appearance_id == appearance)
        })
        .ok_or("TBI source object")?;
    let key = crate::map::view::placement_key(tile, ordinal).ok_or("TBI target")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn step_to(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    at: Position,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        StepDirection, StepDisposition, decode_step_result, encode_step_intent,
    };
    let from = adjacent(owner, at)?;
    let direction = match (
        i32::from(at.x) - i32::from(from.x),
        i32::from(at.y) - i32::from(from.y),
    ) {
        (1, 0) => StepDirection::East,
        (-1, 0) => StepDirection::West,
        (0, 1) => StepDirection::South,
        (0, -1) => StepDirection::North,
        _ => return Err("invalid recon approach".into()),
    };
    position(owner, admitted, from).await?;
    let reply = command(owner, admitted, 1, encode_step_intent(direction)).await?;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}

fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::bundle;
    use oteryn_world_bundle_compiler::{compile, project, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let regions = [
        (126, 125),
        (128, 125),
        (128, 124),
        (127, 127),
        (128, 123),
        (125, 122),
        (125, 121),
        (127, 124),
    ];
    let project = source::project_regions(&root, &regions)?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity: identity.clone(),
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: project::draft_areas(&project.index)?,
            families: &project.families,
        },
        &registry,
    )?;
    let pins = crate::map::BundlePins {
        digest: compiled.digest,
        project_format_version: identity.project_format_version,
        world_schema_version: identity.world_schema_version,
        content_revision: identity.content_revision,
        production: false,
    };
    let visited = bundle::visit(&compiled.bytes, |_, _| Ok(()))?;
    assert!(visited.npcs.npcs.iter().any(|npc| {
        npc.key == "oteryn:npc.chester_kahs"
            && npc.placements.iter().any(|placement| {
                (placement.x, placement.y, placement.floor)
                    == (CHESTER.x, CHESTER.y, -CHESTER.floor)
            })
    }));
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &compiled.bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: START.x,
                y: START.y,
                floor: START.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)
}

async fn kill(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    creature: &str,
) -> TestResult {
    use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
    use crate::gameplay_transport::kill_reward::{KillSink, record_creature_kill};
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    use oteryn_protocol_oteryn::attack::{
        AttackIntentDisposition, decode_attack_intent_result, encode_attack_target_intent,
    };
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native policies")?;
    let (target, entity) = {
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        if creature == "oteryn:creature.black_knight" {
            let health = runtime
                .companion_policy(creature)
                .map_err(debug)?
                .maximum_health;
            runtime
                .commit_quest_creature_spawn(&[(
                    creature,
                    health,
                    local(Position {
                        x: 32874,
                        y: 31946,
                        floor: -11,
                    }),
                )])
                .map_err(debug)?;
        }
        let boss = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|candidate| {
                runtime.matches_live_creature_identity(candidate.actor, creature.as_bytes())
            })
            .ok_or("source boss actor")?;
        (
            boss.actor,
            EntityRef {
                identity: boss.actor.placement_identity(),
                generation: boss.generation,
            },
        )
    };
    let payload = encode_attack_target_intent(Some(&entity)).map_err(debug)?;
    let reply = command(owner, admitted, 11, payload).await?;
    assert_eq!(
        decode_attack_intent_result(&reply).map_err(debug)?,
        AttackIntentDisposition::Ok
    );
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    // Supply a deterministic lethal hit after the wire target has been accepted.
    {
        let mut runtime = owner.runtime.lock().await;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(debug)?;
        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
        let command = CommandRef::new(
            session,
            CommandId::new(88000 + admitted.continuity.next_command_id).map_err(debug)?,
        );
        runtime
            .borrow_exact_actor_commit_at(observed.get() / 1000)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                command,
                0,
                OwnerDamageCommand {
                    target: creature.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"secret-service-tbi-journey",
                    damage: health,
                },
            )
            .map_err(debug)?;
        record_creature_kill(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: &queue,
                rewards: native.reward_table(),
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session,
                actor,
            },
            observed.get() / 1000,
            Some(observed),
        );
    }
    super::qengine_kill_stories_fixtures::drain_deaths(owner, admitted, &[]).await
}

async fn loot_beard(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    use crate::gameplay_transport::item_move::{ItemMoveStep, item_move};
    use crate::gameplay_transport::item_view::{ItemKey, ItemViewError, SessionItemView};
    use oteryn_protocol_oteryn::item_view::{
        ItemMoveDestination, ItemMoveIntent, ItemMoveOutcome, ItemMoveSelection,
        encode_item_move_intent_for,
    };
    let item: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1 LIMIT 1",
    ).bind(item_key(396)).fetch_one(&h.pool).await?;
    let key = ItemKey::Instance(item.try_into().map_err(|_| "beard instance id")?);
    let mut view = SessionItemView::resume(admitted.continuity.item_view).with_map_view();
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let scope = owner.runtime.lock().await.binding().runtime_scope();
    let bodies = h.root.read_item_loop_ground_for_scope(scope).await?;
    let mut opened = false;
    for body in bodies.into_iter().filter(|body| body.native_corpse) {
        let corpse = ItemKey::Instance(body.item.item_instance_id);
        let Some(observation) = owner.observe_item_target(actor, corpse).await else {
            continue;
        };
        if matches!(&observation.target, crate::gameplay_transport::item_view::UseItemTarget::Corpse { contents, .. }
            if contents.iter().any(|entry| entry.key == key))
        {
            let (_, delta) = view.open(corpse, observation).map_err(debug)?;
            assert!(delta.is_some());
            opened = true;
            break;
        }
    }
    assert!(opened, "the beard must be a reachable native corpse entry");
    let handle = view
        .map_view(&[key], |handles| {
            handles.handle(&key).ok_or(ItemViewError::Encode)
        })
        .map_err(debug)?;
    let selection = ItemMoveSelection::default();
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: handle,
            destination: ItemMoveDestination::MainBackpack,
        },
        selection,
    )
    .map_err(debug)?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("loot fence")?
        .item_fence();
    let result = item_move(
        owner,
        admitted.runtime_actor.ok_or("actor")?,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 90000 + admitted.continuity.next_command_id,
            item_fence: Some(fence),
        },
        &payload,
        selection,
        false,
        &mut view,
    )
    .await;
    assert!(
        matches!(result, ItemMoveStep::Result(ItemMoveOutcome::Moved, _)),
        "{result:?}"
    );
    Ok(())
}
