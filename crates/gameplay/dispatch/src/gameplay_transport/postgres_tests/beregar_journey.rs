//! Native Beregar story and terminal chest, using the generated World placement inputs.
use super::fixture::{Harness, debug};
use super::l6_world_pickups_fixtures::{
    ExpectedTeleport, NativeCrop, native_scene_with_appearances,
};
use super::quest_world_callbacks_fixtures::{input_item, local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::view::MapFacts as _;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Offering, Position};
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result, encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::quest_log::{
    QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query,
};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, encode_step_intent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;

pub(super) const NAME: &str = "beregar-full-journey";
const QUEST: &str = "oteryn:quest.the_hidden_city_of_beregar_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u8_4/the_hidden_city_of_beregar/";
pub(super) const START: Position = at(32614, 31501, -14);

const fn at(x: u16, y: u16, floor: i8) -> Position {
    Position { x, y, floor }
}

#[test]
fn beregar_full_journey_earns_firewalker_boots() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run(NAME, move || {
        drop(admin);
        super::quest_world_callbacks_fixtures::scenario(NAME, at(32611, 31497, -14), false)
    })
}

pub(super) fn lift_callback() -> TestResult<Callback> {
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    oteryn_gameplay_quests::quest::world_callbacks::load_for(loaded.catalogue())?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/beregar/lift-down")
        .ok_or_else(|| "Beregar lift exchange lacks its native STEP binding".into())
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    macro_rules! crop {
        ($file:literal, $z:literal, $ry:literal, $bounds:expr) => {
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/",
                    $file
                )),
                region: ($z, 127, $ry),
                bounds: $bounds,
                start: (START.x, START.y),
                expected: vec![],
            }
        };
    }
    native_scene_with_appearances(
        native,
        vec![
            crop!(
                "region-z14-x127-y123.b3",
                14,
                123,
                (32582, 31487, 32617, 31518)
            ),
            crop!(
                "region-z15-x127-y123.b3",
                15,
                123,
                (32609, 31496, 32615, 31501)
            ),
            crop!(
                "region-z14-x127-y122.b3",
                14,
                122,
                (32598, 31383, 32617, 31400)
            ),
            crop!(
                "region-z08-x127-y122.b3",
                8,
                122,
                (32580, 31399, 32587, 31406)
            ),
            crop!(
                "region-z10-x127-y122.b3",
                10,
                122,
                (32530, 31434, 32537, 31441)
            ),
            crop!(
                "region-z15-x127-y122.b3",
                15,
                122,
                (32546, 31371, 32602, 31416)
            ),
        ],
        &[
            ExpectedTeleport {
                position: (32599, 31397, 14),
                ordinal: 1,
                destination: (32598, 31406, 15),
            },
            ExpectedTeleport {
                position: (32582, 31399, 8),
                ordinal: 1,
                destination: (32582, 31403, 8),
            },
            ExpectedTeleport {
                position: (32577, 31405, 15),
                ordinal: 1,
                destination: (32601, 31396, 14),
            },
        ],
        &[1949, 5815],
    )
}

struct Journey<'a, 'b, 'h, 'r> {
    owner: &'a mut ComposedFreshAdmission<'b, 'h, 'r>,
    admitted: &'a mut AdmittedSession,
    harness: &'a Harness,
    journal_index: u32,
}

impl Journey<'_, '_, '_, '_> {
    async fn send(&mut self, kind: u32, payload: Vec<u8>) -> TestResult<Vec<u8>> {
        self.owner.clock_origin -= Duration::from_secs(2);
        let command = self.admitted.continuity.next_command_id;
        wire(self.owner, self.admitted, command, kind, payload).await
    }

    async fn place_at(&self, position: Position) -> TestResult {
        // Between cropped locations the fixture stages ordinary travel. Every quest
        // translation (lift, dungeon, arena and reward room) is driven by STEP.
        let _permit = self
            .owner
            .spell_lane_permit()
            .await
            .ok_or("Beregar movement lane")?;
        let actor = self.admitted.runtime_actor.ok_or("Beregar player")?;
        let mut runtime = self.owner.runtime.lock().await;
        let current = runtime.read_actor_position(actor).map_err(debug)?;
        if current.position() == local(position) {
            return Ok(());
        }
        let map = self.owner.bundle_map().ok_or("Beregar map")?;
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            self.owner.movement_cells(),
            actor,
            self.admitted.game_session_id,
            current,
            local(position),
            false,
            &doors,
        )
        .map_err(debug)?;
        runtime.commit_source_step(proof).map_err(debug)?;
        Ok(())
    }

    async fn position(&self) -> TestResult<crate::foundation::MovementLocalPosition> {
        Ok(self
            .owner
            .runtime
            .lock()
            .await
            .read_actor_position(self.admitted.runtime_actor.ok_or("Beregar player")?)
            .map_err(debug)?
            .position())
    }

    async fn talk(&mut self, npc: EntityRef, words: &[&str]) -> TestResult {
        for &word in words {
            let result = self
                .send(
                    7,
                    encode_npc_talk_intent(&NpcTalkIntent {
                        npc_actor: npc,
                        text: word.into(),
                    })
                    .map_err(debug)?,
                )
                .await?;
            assert_eq!(
                decode_npc_intent_result(&result).map_err(debug)?,
                NpcIntentDisposition::Ok,
                "{word}"
            );
        }
        Ok(())
    }

    async fn step(&mut self, direction: StepDirection, destination: Position) -> TestResult {
        self.send(1, encode_step_intent(direction)).await?;
        assert_eq!(self.position().await?, local(destination));
        Ok(())
    }

    async fn state(&self, mission: &str, expected: i64, inventory: &[(u16, i64)]) -> TestResult {
        let actual: i64 = sqlx::query_scalar(
            "SELECT coalesce((SELECT value FROM game_character_quest_tracks WHERE track_key=$1),0)",
        )
        .bind(format!("{TRACK}{mission}"))
        .fetch_one(&self.harness.pool)
        .await?;
        assert_eq!(actual, expected, "{mission}");
        for &(item, count) in inventory {
            let actual: i64 = sqlx::query_scalar(
                "SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1",
            ).bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&self.harness.pool).await?;
            assert_eq!(actual, count, "{mission}: item {item}");
        }
        Ok(())
    }

    fn target(&self, position: Position, ordinal: u8) -> TestResult<WorldObjectTarget> {
        let map = self.owner.bundle_map().ok_or("Beregar map")?;
        let key = crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
            ordinal,
        )
        .ok_or("Beregar placement")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        Ok(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })
    }

    async fn use_object(&mut self, position: Position, ordinal: u8) -> TestResult<UseDisposition> {
        let target = self.target(position, ordinal)?;
        let result = self
            .send(2, encode_use_intent(&target).map_err(debug)?)
            .await?;
        decode_use_result(&result)
            .map_err(debug)
            .map_err(Into::into)
    }

    async fn log(&mut self, completed: bool) -> TestResult {
        let payload = encode_quest_log_query(&QuestLogQuery::ListOnly).map_err(debug)?;
        let result = decode_quest_log(&self.send(22, payload).await?).map_err(debug)?;
        let Some(QuestLogView::List(entries)) = result.view else {
            return Err("missing Beregar quest list".into());
        };
        assert!(
            entries.iter().any(
                |entry| entry.quest_index == self.journal_index && entry.completed == completed
            )
        );
        Ok(())
    }

    async fn defeat_pythius(&mut self) -> TestResult {
        let session = self.admitted.game_session_id;
        let actor = self.admitted.runtime_actor.ok_or("Beregar fighter")?;
        let target = {
            let runtime = self.owner.runtime.lock().await;
            let boss = runtime
                .visible_entities()
                .creatures
                .into_iter()
                .find(|creature| {
                    runtime
                        .read_actor_position(creature.actor)
                        .is_ok_and(|p| p.position() == local(at(32571, 31406, -15)))
                })
                .ok_or("native Pythius spawn")?;
            EntityRef {
                identity: boss.actor.placement_identity(),
                generation: boss.generation,
            }
        };
        self.place_at(at(32570, 31406, -15)).await?;
        {
            let runtime = self.owner.runtime.lock().await;
            self.owner
                .states
                .spell_states
                .lock()
                .await
                .get_mut(&runtime, actor, session)
                .ok_or("Beregar combat facts")?
                .facts
                .level = 10_000;
        }
        let result = self
            .send(
                11,
                oteryn_protocol_oteryn::attack::encode_attack_target_intent(Some(&target))
                    .map_err(debug)?,
            )
            .await?;
        assert_eq!(
            result,
            oteryn_protocol_oteryn::attack::encode_attack_intent_result(
                oteryn_protocol_oteryn::attack::AttackIntentDisposition::Ok
            )
        );
        for _ in 0..128 {
            self.owner.refresh_attack_weapon(actor, session).await;
            self.owner.drain_auto_attacks().await;
            self.owner.drain_kill_rewards(session).await;
            let credited: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE track_key=$1 AND value=1)",
            ).bind(format!("{TRACK}pythius_defeated")).fetch_one(&self.harness.pool).await?;
            if credited {
                return self.state("pythius_defeated", 1, &[(9019, 0)]).await;
            }
            self.owner.clock_origin -= Duration::from_secs(3);
        }
        Err("ordinary attacks failed to credit native Pythius".into())
    }
}

fn source_contacts<const N: usize>(keys: [&str; N]) -> TestResult<[Position; N]> {
    let source: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/world/npc-placements/placements-00000-00055.json",
    )))?;
    let records = source["records"]
        .as_array()
        .ok_or("native NPC placements")?;
    let positions = keys
        .into_iter()
        .map(|key| {
            let mut entries = records
                .iter()
                .map(|row| &row["declaration"])
                .filter(|row| row["npc"] == key);
            let row = entries
                .next()
                .ok_or_else(|| format!("missing Beregar World placement for {key}"))?;
            assert!(entries.next().is_none(), "ambiguous Beregar contact {key}");
            let cell = &row["cell"];
            Ok(at(
                u16::try_from(cell["x"].as_u64().ok_or("NPC x")?)?,
                u16::try_from(cell["y"].as_u64().ok_or("NPC y")?)?,
                -i8::try_from(cell["floor"].as_u64().ok_or("NPC floor")?)?,
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    positions
        .try_into()
        .map_err(|_| "Beregar contact count".into())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let keys = [
        "oteryn:npc.xorlosh",
        "oteryn:npc.nokmir",
        "oteryn:npc.emperor_rehal",
        "oteryn:npc.grombur",
        "oteryn:npc.frafnar",
        "oteryn:npc.bolfona",
        "oteryn:npc.pythius_the_rotten",
    ];
    let [xorlosh, nokmir, rehal, grombur, frafnar, bolfona, pythius] =
        super::classic_quests_fixtures::install_positions(
            owner,
            admitted,
            keys,
            source_contacts(keys)?,
        )
        .await?;
    super::qd_world_fixtures::select_talk_capabilities(
        owner,
        admitted,
        &[3, 4, 11, 12, 13, 15, 16, 17],
    )?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "Beregar fence")?
        .ok_or("Beregar fence")?;
    for (index, (item, count)) in [(8775, 3), (8019, 1), (8774, 1), (2903, 1), (3456, 1)]
        .into_iter()
        .enumerate()
    {
        input_item(
            owner,
            fence.item_fence(),
            &Offering {
                item: format!("oteryn:item.tibia.i{item}"),
                count,
                position: START,
            },
            500 + u64::try_from(index)?,
            true,
        )
        .await?;
    }
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let log =
        oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?.content;
    let journal_index = u32::try_from(
        log.active()
            .quests()
            .iter()
            .position(|entry| entry.key == QUEST)
            .ok_or("Beregar quest log entry")?,
    )?;
    owner.quest_log_content = Some(
        crate::gameplay_transport::quest_log::QuestLogLines::new(log)
            .ok_or("Beregar quest log bounds")?,
    );
    owner.admit_quest_session(admitted).await;
    let mut journey = Journey {
        owner,
        admitted,
        harness,
        journal_index,
    };
    journey
        .state("going_down", 0, &[(8775, 3), (8777, 0), (9019, 0)])
        .await?;
    journey.talk(xorlosh, &["hi", "mission", "yes"]).await?;
    journey.state("going_down", 1, &[(8775, 3)]).await?;
    journey.talk(xorlosh, &["mission", "bye"]).await?;
    journey.state("going_down", 2, &[(8775, 0)]).await?;
    journey.log(false).await?;
    journey.place_at(at(32612, 31497, -14)).await?;
    journey
        .step(StepDirection::West, at(32612, 31499, -15))
        .await?;
    journey.state("going_down", 2, &[(8775, 0)]).await?;
    journey.place_at(at(32612, 31497, -15)).await?;
    journey
        .step(StepDirection::West, at(32612, 31499, -14))
        .await?;
    journey.place_at(at(32607, 31491, -14)).await?;
    journey
        .talk(
            nokmir,
            &["hi", "mission", "ring", "grombur", "rerun", "rehal", "bye"],
        )
        .await?;
    journey.state("justice_for_all", 1, &[(8777, 0)]).await?;
    journey.place_at(at(32582, 31402, -8)).await?;
    journey
        .talk(rehal, &["hail emperor", "nokmir", "grombur", "bye"])
        .await?;
    journey.state("justice_for_all", 2, &[(8777, 0)]).await?;
    journey.place_at(at(32609, 31514, -14)).await?;
    journey
        .talk(grombur, &["hi", "nokmir", "rerun", "bye"])
        .await?;
    journey.state("justice_for_all", 3, &[(8777, 0)]).await?;
    journey.place_at(at(32612, 31386, -14)).await?;
    assert_eq!(
        journey.use_object(at(32613, 31386, -14), 2).await?,
        UseDisposition::Committed
    );
    journey.state("justice_for_all", 4, &[(8777, 1)]).await?;
    journey.place_at(at(32582, 31402, -8)).await?;
    journey
        .talk(rehal, &["hail emperor", "nokmir", "bye"])
        .await?;
    journey.state("justice_for_all", 5, &[(8777, 0)]).await?;
    journey.place_at(at(32607, 31491, -14)).await?;
    journey.talk(nokmir, &["hi", "mission", "bye"]).await?;
    journey.state("justice_for_all", 6, &[(8777, 0)]).await?;
    journey.place_at(at(32585, 31501, -14)).await?;
    journey
        .talk(frafnar, &["hi", "mission", "yes", "bye"])
        .await?;
    journey
        .state("sweet_as_chocolate_cake", 1, &[(8019, 1)])
        .await?;
    journey.place_at(at(32532, 31437, -10)).await?;
    journey
        .talk(bolfona, &["hi", "chocolate cake", "yes", "frafnar", "bye"])
        .await?;
    journey
        .state("sweet_as_chocolate_cake", 2, &[(8019, 0)])
        .await?;
    journey.place_at(at(32585, 31501, -14)).await?;
    journey
        .talk(frafnar, &["hi", "mission", "yes", "bye"])
        .await?;
    journey
        .state("sweet_as_chocolate_cake", 3, &[(8019, 0)])
        .await?;
    journey.place_at(at(32609, 31514, -14)).await?;
    journey.talk(grombur, &["hi", "mission", "yes"]).await?;
    journey.state("the_good_guard", 1, &[(8774, 1)]).await?;
    journey.talk(grombur, &["mission", "bye"]).await?;
    journey.state("the_good_guard", 2, &[(8774, 0)]).await?;
    journey.place_at(at(32590, 31407, -15)).await?;
    journey
        .talk(pythius, &["hi", "offer", "yes", "golden mug", "bye"])
        .await?;
    let paid: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE track_key=$1 AND value>0)",
    )
    .bind(format!("{TRACK}pythius_the_rotten"))
    .fetch_one(&harness.pool)
    .await?;
    assert!(paid, "the golden mug must open the timed dungeon");
    journey
        .state("the_good_guard", 2, &[(2903, 0), (9019, 0)])
        .await?;
    journey.place_at(at(32598, 31403, -15)).await?;
    journey
        .step(StepDirection::North, at(32601, 31397, -14))
        .await?;

    journey.place_at(at(32551, 31378, -15)).await?;
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, encode_item_use_intent,
    };
    let pick = journey
        .owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i3456", "definition-r1"))
        .ok_or("native pick")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(pick),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(
            journey.target(at(32551, 31379, -15), 2)?,
        )),
        use_with_position: None,
    })
    .map_err(debug)?;
    assert_eq!(
        decode_use_result(&journey.send(2, payload).await?).map_err(debug)?,
        UseDisposition::Committed
    );
    journey
        .state("pythius_portal_open", 1, &[(3456, 1), (9019, 0)])
        .await?;
    journey.place_at(at(32551, 31377, -15)).await?;
    journey
        .step(StepDirection::North, at(32560, 31404, -15))
        .await?;
    journey.state("pythius_fight", 1, &[(9019, 0)]).await?;
    journey.defeat_pythius().await?;
    journey.place_at(at(32560, 31405, -15)).await?;
    journey
        .step(StepDirection::South, at(32577, 31403, -15))
        .await?;
    journey.state("pythius_fight", 2, &[(9019, 0)]).await?;
    journey.place_at(at(32581, 31405, -15)).await?;
    let target = journey.target(at(32582, 31405, -15), 2)?;
    let first = journey.admitted.continuity.next_command_id;
    let replay = super::quest_callbacks_fixtures::wire_use_targets_retained(
        journey.owner,
        journey.admitted,
        &[(first, &target), (first, &target)],
    )
    .await?;
    assert_eq!(replay[0].1, UseDisposition::Committed);
    assert_eq!(
        replay[1],
        (
            oteryn_protocol_oteryn::CommandStatus::DuplicateReplay,
            UseDisposition::Committed
        )
    );
    journey.state("firewalker_boots", 1, &[(9019, 1)]).await?;
    journey.owner.admit_quest_session(journey.admitted).await;
    assert_ne!(
        journey.use_object(at(32582, 31405, -15), 2).await?,
        UseDisposition::Committed
    );
    journey.state("firewalker_boots", 1, &[(9019, 1)]).await?;
    journey.equip_boots().await?;
    journey.state("firewalker_boots", 1, &[(9019, 1)]).await?;
    journey.log(true).await
}

impl Journey<'_, '_, '_, '_> {
    async fn equip_boots(&mut self) -> TestResult {
        use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
        use oteryn_protocol_oteryn::{self as protocol, MessageType, item_view as item};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let definition = self
            .owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .and_then(|native| native.item_index())
            .and_then(|index| {
                index.definition_ref("Item", "oteryn:item.tibia.i9019", "definition-r1")
            })
            .ok_or("Firewalker Boots definition")?;
        let initial = *self.admitted;
        let command_id = initial.continuity.next_command_id;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let peer = async move {
            let mut snapshot = Vec::new();
            loop {
                let len = usize::try_from(client.read_u32().await?)?;
                assert!(len <= 1 << 20);
                let mut frame = vec![0; len];
                client.read_exact(&mut frame).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                match envelope.message_type() {
                    MessageType::SnapshotChunk => {
                        let (_, bytes) =
                            protocol::decode_snapshot_chunk_framing(envelope.payload())
                                .map_err(debug)?;
                        snapshot.extend_from_slice(bytes);
                    }
                    MessageType::SnapshotCommit => {
                        let domains = protocol::decode_snapshot_body(&snapshot).map_err(debug)?;
                        let domain = domains
                            .iter()
                            .find(|domain| {
                                domain.domain_id == item::STATE_DOMAIN_CHARACTER_INVENTORY
                            })
                            .ok_or("reconnected inventory")?;
                        let inventory =
                            item::decode_character_inventory_with_equip_drop(domain.payload, true)
                                .map_err(debug)?;
                        let boots = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == definition)
                            .ok_or("earned Firewalker Boots in backpack")?;
                        let payload = item::encode_item_move_intent_with_equip_drop(
                            &item::ItemMoveIntent {
                                source: boots.handle,
                                destination: item::ItemMoveDestination::Equipment(
                                    item::EquipmentSlot::Feet,
                                ),
                            },
                            true,
                        )
                        .map_err(debug)?;
                        let frame = protocol::encode_client_command(
                            initial.continuity.connection_generation,
                            &protocol::ClientCommandValue {
                                command_id,
                                command_type: item::COMMAND_TYPE_ITEM_MOVE_INTENT,
                                payload: &payload,
                            },
                        )
                        .map_err(debug)?;
                        client.write_u32(u32::try_from(frame.len())?).await?;
                        client.write_all(&frame).await?;
                    }
                    MessageType::CommandResult => {
                        let result =
                            protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                        assert_eq!(result.command_id, command_id);
                        assert_eq!(
                            item::decode_item_move_result(result.payload).map_err(debug)?,
                            item::ItemMoveOutcome::Moved
                        );
                        client.shutdown().await?;
                        return Ok::<(), Box<dyn std::error::Error>>(());
                    }
                    _ => continue,
                }
            }
        };
        let (end, result) = tokio::time::timeout(
            Duration::from_secs(30),
            super::fixture::join_two(
                serve_admitted(&mut server, initial, self.owner, IDLE_LIVENESS),
                peer,
            ),
        )
        .await?;
        result?;
        let ConnectionEnd::AdmittedThenDisconnected(next) = end else {
            return Err(format!("Beregar MOVE reconnect: {end:?}").into());
        };
        *self.admitted = next;
        let equipped: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_character_equipment_slots e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i9019')",
        ).fetch_one(&self.harness.pool).await?;
        assert!(equipped);
        Ok(())
    }
}
