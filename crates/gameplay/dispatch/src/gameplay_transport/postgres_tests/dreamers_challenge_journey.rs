//! The chosen Nightmare route runs on the compiled production World and catalogue.
use super::fixture::{Harness, debug, id, run};
use super::full_world_support::{
    ChannelContentPin, FreshAdmissionAuthority, MovementLocalPosition, NativeItemWeights,
    SelectedCapabilities,
};
use super::quest_world_callbacks_fixtures::{input_item, local, wire};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::{CollisionClass, LogicalCell};
use crate::gameplay_transport::connection::{FirstEntryOutcome, LogoutReply, SessionContinuity};
use crate::gameplay_transport::quest_log::QuestLogLines;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_quests::quest::{
    dreamer_fire_bug,
    loader::load_embedded_quest_state,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::{
    logout::LogoutResult,
    npc_service::{
        NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result, encode_npc_talk_intent,
    },
    world_object::{UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent},
    world_spatial::{StepDirection, StepDisposition},
    world_spatial_entities::EntityRef,
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

const QUEST: &str = "oteryn:quest.dreamer_s_challenge_quest";
const TRACK: &str = "oteryn:quest-progress/chosen-source/dreamer_s_challenge_quest/";
const REPORT: &str = "oteryn:quest-reward/dreamers-challenge/report";
const NPC: &str = "oteryn:npc.the_dream_master";
const START: Position = pos(32861, 32240, 9);
const NAME: &str = "dreamers_challenge_full_journey";

// Crystal actions_walls.lua and actions_tic_tac_teleport.lua on the committed cells.
const WALL_CONTROLS: [(u16, u16); 4] = [
    (32763, 32291),
    (32761, 32291),
    (32763, 32289),
    (32761, 32289),
];
const TOKENS: [(u16, u16, u16); 9] = [
    (32845, 32264, 3548),
    (32843, 32266, 3548),
    (32843, 32268, 3548),
    (32845, 32268, 3548),
    (32844, 32267, 3548),
    (32840, 32269, 3548),
    (32841, 32269, 3547),
    (32840, 32268, 3547),
    (32842, 32267, 3547),
];

const fn pos(x: u16, y: u16, z: i8) -> Position {
    Position { x, y, floor: -z }
}

#[test]
fn dreamers_challenge_full_journey() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, move || {
        run(async move |_| {
            let harness = Harness::create_with_admission_seed(admin, NAME, true, false).await?;
            let result = exercise(&harness).await;
            harness.cleanup().await?;
            result
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {NAME} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let lowered = load_embedded_quest_state("content-1").map_err(debug)?;
    assert!(
        lowered.catalogue().transition(REPORT).is_some(),
        "chosen Dreamer report must be embedded"
    );
    let journal = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let catalogue = Arc::new(lowered.catalogue().clone());
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, active_pin) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("active production content")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let world = super::qc_pinned_world::world(native)?;
    let npc_position = dream_master_placement()?;
    let cells = world
        .movement_cells(active_pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(world_id);
    let server = generation.identity().server_artifact_digest();
    let client = generation.identity().client_artifact_digest();
    let start = local(START);
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        server,
        client,
        frame,
        world.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let deps = Dependencies::with_content(
        pin,
        active_pin.qualified_room().clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.quest_catalogue = Some(catalogue);
    owner.quest_log_content = Some(QuestLogLines::new(journal.content).ok_or("quest log")?);
    owner.movement_cells = (&cells).into();
    let mut admitted = deps.admitted;
    deps.runtime
        .lock()
        .await
        .initialize_pinned_test_position(deps.actor, start)
        .map_err(debug)?;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("journey character fence")?;
    admitted.item_fence = Some(fence.item_fence());
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(server.as_slice()).bind(client.as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            server,
            fresh.operation(),
        )
        .await
        .map_err(debug)?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: super::viola_book_fixtures::definition(2854),
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &NativeItemWeights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    owner.premium_coordinator = Some(&premium);
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &offered,
        &[
            3,
            4,
            6,
            11,
            12,
            13,
            14,
            15,
            16,
            18,
            25,
            oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_STACK_COUNT_V1,
        ],
    )
    .ok_or("journey capabilities")?;
    deps.initialize(&owner).await;
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;

    // Equipment from outside the quest. The essence supply stays on the floor until MOVE.
    for (serial, item) in [(60, 3595), (61, 5467), (62, 6299)] {
        input_item(
            &owner,
            fence.item_fence(),
            &Offering {
                position: START,
                item: item_key(item),
                count: 1,
            },
            serial,
            true,
        )
        .await?;
    }
    let mut journey = Journey {
        owner,
        admitted,
        h,
        mission: 0,
        inventory: inventory(h).await?,
    };
    journey.check("fresh character").await?;
    journey.entrance().await?;
    journey.puzzles().await?;
    journey.maze().await?;
    journey.finish(npc_position).await
}

struct Journey<'a, 'f, 's> {
    owner: ComposedFreshAdmission<'a, 'f, 's>,
    admitted: AdmittedSession,
    h: &'a Harness,
    mission: u8,
    inventory: BTreeMap<String, i64>,
}

impl Journey<'_, '_, '_> {
    async fn check(&self, interaction: &str) -> TestResult {
        let states: Vec<(String, i64)> = sqlx::query_as(
            "SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key LIKE $2 ORDER BY track_key LIMIT 32")
            .bind(id(41).as_slice()).bind(format!("{TRACK}%")).fetch_all(&self.h.pool).await?;
        for stage in 1..=7 {
            let key = format!("{TRACK}s{stage}");
            let value = states
                .iter()
                .find(|(track, _)| track == &key)
                .map_or(0, |(_, value)| *value);
            assert_eq!(
                value,
                i64::from(stage <= self.mission),
                "{interaction}, mission {stage}"
            );
        }
        let completed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND completed_character_revision IS NOT NULL)")
            .bind(id(41).as_slice()).bind(QUEST).fetch_one(&self.h.pool).await?;
        assert_eq!(completed, self.mission == 7, "{interaction}, completion");
        assert_eq!(
            inventory(self.h).await?,
            self.inventory,
            "{interaction}, inventory"
        );
        Ok(())
    }

    fn update(&mut self, item: u16, delta: i64) {
        let key = item_key(item);
        let quantity = self.inventory.entry(key.clone()).or_default();
        *quantity += delta;
        if *quantity == 0 {
            self.inventory.remove(&key);
        }
    }

    async fn entrance(&mut self) -> TestResult {
        self.use_item(pos(32860, 32249, 9), 4240, Some(6195))
            .await?;
        self.update(2854, 1);
        self.check("Hugo's backpack").await?;
        self.walk(pos(32861, 32239, 9), 0).await?;
        self.arrived(pos(32861, 32235, 9)).await?;
        self.update(3595, -1);
        self.check("premium carrot step").await?;
        self.use_item(dreamer_fire_bug::SOURCE, 3134, Some(2243))
            .await?;
        self.check("retained Fire Bug after ignition").await?;
        self.walk(dreamer_fire_bug::SOURCE, 0).await?;
        self.arrived(dreamer_fire_bug::DESTINATION).await?;
        self.check("Fire Bug portal").await?;
        self.drop_item(pos(32857, 32225, 11), 6299, 1).await?;
        self.update(6299, -1);
        self.check("Death Ring MOVE offering").await?;
        self.walk(pos(32857, 32230, 11), 0).await?;
        self.arrived(pos(32819, 32347, 9)).await?;
        self.mission = 1;
        self.check("Death Ring room exit").await
    }

    async fn puzzles(&mut self) -> TestResult {
        for (control, count) in COLOUR_MOVES {
            let plate = if control < 6 {
                pos(32816, 32334 + control, 9)
            } else {
                pos(32818 + control - 6, 32332, 9)
            };
            for _ in 0..count {
                self.walk(plate, 0).await?;
                self.walk(
                    pos(
                        plate.x - u16::from(control < 6),
                        plate.y - u16::from(control >= 6),
                        9,
                    ),
                    0,
                )
                .await?;
                self.check("matching-colour floor switch").await?;
            }
        }
        self.walk(pos(32826, 32347, 9), 0).await?;
        self.arrived(pos(32766, 32275, 14)).await?;
        self.mission = 2;
        self.check("matching-colour exit").await?;
        self.walk(pos(32762, 32280, 14), 0).await?;
        self.arrived(pos(32749, 32289, 14)).await?;
        self.walk(pos(32764, 32305, 14), 0).await?;
        self.arrived(pos(32762, 32289, 14)).await?;
        for (x, y) in WALL_CONTROLS {
            self.use_item(pos(x, y, 14), 2772, None).await?;
            self.check("wall lever").await?;
        }
        self.walk(pos(32762, 32290, 14), 0).await?;
        self.arrived(pos(32852, 32287, 14)).await?;
        self.mission = 3;
        self.check("open wall exit").await?;

        self.use_item(pos(32850, 32285, 14), 2472, Some(6196))
            .await?;
        self.update(2821, 1);
        self.check("Book of the Lost Heroes").await?;
        let book: String = sqlx::query_scalar(
            "SELECT t.text FROM game_item_texts t JOIN game_item_instances i USING(item_instance_id) \
             JOIN game_item_container_entries e USING(item_instance_id) \
             WHERE e.character_id=encode($1,'hex')::uuid AND i.definition_production_key=$2 LIMIT 1",
        )
        .bind(id(41).as_slice())
        .bind(item_key(2821))
        .fetch_one(&self.h.pool)
        .await?;
        assert_eq!(book, "Askuro\nMantar\nIldoni\nMaxmof\nXemodo\nOxanti");
        self.use_item(pos(32838, 32263, 14), 2772, Some(2272))
            .await?;
        self.update(3547, 8);
        self.update(3548, 12);
        self.check("token switch").await?;
        for (x, y, item) in TOKENS {
            self.drop_item(pos(x, y, 14), item, 1).await?;
            self.update(item, -1);
            self.check("tic-tac-toe token placed through MOVE").await?;
        }
        self.use_item(pos(32834, 32285, 14), 2772, None).await?;
        self.check("tic-tac-toe switch").await?;
        self.walk(pos(32836, 32288, 14), 0).await?;
        self.arrived(pos(32920, 32296, 13)).await?;
        self.mission = 4;
        self.check("tic-tac-toe exit").await
    }

    async fn maze(&mut self) -> TestResult {
        for (source, destination) in [
            (pos(32920, 32297, 13), pos(32916, 32278, 14)),
            (pos(32915, 32278, 14), pos(32915, 32263, 14)),
            (pos(32916, 32263, 14), pos(32946, 32270, 13)),
            (pos(32953, 32277, 13), pos(32976, 32270, 14)),
            (pos(32976, 32271, 14), pos(32933, 32282, 13)),
            (pos(32933, 32269, 13), pos(32753, 32344, 14)),
        ] {
            self.walk(source, 0).await?;
            self.arrived(destination).await?;
            self.check("dream maze STEP").await?;
        }
        self.use_item(pos(32753, 32341, 14), 2469, Some(6199))
            .await?;
        self.update(2853, 1);
        self.check("Dream Realm wine bag").await?;
        let bag: Vec<(String, i32)> = sqlx::query_as(
            "SELECT i.definition_production_key,coalesce(f.fluid_subtype,0)::integer \
             FROM game_item_nested_entries n JOIN game_item_instances i USING(item_instance_id) \
             LEFT JOIN game_item_fluid_states f USING(item_instance_id) \
             JOIN game_item_instances root ON root.item_instance_id=n.parent_item_instance_id \
             JOIN game_item_container_entries e ON e.item_instance_id=root.item_instance_id \
             WHERE e.character_id=encode($1,'hex')::uuid AND root.definition_production_key=$2 \
             ORDER BY i.definition_production_key LIMIT 4",
        )
        .bind(id(41).as_slice())
        .bind(item_key(2853))
        .fetch_all(&self.h.pool)
        .await?;
        assert_eq!(
            bag,
            [
                (item_key(2874), 2),
                (item_key(3602), 0),
                (item_key(6498), 0)
            ]
        );
        self.walk(pos(32760, 32330, 14), 0).await?;
        self.arrived(pos(32844, 32238, 14)).await?;
        self.walk(pos(32840, 32226, 14), 0).await?;
        self.arrived(pos(32844, 32229, 14)).await?;
        self.check("Nightmare entrance").await
    }

    async fn finish(&mut self, placement: Position) -> TestResult {
        let [npc] = super::classic_quests_fixtures::install_positions(
            &self.owner,
            &self.admitted,
            [NPC],
            [placement],
        )
        .await?;
        assert_eq!(self.talk(npc, "hi").await?, NpcIntentDisposition::Ok);
        assert_eq!(
            self.talk(npc, "nightmare knights").await?,
            NpcIntentDisposition::Ok
        );
        self.mission = 5;
        self.check("Nightmare Knights chosen").await?;

        let at = pos(32844, 32229, 14);
        let fence = self
            .owner
            .current_quest_fence(self.admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("essence pickup fence")?;
        input_item(
            &self.owner,
            fence.item_fence(),
            &Offering {
                position: at,
                item: item_key(6499),
                count: 100,
            },
            63,
            false,
        )
        .await?;
        self.check("essence source is on the floor").await?;
        self.pickup_essences(at).await?;
        self.update(6499, 100);
        self.mission = 6;
        self.check("collect 100 essences").await?;

        assert_eq!(
            self.talk(npc, "advancement").await?,
            NpcIntentDisposition::Ok
        );
        self.check("hand-in prompt").await?;
        let command = self.admitted.continuity.next_command_id;
        let payload = encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: "yes".into(),
        })
        .map_err(debug)?;
        self.tick(Duration::from_millis(500))?;
        let reply = wire(&self.owner, &mut self.admitted, command, 7, payload.clone()).await?;
        assert_eq!(
            decode_npc_intent_result(&reply).map_err(debug)?,
            NpcIntentDisposition::Ok
        );
        self.update(6499, -100);
        self.update(768, 1);
        self.mission = 7;
        self.check("100 essences exchanged for follower document")
            .await?;
        self.terminal_receipts().await?;
        let replay = wire(&self.owner, &mut self.admitted, command, 7, payload).await?;
        assert_eq!(replay, reply);
        self.check("terminal command replay").await?;
        self.terminal_receipts().await?;
        self.reconnect(npc).await?;
        self.check("fresh command refused after reconnect").await?;
        self.terminal_receipts().await?;
        self.hidden_log().await
    }

    fn tick(&mut self, duration: Duration) -> TestResult {
        self.owner.clock_origin = self
            .owner
            .clock_origin
            .checked_sub(duration)
            .ok_or("fixture clock")?;
        Ok(())
    }

    async fn current(&self) -> TestResult<MovementLocalPosition> {
        super::full_world_support::position(
            &self.owner,
            self.admitted.runtime_actor.ok_or("actor")?,
        )
        .await
    }

    async fn arrived(&self, expected: Position) -> TestResult {
        assert_eq!(self.current().await?, local(expected), "journey landing");
        Ok(())
    }

    async fn walk(&mut self, target: Position, reach: u16) -> TestResult {
        let current = self.current().await?;
        let cells = &self.owner.movement_cells;
        let destination = local(target);
        if current.floor != destination.floor {
            return Err(format!("cannot approach {destination:?} from {current:?}").into());
        }
        let mut frontier = VecDeque::from([(current.x, current.y)]);
        let mut parents = BTreeMap::from([((current.x, current.y), None)]);
        let mut end = None;
        for _ in 0..4096 {
            let Some((x, y)) = frontier.pop_front() else {
                break;
            };
            if x.abs_diff(destination.x).max(y.abs_diff(destination.y)) <= u32::from(reach) {
                end = Some((x, y));
                break;
            }
            for direction in [
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ] {
                let (dx, dy) = direction.delta();
                let next = (x + dx, y + dy);
                if parents.contains_key(&next)
                    || next
                        .0
                        .abs_diff(destination.x)
                        .max(next.1.abs_diff(destination.y))
                        > 80
                {
                    continue;
                }
                let cell = LogicalCell {
                    x: next.0,
                    y: next.1,
                    z: i32::from(current.floor),
                };
                if !matches!(
                    cells.index().lookup(cells.scope(), cell),
                    Ok(CollisionClass::Walkable)
                ) || (next != (destination.x, destination.y)
                    && crate::map::floor::is_teleport(cells, cell))
                {
                    continue;
                }
                parents.insert(next, Some(((x, y), direction)));
                frontier.push_back(next);
            }
        }
        let mut at =
            end.ok_or_else(|| format!("no production route from {current:?} to {target:?}"))?;
        let mut steps = Vec::new();
        while let Some(&(previous, direction)) = parents.get(&at).and_then(Option::as_ref) {
            steps.push(direction);
            at = previous;
        }
        for direction in steps.into_iter().rev() {
            self.tick(Duration::from_millis(450))?;
            let (outcome, _) = self
                .owner
                .paced_step(
                    self.admitted.runtime_actor.ok_or("actor")?,
                    self.admitted.game_session_id,
                    direction,
                )
                .await;
            assert_eq!(
                outcome.disposition,
                StepDisposition::Moved,
                "approaching {target:?}"
            );
        }
        Ok(())
    }

    async fn use_item(&mut self, at: Position, appearance: u16, uid: Option<u16>) -> TestResult {
        self.walk(at, 1).await?;
        let map = self.owner.bundle_map().ok_or("world")?;
        let base = map.overlay.base();
        let tile = base.tile(at.x, at.y, at.floor).ok_or("source tile")?;
        let ordinal = super::qc_pinned_world::ordinal(tile, appearance, uid)?;
        let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, ordinal)
            .ok_or("source placement")?;
        let mut placement = base.digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        };
        let payload = encode_use_intent(&target).map_err(debug)?;
        let command = self.admitted.continuity.next_command_id;
        self.tick(Duration::from_millis(250))?;
        let reply = wire(&self.owner, &mut self.admitted, command, 2, payload).await?;
        assert_eq!(
            decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed,
            "USE {appearance} at {at:?}"
        );
        Ok(())
    }

    async fn drop_item(&mut self, at: Position, item: u16, count: u8) -> TestResult {
        if item != 6299 {
            self.walk(at, 1).await?;
        }
        let reference = item_reference(&self.owner, item)?;
        let (next, outcome, _) = super::map_seed_planting_fixtures::drop_exchange_at(
            &self.owner,
            self.admitted,
            self.admitted.continuity.next_command_id,
            None,
            reference,
            count,
            false,
            oteryn_protocol_oteryn::item_view::WorldTilePosition {
                x: i32::from(at.x),
                y: i32::from(at.y),
                floor: -i16::from(at.floor),
            },
        )
        .await?;
        self.admitted = next;
        assert_eq!(
            outcome,
            Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Moved),
            "MOVE item {item} to {at:?}"
        );
        Ok(())
    }

    async fn pickup_essences(&mut self, at: Position) -> TestResult {
        use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
        use oteryn_protocol_oteryn::{
            ClientCommandValue, MessageType, decode_command_result, decode_snapshot_body,
            decode_snapshot_chunk_framing, decode_wire_envelope, encode_client_command,
            item_view::{
                COMMAND_TYPE_ITEM_MOVE_INTENT, ItemHandle, ItemMoveDestination, ItemMoveIntent,
                ItemMoveOutcome, ItemMoveSelection, decode_item_move_result_with_equip_drop,
                encode_item_move_intent_for,
            },
            world_map::{
                MapDefinition, MapOrigin, STATE_DOMAIN_WORLD_MAP_VIEW, decode_world_map_snapshot,
            },
        };
        use tokio::io::AsyncWriteExt;

        let item = item_reference(&self.owner, 6499)?;
        let admitted = self.admitted;
        let command = admitted.continuity.next_command_id;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let serve = async {
            let end = serve_admitted(&mut server, admitted, &self.owner, IDLE_LIVENESS).await;
            drop(server);
            end
        };
        let peer = async {
            let mut snapshot = Vec::new();
            loop {
                let raw = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&raw).map_err(debug)?;
                match envelope.message_type() {
                    MessageType::SnapshotChunk => {
                        let (_, body) =
                            decode_snapshot_chunk_framing(envelope.payload()).map_err(debug)?;
                        snapshot.extend_from_slice(body);
                    }
                    MessageType::SnapshotCommit => break,
                    _ => continue,
                }
            }
            let domains = decode_snapshot_body(&snapshot).map_err(debug)?;
            let map = domains
                .iter()
                .find(|domain| domain.domain_id == STATE_DOMAIN_WORLD_MAP_VIEW)
                .ok_or("world map snapshot")?;
            let map = decode_world_map_snapshot(map.payload).map_err(debug)?;
            let cell = map
                .tiles
                .iter()
                .find(|cell| {
                    cell.position.x == i32::from(at.x)
                        && cell.position.y == i32::from(at.y)
                        && cell.position.floor == -i16::from(at.floor)
                })
                .ok_or("visible essence cell")?;
            let item = cell
                .items
                .iter()
                .find(|entry| entry.definition == MapDefinition::Item(item))
                .ok_or("visible essence stack")?;
            assert_eq!(item.count, 100);
            let MapOrigin::Handle(handle) = item.origin else {
                return Err("essence pickup handle".into());
            };
            let payload = encode_item_move_intent_for(
                &ItemMoveIntent {
                    source: ItemHandle::new(handle.get()).ok_or("essence item handle")?,
                    destination: ItemMoveDestination::MainBackpack,
                },
                ItemMoveSelection {
                    container_tree: true,
                    equip_drop: true,
                    ..Default::default()
                },
            )
            .map_err(debug)?;
            let frame = encode_client_command(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: command,
                    command_type: COMMAND_TYPE_ITEM_MOVE_INTENT,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            client.write_u32(u32::try_from(frame.len())?).await?;
            client.write_all(&frame).await?;
            loop {
                let raw = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&raw).map_err(debug)?;
                if envelope.message_type() != MessageType::CommandResult {
                    continue;
                }
                let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(reply.command_id, command);
                assert_eq!(
                    decode_item_move_result_with_equip_drop(reply.payload, true).map_err(debug)?,
                    ItemMoveOutcome::Moved
                );
                client.shutdown().await?;
                return Ok::<_, Box<dyn std::error::Error>>(());
            }
        };
        let (end, outcome) = tokio::time::timeout(
            Duration::from_secs(30),
            super::fixture::join_two(serve, peer),
        )
        .await?;
        outcome?;
        let ConnectionEnd::AdmittedThenDisconnected(next) = end else {
            return Err(format!("essence MOVE disconnected unexpectedly: {end:?}").into());
        };
        self.admitted = next;
        Ok(())
    }

    async fn talk(&mut self, npc: EntityRef, text: &str) -> TestResult<NpcIntentDisposition> {
        self.tick(Duration::from_millis(500))?;
        let payload = encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: npc,
            text: text.into(),
        })
        .map_err(debug)?;
        let command = self.admitted.continuity.next_command_id;
        let response = wire(&self.owner, &mut self.admitted, command, 7, payload).await?;
        decode_npc_intent_result(&response).map_err(|error| format!("{text}: {error:?}").into())
    }

    async fn reconnect(&mut self, npc: EntityRef) -> TestResult {
        let previous = self.admitted.game_session_id;
        assert_eq!(
            self.owner.logout_session(self.admitted).await,
            LogoutReply::Result(LogoutResult::Accepted)
        );
        let next = super::full_world_support::relogin(self.h).await?;
        assert_ne!(next, previous);
        let fence = self
            .owner
            .current_quest_fence(next)
            .await
            .map_err(debug)?
            .ok_or("successor fence")?;
        let decided = self
            .owner
            .read_admission_position(fence)
            .await
            .map_err(debug)?;
        let actor = {
            let mut runtime = self.owner.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(next).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            self.owner
                .initialize_admitted_position(&mut runtime, actor, next, &decided)
                .await
                .map_err(debug)?;
            actor
        };
        assert!(self.owner.track_restored(next, actor, &decided, fence));
        let capabilities = self.admitted.continuity.selected_capabilities;
        self.admitted = AdmittedSession {
            game_session_id: next,
            runtime_actor: Some(actor),
            item_fence: Some(fence.item_fence()),
            continuity: SessionContinuity {
                selected_capabilities: capabilities,
                ..SessionContinuity::FRESH
            },
            ..self.admitted
        };
        self.owner.admit_quest_session(&self.admitted).await;
        self.owner
            .states
            .chat
            .admit(next, "Dreamer witness", self.owner.owner_now());
        self.owner.states.chat.set_npc_service(next);
        assert_eq!(self.talk(npc, "hi").await?, NpcIntentDisposition::Ok);
        let response = self.talk(npc, "advancement").await?;
        assert_eq!(response, NpcIntentDisposition::Rejected);
        self.check("completed Dreamer refuses a new offer").await
    }

    async fn terminal_receipts(&self) -> TestResult {
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key=$2 AND completes),              (SELECT count(*) FROM game_account_achievements WHERE account_id=encode($3,'hex')::uuid AND achievement_key='oteryn:achievement/lucid_dreamer')")
             .bind(id(41).as_slice()).bind(REPORT).bind(id(40).as_slice()).fetch_one(&self.h.pool).await?;
        assert_eq!(counts, (1, 1), "exactly one completion and Lucid Dreamer");
        Ok(())
    }

    async fn hidden_log(&mut self) -> TestResult {
        use oteryn_protocol_oteryn::{
            ClientCommandValue, MessageType, decode_state_delta, decode_wire_envelope,
            encode_client_command,
            quest_log::{QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query},
        };
        let payload = encode_quest_log_query(&QuestLogQuery::List).map_err(debug)?;
        let frame = encode_client_command(
            self.admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id: self.admitted.continuity.next_command_id,
                command_type: 22,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        let mut input = u32::try_from(frame.len())?.to_be_bytes().to_vec();
        input.extend(frame);
        let (_, output) = super::weather_wire_fixtures::fixture_access::serve_input(
            &self.owner,
            self.admitted,
            &input,
        )
        .await?;
        let mut reader = output.as_slice();
        while !reader.is_empty() {
            let message = crate::gameplay_transport::tcp_tls::read_frame(&mut reader).await?;
            let envelope = decode_wire_envelope(&message).map_err(debug)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == 16 {
                    let list = decode_quest_log(delta.payload).map_err(debug)?;
                    assert!(
                        matches!(list.view, Some(QuestLogView::List(entries)) if entries.is_empty()),
                        "the fresh character's completed hidden quest must not appear"
                    );
                    return Ok(());
                }
            }
        }
        Err("quest log did not publish a list".into())
    }
}

fn item_key(item: u16) -> String {
    format!("oteryn:item.tibia.i{item}")
}

fn item_reference(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    item: u16,
) -> TestResult<std::num::NonZeroU32> {
    owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", &item_key(item), "definition-r1"))
        .ok_or_else(|| format!("active item binding for {item}").into())
}

async fn inventory(h: &Harness) -> TestResult<BTreeMap<String, i64>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT i.definition_production_key,sum(i.quantity)::bigint FROM game_item_container_entries e          JOIN game_item_instances i USING(item_instance_id) WHERE e.character_id=encode($1,'hex')::uuid          AND i.lifecycle=1 GROUP BY i.definition_production_key ORDER BY i.definition_production_key LIMIT 64")
        .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
    Ok(rows.into_iter().collect())
}

fn dream_master_placement() -> TestResult<Position> {
    let (bytes, _) = super::qc_pinned_world::artifact()?;
    let bundle = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let master = bundle
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == NPC)
        .ok_or("The Dream Master is absent from the production World")?;
    let [placement] = master.placements.as_slice() else {
        return Err("ambiguous Dream Master placement".into());
    };
    assert_eq!(
        (placement.x, placement.y, placement.floor),
        (32844, 32228, -14)
    );
    Ok(pos(placement.x, placement.y, -placement.floor))
}

// Legal row/column presses that solve the committed B3 board, in zero-based control order.
const COLOUR_MOVES: [(u16, u16); 124] = [
    (2, 4),
    (11, 5),
    (2, 2),
    (11, 1),
    (0, 5),
    (8, 1),
    (0, 1),
    (8, 5),
    (6, 3),
    (1, 2),
    (6, 3),
    (1, 4),
    (6, 3),
    (3, 3),
    (6, 3),
    (3, 3),
    (5, 3),
    (8, 2),
    (5, 3),
    (8, 4),
    (4, 5),
    (9, 3),
    (4, 1),
    (9, 3),
    (5, 3),
    (10, 2),
    (5, 3),
    (10, 4),
    (6, 3),
    (2, 4),
    (6, 3),
    (2, 2),
    (1, 2),
    (7, 1),
    (1, 4),
    (7, 5),
    (10, 4),
    (4, 3),
    (10, 2),
    (4, 3),
    (4, 2),
    (9, 5),
    (4, 4),
    (9, 1),
    (1, 5),
    (7, 5),
    (1, 1),
    (7, 1),
    (9, 4),
    (0, 4),
    (9, 2),
    (0, 2),
    (5, 1),
    (7, 1),
    (5, 5),
    (7, 5),
    (11, 2),
    (5, 2),
    (11, 4),
    (5, 4),
    (7, 1),
    (1, 5),
    (7, 5),
    (1, 1),
    (7, 5),
    (0, 1),
    (7, 1),
    (0, 5),
    (8, 2),
    (2, 2),
    (8, 4),
    (2, 4),
    (11, 5),
    (4, 2),
    (11, 1),
    (4, 4),
    (8, 1),
    (2, 1),
    (8, 5),
    (2, 5),
    (2, 2),
    (8, 2),
    (2, 4),
    (8, 4),
    (9, 1),
    (2, 4),
    (9, 5),
    (2, 2),
    (7, 4),
    (0, 5),
    (7, 2),
    (0, 1),
    (4, 5),
    (6, 1),
    (4, 1),
    (6, 5),
    (5, 4),
    (6, 2),
    (5, 2),
    (6, 4),
    (4, 4),
    (6, 1),
    (4, 2),
    (6, 5),
    (4, 5),
    (9, 5),
    (4, 1),
    (9, 1),
    (9, 1),
    (5, 5),
    (9, 5),
    (5, 1),
    (8, 5),
    (1, 1),
    (8, 1),
    (1, 5),
    (0, 2),
    (10, 1),
    (0, 4),
    (10, 5),
    (0, 1),
    (10, 1),
    (0, 5),
    (10, 5),
];
