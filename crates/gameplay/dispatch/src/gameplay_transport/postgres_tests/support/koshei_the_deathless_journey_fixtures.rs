//! The selected Koshei recipe against the embedded catalogue and retained Darashia tiles.
use super::fixture::{Harness, debug, id};
use super::m01b_explore_fixtures::koshei::{POINTS, TRACK, relocate, step};
use super::quest_world_callbacks_fixtures::{input_item, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_gameplay_world::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    self as use_wire, MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) const NAME: &str = "koshei_the_deathless_full_journey";
const QUEST: &str = "oteryn:quest.koshei_the_deathless_quest";
const ENTRY: &str = "oteryn:quest-callback/koshei/tomb-entry";
const FORGE: &str = "oteryn:quest-progress/qnpcfx/koshei/amulet";
const FORGE_STARTED_AT: &str = "oteryn:quest-progress/qnpcfx/koshei/amulet_started_at";
const AGE_FORGE: &str = "oteryn:quest-transition/koshei/fixture-age-forge-order";
const ENTERED: &str = "oteryn:quest-progress/koshei/tomb-entered";
const KILLED: &str = "oteryn:quest-progress/koshei/tomb-defeated";
const SEALED: &str = "oteryn:quest-progress/quest/u8_1/koshei_the_deathless/reward_door";
const CREATURE: &str = "oteryn:creature.koshei_the_deathless";
const CYCLOPS: &str = "oteryn:npc.a_sweaty_cyclops";

pub(super) const SWITCH: Position = at(33281, 32447, -8);
const NPC: Position = at(32699, 31674, -3);
const DOOR: Position = at(33269, 32446, -12);
const LEGS: Position = at(33261, 32444, -12);
const PLATINUM: Position = at(33261, 32448, -12);
const ITEMS: [i32; 7] = [7530, 7528, 7531, 7529, 7532, 645, 3035];

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

pub(super) fn callback() -> TestResult<Callback> {
    use oteryn_gameplay_quests::quest::{loader, world_callbacks};
    let loaded = loader::load_embedded_quest_state("content-1").map_err(debug)?;
    world_callbacks::load_for(loaded.catalogue())?
        .into_iter()
        .find(|callback| callback.key == ENTRY)
        .ok_or_else(|| "production Koshei tomb encounter is missing".into())
}

/// Use the production compiler and native registry for each of the four source regions.
/// Travel between disconnected sections is arranged separately from the quest commands.
pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::{
        BundlePins,
        boot::{BootPins, boot},
        overlay::{TilePos, map_revision},
    };
    use oteryn_world_bundle_compiler::{bundle, compile, project, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut project =
        source::project_regions(&root, &[(127, 123), (129, 126), (129, 127), (130, 126)])?;
    project
        .families
        .spawns
        .retain(|source| source.points.iter().any(|point| point.creature == CREATURE));
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let mut pins = BundlePins {
        digest: [0; 32],
        project_format_version: identity.project_format_version.clone(),
        world_schema_version: identity.world_schema_version.clone(),
        content_revision: identity.content_revision.clone(),
        production: false,
    };
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: project::draft_areas(&project.index)?,
            families: &project.families,
        },
        &registry,
    )?;
    assert!(
        project
            .families
            .npc_placements
            .iter()
            .any(|row| { row.npc == CYCLOPS && row.cell == (NPC.x, NPC.y, 3) }),
        "the forge must be a production NPC placement"
    );
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    boot(
        &compiled.bytes,
        &BootPins {
            map_revision: map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: SWITCH.x - 1,
                y: SWITCH.y,
                floor: SWITCH.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |key| crate::map::boot::native_item_definition(Some(native), key),
    )
    .map_err(debug)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct State {
    mission: [i64; 5],
    items: [i64; ITEMS.len()],
    completed: bool,
}
impl State {
    async fn read(h: &Harness) -> TestResult<Self> {
        let mission = sqlx::query_as::<_, (i64, i64, i64, i64, i64)>(
            "SELECT coalesce(max(value) FILTER(WHERE track_key=$1),0), \
             coalesce(max(value) FILTER(WHERE track_key=$2),0), \
             coalesce(max(value) FILTER(WHERE track_key=$3),0), \
             coalesce(max(value) FILTER(WHERE track_key=$4),0), \
             greatest(max(value) FILTER(WHERE track_key=$5),0) \
             FROM game_character_quest_tracks WHERE character_id=encode($6,'hex')::uuid",
        )
        .bind(TRACK)
        .bind(FORGE)
        .bind(ENTERED)
        .bind(KILLED)
        .bind(SEALED)
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        let mut items = [0; ITEMS.len()];
        for (index, item) in ITEMS.iter().enumerate() {
            items[index] = sqlx::query_scalar(
                "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances \
                 WHERE lifecycle=1 AND definition_production_key=$1",
            )
            .bind(format!("oteryn:item.tibia.i{item}"))
            .fetch_one(&h.pool)
            .await?;
        }
        let completed = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE \
             character_id=encode($1,'hex')::uuid AND quest_key=$2 AND completed_character_revision IS NOT NULL)",
        ).bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
        Ok(Self {
            mission: [mission.0, mission.1, mission.2, mission.3, mission.4],
            items,
            completed,
        })
    }
}

fn target(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    p: Position,
    ordinal: u8,
) -> TestResult<WorldObjectTarget> {
    let map = owner.bundle_map().ok_or("Koshei map")?;
    let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, ordinal)
        .ok_or("Koshei placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    Ok(WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    })
}

async fn use_object(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    object: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("world USE pacing")?;
    let payload = use_wire::encode_use_intent(object).map_err(debug)?;
    let reply = wire(
        owner,
        admitted,
        command,
        use_wire::COMMAND_TYPE_USE_INTENT,
        payload,
    )
    .await?;
    use_wire::decode_use_result(&reply)
        .map_err(debug)
}

async fn move_to(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    p: Position,
) -> TestResult {
    relocate(
        owner,
        admitted.runtime_actor.ok_or("player")?,
        admitted.game_session_id,
        p,
    )
    .await
}

async fn use_next(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    object: &WorldObjectTarget,
) -> TestResult<UseDisposition> {
    let command = admitted.continuity.next_command_id;
    use_object(owner, admitted, command, object).await
}

async fn commission(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    expected: &mut State,
) -> TestResult {
    use oteryn_gameplay_quests::quest::reward_callbacks;
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    let callbacks =
        reward_callbacks::load_for(owner.quest_catalogue.as_deref().ok_or("quest catalogue")?)?;
    let start = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/qnpcfx-koshei-start")
        .ok_or("forge start")?;
    let collect = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/qnpcfx-koshei-collect")
        .ok_or("forge collect")?;
    move_to(owner, admitted, at(NPC.x + 1, NPC.y, NPC.floor)).await?;
    let [npc] =
        super::classic_quests_fixtures::install_positions(owner, admitted, [CYCLOPS], [NPC])
            .await?;
    let (paid, _, _) =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, start, false).await?;
    assert_eq!(paid, NpcIntentDisposition::Ok);
    expected.items = [0; ITEMS.len()];
    expected.mission[1] = 1;
    assert_eq!(State::read(h).await?, *expected);
    let (early, _, _) =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, collect, false).await?;
    assert_eq!(early, NpcIntentDisposition::Rejected);
    assert_eq!(State::read(h).await?, *expected);
    // Model the donor's 24-hour wait through a new quest revision. The reward
    // and its deferred SQL guard both continue to use PostgreSQL time.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("forge timing fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition_with_admission(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: AGE_FORGE.into(),
                cause: crate::durability::quest_state::QuestCause::Command(
                    crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(51_001).map_err(debug)?,
                    ),
                ),
            },
            std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("quest catalogue")?),
            &owner.premium_coordinator,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(
        outcome,
        crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
    ));
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let (granted, _, _) =
        super::qengine_rewards_fixtures::wire_reward(owner, admitted, npc, collect, false).await?;
    assert_eq!(granted, NpcIntentDisposition::Ok);
    expected.items[4] = 1;
    expected.mission[1] = 2;
    assert_eq!(State::read(h).await?, *expected);
    Ok(())
}

async fn seal(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<[u8; 16]> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    let body = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .find(|corpse| corpse.item.production_key == "oteryn:item.tibia.i7535")
        .ok_or("the committed encounter kill must produce the fresh Koshei corpse")?;
    let index = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("item index")?;
    let amulet = index
        .definition_ref("Item", "oteryn:item.tibia.i7532", "definition-r1")
        .ok_or("amulet definition ref")?;
    let command = admitted.continuity.next_command_id;
    let generation = admitted.continuity.connection_generation;
    let (mut server, client) = tokio::io::duplex(1 << 20);
    let current = *admitted;
    let (end, result) = tokio::join!(
        async move { serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await },
        seal_client(client, amulet, body.position, generation, command),
    );
    match end {
        ConnectionEnd::AdmittedThenDisconnected(next) => *admitted = next,
        other => return Err(format!("seal connection ended: {other:?}").into()),
    }
    assert_eq!(result?, UseDisposition::Committed);
    Ok(body.item.item_instance_id)
}

async fn seal_client(
    mut client: tokio::io::DuplexStream,
    amulet: std::num::NonZeroU32,
    body: crate::foundation::MovementLocalPosition,
    generation: u64,
    command: u64,
) -> TestResult<UseDisposition> {
    use oteryn_protocol_oteryn::{self as wire, MessageType, item_view, world_map};
    use tokio::io::AsyncWriteExt;
    let (mut source, mut target) = (None, None);
    loop {
        let raw = crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
        let frame = wire::decode_wire_envelope(&raw)?;
        match frame.message_type() {
            MessageType::SnapshotChunk => {
                let (_, domains) = wire::decode_snapshot_chunk(frame.payload())?;
                for domain in domains {
                    if domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory =
                            item_view::decode_character_inventory(domain.payload).map_err(debug)?;
                        source = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == amulet)
                            .map(|entry| entry.handle);
                    } else if domain.domain_id == world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
                        let map =
                            world_map::decode_world_map_snapshot(domain.payload).map_err(debug)?;
                        target = map
                            .tiles
                            .iter()
                            .filter(|tile| {
                                (tile.position.x, tile.position.y, tile.position.floor)
                                    == (body.x, body.y, body.floor)
                            })
                            .flat_map(|tile| &tile.items)
                            .find_map(|item| match item.origin {
                                world_map::MapOrigin::Handle(handle)
                                    if item.appearance_id == 7535 =>
                                {
                                    Some(handle)
                                }
                                _ => None,
                            })
                            .or(target);
                    }
                }
            }
            MessageType::SnapshotCommit => break,
            _ => continue,
        }
    }
    let payload = use_wire::encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(source.ok_or("projected amulet handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Item(
            target.ok_or("projected fresh corpse handle")?,
        )),
        use_with_position: None,
    })
    .map_err(debug)?;
    let frame = wire::encode_client_command(
        generation,
        &wire::ClientCommandValue {
            command_id: command,
            command_type: use_wire::COMMAND_TYPE_USE_INTENT,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    loop {
        let raw = crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
        let frame = wire::decode_wire_envelope(&raw)?;
        if frame.message_type() == MessageType::CommandResult {
            let result = wire::decode_command_result(frame.payload())?;
            assert_eq!(result.command_id, command);
            client.shutdown().await?;
            return use_wire::decode_use_result(result.payload)
                .map_err(debug);
        }
    }
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    world: &crate::map::boot::BundleWorld,
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_gameplay_quests::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestTransition};
    use oteryn_protocol_oteryn::attack::ChaseMode;
    let mut capabilities = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    capabilities.extend([3, 4, 6, 11, 13, 15, 16, 17, 18, 25]);
    capabilities.sort_unstable();
    capabilities.dedup();
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &capabilities)
            .ok_or("journey capabilities")?;
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM now()))::bigint")
        .fetch_one(&h.pool)
        .await?;
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    owner.quest_catalogue = Some(std::sync::Arc::new(
        catalogue
            .with_fixture_transition(QuestTransition {
                world_change: None,
                key: AGE_FORGE.into(),
                quest: QUEST.into(),
                effects: vec![QuestEffect {
                    track: FORGE_STARTED_AT.into(),
                    from: QuestComparison::Ge(1),
                    effect: QuestEffectKind::Set(now - 86_400),
                }],
                completes: false,
                experience: None,
            })
            .map_err(debug)?,
    ));
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let mut expected = State::default();
    assert_eq!(State::read(h).await?, expected);

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("forge fence")?;
    // Arrange the initial wallet, with no quest item, on the new test character.
    input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: NPC,
            item: "oteryn:item.tibia.i3035".into(),
            count: 50,
        },
        51000,
        true,
    )
    .await?;
    expected.items[6] = 50;
    assert_eq!(State::read(h).await?, expected);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("combat fence")?;
    let build = owner
        .root
        .read_character_build_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    super::annihilator_combat::prepare_supplies(owner, admitted, &build, false).await?;
    let fragment_chests = [
        (at(33194, 32458, -7), 0),
        (at(33305, 32277, -10), 1),
        (at(33212, 32593, -7), 0),
        (at(33053, 32468, -11), 1),
    ];
    for (index, ((x, y, floor, direction), (position, ordinal))) in
        POINTS.into_iter().zip(fragment_chests).enumerate()
    {
        let start = if direction == StepDirection::West {
            x + 1
        } else {
            x - 1
        };
        move_to(owner, admitted, at(start, y, floor)).await?;
        let command = admitted.continuity.next_command_id;
        step(owner, admitted, command, direction).await?;
        expected.mission[0] += 1;
        assert_eq!(State::read(h).await?, expected);
        let chest = target(owner, position, ordinal)?;
        assert_eq!(
            use_next(owner, admitted, &chest).await?,
            UseDisposition::Committed
        );
        expected.items[index] = 1;
        assert_eq!(State::read(h).await?, expected);
    }

    commission(owner, admitted, h, &mut expected).await?;
    // The fragments and forge already existed before the tomb journey shipped.
    // Resume the same durable state under the exact pre-journey definition pin.
    let previous = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state_with_bindings(
        "content-1",
        &oteryn_gameplay_quests::quest::authored_bindings::Bindings::parse_documents(&[])?,
    )
    .map_err(debug)?;
    let pin = previous.catalogue().definition_hash(QUEST).ok_or("pre-journey Koshei pin")?;
    let mut saved = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *saved)
        .await?;
    let updated = sqlx::query(
        "UPDATE game_character_quest_states SET definition_hash=$2
         WHERE character_id=encode($1,'hex')::uuid AND quest_key=$3",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(pin.as_slice())
    .bind(QUEST)
    .execute(&mut *saved)
    .await?;
    assert_eq!(updated.rows_affected(), 1);
    saved.commit().await?;
    move_to(owner, admitted, at(DOOR.x + 1, DOOR.y, DOOR.floor)).await?;
    let door = target(owner, DOOR, 1)?;
    assert_eq!(
        use_next(owner, admitted, &door).await?,
        UseDisposition::Locked
    );
    assert_eq!(State::read(h).await?, expected);

    super::questpc7_native_deaths_fixtures::prepare_native_hunter(owner, admitted).await?;
    let native_boss = {
        let mut runtime = owner.runtime.lock().await;
        let sources = world
            .activation_spawns(|key| {
                let policy = runtime.companion_policy(key).ok()?;
                Some((
                    u64::try_from(policy.maximum_health).ok()?,
                    policy.blocked_by_nearby_players,
                ))
            })
            .ok_or("source Koshei spawn policy")?;
        assert_eq!(
            runtime.begin_activation_spawn_sources(sources).map_err(debug)?,
            1
        );
        let progress = runtime.realize_activation_spawn_window(0).map_err(debug)?;
        assert_eq!((progress.realized, progress.remaining), (1, 0));
        let points = runtime.spawn_point_creatures();
        let [(_, _, Some(boss))] = points.as_slice() else {
            return Err("one live Koshei from the shipped spawn table".into());
        };
        *boss
    };
    let potion_fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("Koshei potion fence")?;
    // Seed full stacks so the fixture covers the native fight's healing demand
    // when unrelated content changes combat rolls.
    for occurrence in [175, 176] {
        input_item(
            owner,
            potion_fence.item_fence(),
            &Offering {
                position: at(SWITCH.x - 1, SWITCH.y, SWITCH.floor),
                item: super::annihilator_combat::PALADIN_POTION.into(),
                count: 100,
            },
            occurrence,
            true,
        )
        .await?;
    }
    move_to(owner, admitted, at(SWITCH.x - 1, SWITCH.y, SWITCH.floor)).await?;
    let switch = target(owner, SWITCH, 1)?;
    assert_eq!(
        use_next(owner, admitted, &switch).await?,
        UseDisposition::Committed
    );
    expected.mission[2] = 1;
    assert_eq!(State::read(h).await?, expected);
    let boss = {
        let runtime = owner.runtime.lock().await;
        let visible = runtime.visible_entities();
        let mut matching = visible.creatures.into_iter().filter(|creature| {
            runtime.matches_live_creature_identity(creature.actor, CREATURE.as_bytes())
        });
        let boss = matching.next().ok_or(CREATURE)?;
        assert_eq!(boss.actor, native_boss, "entry must use the live native boss");
        assert!(matching.next().is_none(), "entry must keep one live boss");
        boss
    };
    let encounter = callback()?.effects.encounter.ok_or("Koshei encounter")?;
    let [member] = encounter.players.as_slice() else {
        return Err("one Koshei staging position".into());
    };
    move_to(owner, admitted, member.from).await?;
    let switch = target(owner, SWITCH, 1)?;
    assert_eq!(
        use_next(owner, admitted, &switch).await?,
        UseDisposition::Occupied
    );
    let entry_receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(ENTRY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(entry_receipts, 1, "a live defender cannot be claimed again");
    move_to(owner, admitted, member.to).await?;
    super::elvenbane_combat_fixtures::fight(
        owner,
        admitted,
        boss.actor,
        boss.generation,
        super::annihilator_combat::PALADIN_POTION,
        ChaseMode::Chase,
    )
    .await?;
    expected.mission[3] = 1;
    assert_eq!(State::read(h).await?, expected);

    let corpse = seal(owner, admitted).await?;
    let sample: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_quest_corpse_samples WHERE \
         character_id=encode($1,'hex')::uuid AND corpse_item=encode($2,'hex')::uuid \
         AND transition_key='oteryn:quest-reward/koshei/tomb-seal' AND appearance=7535)",
    )
    .bind(id(41).as_slice())
    .bind(corpse.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert!(
        sample,
        "the seal receipt must identify the corpse minted by the encounter"
    );
    expected.mission[4] = 1;
    expected.items[4] = 0;
    assert_eq!(State::read(h).await?, expected);

    move_to(owner, admitted, at(DOOR.x + 1, DOOR.y, DOOR.floor)).await?;
    let door = target(owner, DOOR, 1)?;
    assert_eq!(
        use_next(owner, admitted, &door).await?,
        UseDisposition::Committed
    );
    assert_eq!(State::read(h).await?, expected);
    for (position, item, quantity) in [(LEGS, 5, 1), (PLATINUM, 6, 50)] {
        super::desert_earned_route_fixtures::walk_to(
            owner,
            admitted,
            at(position.x + 1, position.y, position.floor),
        )
        .await?;
        let chest = target(owner, position, 1)?;
        let command = admitted.continuity.next_command_id;
        assert_eq!(
            use_object(owner, admitted, command, &chest).await?,
            UseDisposition::Committed
        );
        expected.items[item] = quantity;
        expected.completed = item == 6;
        assert_eq!(State::read(h).await?, expected);
        assert_eq!(
            use_object(owner, admitted, command, &chest).await?,
            UseDisposition::Committed
        );
        assert_eq!(State::read(h).await?, expected);
    }

    let previous = *admitted;
    assert_eq!(
        owner
            .lose_control_for_cause(previous, std::time::Duration::ZERO, None)
            .await,
        crate::gameplay_transport::connection::ControlLossResult::Recorded
    );
    let token = super::house_transfer_fixtures::RecoverySource::token(7)?;
    *admitted = super::house_transfer_fixtures::resume_same_session(
        owner,
        previous.game_session_id,
        &token,
        10,
        previous.continuity.selected_capabilities.as_slice(),
    )
    .await?;
    assert_eq!(
        admitted.continuity.connection_generation,
        previous.continuity.connection_generation + 1
    );
    let chest = target(owner, PLATINUM, 1)?;
    assert_eq!(
        use_next(owner, admitted, &chest).await?,
        UseDisposition::NothingToUse
    );
    assert_eq!(State::read(h).await?, expected);

    use oteryn_protocol_oteryn::quest_log::{
        QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query,
    };
    let command = admitted.continuity.next_command_id;
    let reply = wire(
        owner,
        admitted,
        command,
        22,
        encode_quest_log_query(&QuestLogQuery::ListOnly).map_err(debug)?,
    )
    .await?;
    let log = decode_quest_log(&reply).map_err(debug)?;
    let Some(QuestLogView::List(entries)) = log.view else {
        return Err("quest log list".into());
    };
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let log_content =
        oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let listed = log_content.content.active().quests();
    assert!(entries.iter().all(|entry| {
        listed
            .get(entry.quest_index as usize)
            .is_some_and(|quest| quest.key != QUEST)
    }));
    assert_eq!(State::read(h).await?, expected);
    Ok(())
}
