//! Generic world puzzle mechanisms through the admitted world/character harness.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};

fn scenario(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    eprintln!("POSTGRES_CASE_STARTED {name}");
    super::qengine_rewards::scenario(name)
}
#[test]
fn l6_world_puzzle_readonly_requirements_guard_native_state() -> TestResult {
    scenario("qengine-rewards-questpc4-puzzle-predicate")
}
#[test]
fn l6_world_puzzle_readonly_requirements_refuse_wrong_state() -> TestResult {
    scenario("qengine-rewards-questpc4-puzzle-predicate-refuse")
}
#[test]
fn l6_world_puzzle_presence_is_owned_and_creature_effects_are_value_free() -> TestResult {
    scenario("qengine-rewards-questpc4-puzzle-presence")
}
#[test]
fn l6_world_puzzle_ladder_use_commits_sealed_ascent() -> TestResult {
    scenario("qengine-rewards-questpc4-puzzle-ladder")
}

pub(super) fn presence_fixture(mut callback: Callback, trigger: Position) -> Callback {
    callback.key = "oteryn:quest-callback/l6-presence-fixture".into();
    callback.selector.position = Some(trigger);
    callback.selector.unique = None;
    callback.selector.action = None;
    callback.selector.appearance = Some(431);
    callback.actor_kind = oteryn_gameplay_quests::quest::world_callbacks::ActorKind::Monster;
    callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn;
    callback.lever = None;
    callback.reversible = false;
    callback.passages.clear();
    callback.storage.clear();
    callback.offerings.clear();
    callback.teleport = None;
    callback.effects.transform = Some(430);
    callback.effects.item_presence = Some(
        oteryn_gameplay_quests::quest::world_callbacks::ItemPresence {
            position: Position {
                x: trigger.x - 1,
                ..trigger
            },
            appearance: 1948,
        },
    );
    callback
}

pub(super) fn predicate_fixture(
    mut callback: Callback,
    trigger: Position,
    refuse: bool,
) -> Callback {
    callback.key = "oteryn:quest-callback/l6-predicate-fixture".into();
    callback.selector.position = Some(trigger);
    callback.selector.unique = None;
    callback.selector.action = None;
    callback.selector.appearance = Some(2772);
    callback.lever = Some([2772, 2773]);
    callback.reversible = true;
    callback.passages.clear();
    callback.storage.clear();
    callback.offerings.clear();
    callback.teleport = None;
    callback.world_requirements = vec![
        oteryn_gameplay_quests::quest::world_callbacks::WorldRequirement { ground: None,
            position: trigger,
            appearance: if refuse { 2773 } else { 2772 },
            ground_stack: None,
        },
    ];
    callback
}

pub(super) fn ladder_fixture(mut callback: Callback, trigger: Position) -> Callback {
    callback.key = "oteryn:quest-callback/l6-ladder-fixture".into();
    callback.selector.position = Some(trigger);
    callback.selector.unique = None;
    callback.selector.action = None;
    callback.selector.appearance = Some(1948);
    callback.lever = None;
    callback.reversible = false;
    callback.passages.clear();
    callback.storage.clear();
    callback.offerings.clear();
    callback.item_on_target = None;
    // The world fixture includes the native-rule upstairs landing, not an active teleport.
    callback.teleport = Some(Position {
        x: trigger.x,
        y: trigger.y + 1,
        floor: trigger.floor + 1,
    });
    callback
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    if name.ends_with("-srv7-m02") {
        return super::srv7_m02_ground_stack::exercise(owner, admitted, h).await;
    }

    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY {name} version={version}");
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key.ends_with("/behemoth"))
        .ok_or("template")?;
    let value = serde_json::to_value(callback)?;
    if name.contains("-predicate") {
        use oteryn_protocol_oteryn::world_object::*;
        let map = owner.bundle_map().ok_or("map")?;
        let key =
            oteryn_world_bundle::bundle::placement_key(-15, 33294, 31677, 1).ok_or("source")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
        let payload = encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: 0,
        })
        .map_err(debug)?;
        let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
        let refused = name.ends_with("-refuse");
        assert_eq!(
            decode_use_result(&reply).map_err(debug)?,
            if refused {
                UseDisposition::Rejected
            } else {
                UseDisposition::Committed
            }
        );
        assert_eq!(
            owner.bundle_map().ok_or("map")?.facts.object_revision(key),
            u64::from(!refused)
        );
        let after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(
            before, after,
            "readonly map puzzle must not write quest receipts"
        );
        if !refused {
            // Closing remains possible after the opening predicate changes with the lever.
            // A distinct client interaction observes the existing USE cadence; do not
            // weaken or clear the gate to exercise the changed opening condition.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("puzzle clock")?;
            let payload = encode_use_intent(&WorldObjectTarget {
                placement,
                expected_revision: 1,
            })
            .map_err(debug)?;
            let reply = super::quest_world_callbacks::wire(owner, admitted, 2, 2, payload).await?;
            assert_eq!(
                decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed
            );
            assert_eq!(
                owner.bundle_map().ok_or("map")?.facts.object_revision(key),
                2
            );
        }
    } else if name.ends_with("-presence") {
        use oteryn_gameplay_quests::quest::world_callbacks::Trigger;
        let trigger = Position {
            x: 33294,
            y: 31677,
            floor: -15,
        };
        let enter = presence_fixture(serde_json::from_value(value)?, trigger);
        let mut exit = enter.clone();
        exit.key.push_str("-exit");
        exit.trigger = Trigger::StepOut;
        exit.effects.transform = Some(431);
        exit.selector.appearance = Some(431);
        let registry = vec![enter, exit];
        let native = owner
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or("native policies")?;
        let mut runtime = owner.runtime.lock().await;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let map = owner.bundle_map().ok_or("map")?;
        let position = crate::map::overlay::TilePos {
            x: trigger.x - 1,
            y: trigger.y,
            floor: trigger.floor,
        };
        let settle = |runtime: &crate::foundation::ChannelRuntimeV1| {
            crate::gameplay_transport::quest_callbacks::monsters::settle_monster_puzzles(
                runtime, map, &registry,
            );
        };
        settle(&runtime);
        assert!(
            map.facts.movement_items(position).is_empty(),
            "players cannot press a monster plate"
        );
        let pin = runtime.content_pin().clone();
        let monster = runtime
            .realize_native_qualification_spawn(
                pin,
                "oteryn:creature.rat",
                crate::foundation::MovementLocalPosition {
                    x: 33294,
                    y: 31677,
                    floor: 15,
                },
            )
            .map_err(debug)?;
        settle(&runtime);
        let entries = map.facts.movement_items(position);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].facts.appearance_id, 1948);
        assert!(!entries[0].facts.pickupable);
        let original = entries[0].source;
        settle(&runtime);
        assert_eq!(
            map.facts.movement_items(position)[0].source,
            original,
            "reconciliation is idempotent"
        );
        runtime.remove_test_actor(monster.actor).map_err(debug)?;
        settle(&runtime);
        assert!(
            map.facts.movement_items(position).is_empty(),
            "despawn closes the owned ladder"
        );
        // Closing never deletes another map entry or leaves the old handle valid.
        if let crate::map::view::EntrySource::Added(id) = original {
            assert!(map.facts.movement_item(id.get()).is_none());
        } else {
            return Err("presence did not allocate a map identity".into());
        }
        let pin = runtime.content_pin().clone();
        runtime
            .realize_native_qualification_spawn_for_source(
                u16::MAX - 1,
                pin,
                "oteryn:creature.rat",
                crate::foundation::MovementLocalPosition {
                    x: 33294,
                    y: 31677,
                    floor: 15,
                },
            )
            .map_err(debug)?;
        settle(&runtime);
        assert_ne!(map.facts.movement_items(position)[0].source, original);
        drop(runtime);
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[4, 6, 11, 15, 18],
            )
            .ok_or("ladder handle capabilities")?;
        use_added_ladder(owner, admitted, position).await?;
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        assert_eq!((at.x, at.y, at.floor), (33293, 31678, 14));
    } else {
        use oteryn_protocol_oteryn::world_object::*;
        let mut placement = owner
            .bundle_map()
            .ok_or("map")?
            .overlay
            .base()
            .digest()
            .to_vec();
        placement.extend_from_slice(
            &oteryn_world_bundle::bundle::placement_key(-15, 33294, 31677, 1)
                .ok_or("ladder source")?
                .to_be_bytes(),
        );
        let payload = encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: 0,
        })
        .map_err(debug)?;
        let reply = super::quest_world_callbacks::wire(owner, admitted, 1, 2, payload).await?;
        assert_eq!(
            decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed
        );
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
            .map_err(debug)?
            .position();
        assert_eq!((at.x, at.y, at.floor), (33294, 31678, 14));
    }
    Ok(())
}

pub(super) async fn use_added_ladder(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    position: crate::map::overlay::TilePos,
) -> TestResult {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn::world_map::{
        MapOrigin, STATE_DOMAIN_WORLD_MAP_VIEW, decode_world_map_snapshot,
    };
    use oteryn_protocol_oteryn::world_object::{UseDisposition, decode_use_result};
    use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget, encode_item_use_intent};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    async fn frame(client: &mut tokio::io::DuplexStream) -> TestResult<Vec<u8>> {
        let length = client.read_u32().await?;
        if length > 1 << 20 {
            return Err("unbounded puzzle test frame".into());
        }
        let mut bytes = vec![0; usize::try_from(length)?];
        client.read_exact(&mut bytes).await?;
        Ok(bytes)
    }
    let first = admitted.continuity.next_command_id;
    let initial = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, initial, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let receiving = async move {
        let mut body = Vec::new();
        loop {
            let bytes = frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, data) =
                    oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())
                        .map_err(debug)?;
                body.extend_from_slice(data);
            } else if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let domains = oteryn_protocol_oteryn::decode_snapshot_body(&body).map_err(debug)?;
        let domain = domains
            .iter()
            .find(|d| d.domain_id == STATE_DOMAIN_WORLD_MAP_VIEW)
            .ok_or("map snapshot")?;
        let map = decode_world_map_snapshot(domain.payload).map_err(debug)?;
        let tile = map
            .tiles
            .iter()
            .find(|tile| {
                tile.position.x == i32::from(position.x)
                    && tile.position.y == i32::from(position.y)
                    && tile.position.floor == i16::from(position.floor)
            })
            .ok_or("ladder tile")?;
        let handle = tile
            .items
            .iter()
            .find_map(|item| match item.origin {
                MapOrigin::Handle(handle) => Some(handle),
                _ => None,
            })
            .ok_or("advertised ladder handle")?;
        let payload = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(handle),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(debug)?;
        let bytes = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id: first,
                command_type: 2,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        client.write_u32(u32::try_from(bytes.len())?).await?;
        client.write_all(&bytes).await?;
        loop {
            let bytes = frame(&mut client).await?;
            let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                    .map_err(debug)?;
                assert_eq!(result.command_id, first);
                assert_eq!(
                    decode_use_result(result.payload).map_err(debug)?,
                    UseDisposition::Committed
                );
                client.shutdown().await?;
                break;
            }
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let reader = tokio::spawn(async move { receiving.await.map_err(|error| error.to_string()) });
    let end = serving.await;
    let result: TestResult = reader.await?.map_err(Into::into);
    if let ConnectionEnd::AdmittedThenDisconnected(next)
    | ConnectionEnd::AdmittedThenClosed(next, _) = end
    {
        *admitted = next;
    }
    result
}
