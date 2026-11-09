//! Literal USE against retained palace B3 tiles and the actual authored bindings.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Position},
};

fn stage(name: &str) -> TestResult<&str> {
    name.rsplit('-').next().ok_or_else(|| "Asura stage".into())
}
fn bindings() -> TestResult<Bindings> {
    let b = Bindings::parse(include_bytes!(
        "../../../../../../content/quests/authored-bindings/engenc-asura-palace.json"
    ))?;
    b.verify_embedded_recipe_evidence()?;
    Ok(b)
}
pub(super) fn callback(name: &str) -> TestResult<Callback> {
    let key = format!("oteryn:quest-callback/engenc/asura/{}/entry", stage(name)?);
    bindings()?
        .world_callbacks()
        .find(|c| c.key == key)
        .cloned()
        .ok_or_else(|| "Asura entry".into())
}
pub(super) fn start(name: &str) -> TestResult<Position> {
    Ok(callback(name)?
        .effects
        .encounter
        .as_ref()
        .ok_or("encounter")?
        .players[0]
        .from)
}
pub(super) fn catalogue(name: &str) -> TestResult<QuestStateCatalogue> {
    let mut b = bindings()?;
    // Isolate each real caller at its authored predecessor, with no fabricated receipts.
    let predecessor = match stage(name)? {
        "s2" => "oteryn:quest-progress/authored/asura_palace_quest/s1",
        "s3" => "oteryn:quest-progress/engenc/asura/s2/opened",
        "s4" => "oteryn:quest-progress/engenc/asura/s3/opened",
        _ => return Err("Asura stage".into()),
    };
    b.quests[0]
        .tracks
        .iter_mut()
        .find(|t| t.key == predecessor)
        .ok_or("predecessor")?
        .initial = 1;
    Ok(b.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}
pub(super) fn world(
    native: &NativeGameplayState,
    name: &str,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene_with_appearances};
    let cb = callback(name)?;
    let at = cb.selector.position.ok_or("entry position")?;
    let staging = start(name)?;
    let (bytes, region, bounds, ids) = match stage(name)? {
        "s2" => (
            include_bytes!("../../../../../../content/world/placements/region-z03-x128-y127.b3")
                .as_slice(),
            (3, 128, 127),
            (32958, 32674, 32961, 32677),
            vec![923, 21973],
        ),
        "s3" => (
            include_bytes!("../../../../../../content/world/placements/region-z02-x128-y127.b3")
                .as_slice(),
            (2, 128, 127),
            (32952, 32676, 32962, 32683),
            vec![923],
        ),
        "s4" => (
            include_bytes!("../../../../../../content/world/placements/region-z02-x128-y127.b3")
                .as_slice(),
            (2, 128, 127),
            (32942, 32675, 32950, 32683),
            vec![923],
        ),
        _ => return Err("Asura stage".into()),
    };
    let mut crops = vec![NativeCrop {
        bytes,
        region,
        bounds,
        start: (staging.x, staging.y),
        expected: vec![ExpectedTile {
            position: (at.x, at.y),
            ids,
            counted_entry: None,
        }],
    }];
    if stage(name)? == "s2" {
        crops.push(NativeCrop {
            bytes: include_bytes!(
                "../../../../../../content/world/placements/region-z04-x128-y127.b3"
            ),
            region: (4, 128, 127),
            bounds: (32957, 32673, 32965, 32680),
            start: (staging.x, staging.y),
            expected: vec![ExpectedTile {
                position: (32963, 32677),
                ids: vec![936, 5736],
                counted_entry: None,
            }],
        });
    }
    let world = native_scene_with_appearances(native, crops, &[], &[1675, 5737])?;
    for defender in &cb.effects.encounter.as_ref().ok_or("encounter")?.monsters {
        assert!(
            world.enterable(crate::map::overlay::TilePos {
                x: defender.position.x,
                y: defender.position.y,
                floor: defender.position.floor,
            }),
            "{} defender tile must retain walkable native geometry",
            defender.creature
        );
    }
    Ok(world)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let cb = callback(name)?;
    let encounter = cb.effects.encounter.as_ref().ok_or("encounter")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .bind_attacker_lease(
                admitted.runtime_actor.ok_or("actor")?,
                admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    owner
        .refresh_quest_session(admitted.game_session_id, true)
        .await;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    owner
        .runtime
        .lock()
        .await
        .install_companion_policies(native.policies().clone())
        .map_err(debug)?;
    let selector = cb.selector.native.as_ref().ok_or("native selector")?;
    let key = oteryn_world_bundle::bundle::placement_key(
        selector.position.floor,
        selector.position.x,
        selector.position.y,
        selector.ordinal,
    )
    .ok_or("placement")?;
    let map = owner.bundle_map().ok_or("map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(
        &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        },
    )
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let reply =
        super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone()).await;
    if reply.is_err() {
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(&cb.key)
        .fetch_one(&h.pool)
        .await?;
        eprintln!(
            "M30_ASURA_DISPATCH_UNRESOLVED receipts={receipts} creatures={}",
            owner
                .runtime
                .lock()
                .await
                .visible_entities()
                .creatures
                .len()
        );
    }
    let reply = reply?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    let roster = {
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(encounter.players[0].to)
        );
        let creatures = runtime.visible_entities().creatures;
        assert_eq!(creatures.len(), 2);
        for creature in &creatures {
            let snapshot = runtime.companion_snapshot(creature.actor).map_err(debug)?;
            assert_eq!(
                snapshot.state.policy.definition_key,
                encounter.monsters[0].creature
            );
        }
        creatures.iter().map(|c| c.actor).collect::<Vec<_>>()
    };
    let reply = super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
        oteryn_protocol_oteryn::world_object::UseDisposition::Committed
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .iter()
            .map(|c| c.actor)
            .collect::<Vec<_>>(),
        roster
    );
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(&cb.key)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(count, 1);
    eprintln!("M30_ASURA_DISPATCH_ADMISSION_EXECUTED {}", stage(name)?);
    Ok(())
}

fn run(stage: &'static str, trigger: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(stage, trigger, false).map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Asura dispatch thread failed")?
        .map_err(Into::into)
}
#[test]
fn engenc_postgres_native_destroyer_chamber_admission() -> TestResult {
    run(
        "engenc-asura-native-s2",
        Position {
            x: 32959,
            y: 32675,
            floor: -3,
        },
    )
}
#[test]
fn engenc_postgres_native_fire_chamber_admission() -> TestResult {
    run(
        "engenc-asura-native-s3",
        Position {
            x: 32960,
            y: 32679,
            floor: -2,
        },
    )
}
#[test]
fn engenc_postgres_native_energy_chamber_admission() -> TestResult {
    run(
        "engenc-asura-native-s4",
        Position {
            x: 32948,
            y: 32679,
            floor: -2,
        },
    )
}
