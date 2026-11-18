//! Demon Oak actions and movement on the retained production map crop.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    demon_oak as rules,
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::{
    world_object::{
        MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
        encode_item_use_intent,
    },
    world_spatial::{StepDirection, StepDisposition, decode_step_result, encode_step_intent},
};

pub(super) const START: Position = Position {
    x: 32716,
    y: 32348,
    floor: -7,
};
const BIRD: Position = Position { y: 32349, ..START };

#[test]
fn areadmg_demon_oak_final_cut_summons_demon_then_finishes_after_clear() -> TestResult {
    run("areadmg-oak-final")
}

#[test]
fn areadmg_demon_oak_fifth_cut_spawns_its_wave_and_harms_the_cutter() -> TestResult {
    run("areadmg-oak-fifth")
}

#[test]
fn areadmg_demon_oak_step_in_rolls_earth_damage() -> TestResult {
    run("areadmg-oak-area")
}

fn run(name: &'static str) -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, BIRD, false)
    })
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTeleport, ExpectedTile, NativeCrop, native_scene};
    native_scene(
        native,
        vec![NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x127-y126.b3"
            )),
            region: (7, 127, 126),
            bounds: (32710, 32340, 32722, 32355),
            start: (START.x, START.y),
            expected: vec![
                ExpectedTile {
                    position: (BIRD.x, BIRD.y),
                    ids: vec![918, 914],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (START.x, START.y),
                    ids: vec![918],
                    counted_entry: None,
                },
            ],
        }],
        &[ExpectedTeleport {
            position: (32714, 32345, 7),
            ordinal: 3,
            destination: (32716, 32340, 7),
        }],
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
    name: &str,
) -> TestResult {
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
    if name.ends_with("area") {
        return area(owner, admitted).await;
    }
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: START,
            item: rules::AXE.into(),
            count: 1,
        },
        951,
        true,
    )
    .await?;
    let final_cut = name.ends_with("final");
    let track_values: [i64; 4] = if final_cut {
        [10, 11, 11, 11]
    } else {
        [5, 0, 0, 0]
    };
    for (track, value) in rules::CUT_TRACKS.into_iter().zip(track_values) {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(track).bind(rules::QUEST).bind(value)
            .execute(&harness.pool).await?;
    }
    let map = owner.bundle_map().ok_or("map")?;
    let placement = oteryn_world_bundle::bundle::placement_key(BIRD.floor, BIRD.x, BIRD.y, 1)
        .ok_or("weak point")?;
    let mut target = map.overlay.base().digest().to_vec();
    target.extend_from_slice(&placement.to_be_bytes());
    let axe = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", rules::AXE, "definition-r1"))
        .ok_or("axe reference")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(axe),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement: target,
            expected_revision: map.facts.object_revision(placement),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let before = health(owner, admitted).await?;
    assert_eq!(
        use_axe(owner, admitted, &payload, axe).await?,
        UseDisposition::Committed
    );
    let after = health(owner, admitted).await?;
    assert!((170..=210).contains(&(before - after)), "earth retaliation");
    let (mut monsters, actors) = {
        let runtime = owner.runtime.lock().await;
        let actors = runtime.current_live_creature_census().map_err(debug)?;
        let monsters = actors
            .iter()
            .map(|actor| {
                runtime
                    .current_live_creature_identity(*actor)
                    .map(|key| String::from_utf8_lossy(key).into_owned())
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(debug)?;
        (monsters, actors)
    };
    monsters.sort();
    let tracks = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        tracks.tracks().get(rules::CUT_TRACKS[0]),
        Some(&(track_values[0] + 1))
    );
    assert_ne!(tracks.tracks().get(rules::DONE), Some(&1));
    if final_cut {
        assert_eq!(monsters, ["oteryn:creature.demon"]);
        assert_eq!(
            use_axe(owner, admitted, &payload, axe).await?,
            UseDisposition::NotPossible
        );
        {
            let mut runtime = owner.runtime.lock().await;
            for monster in actors {
                runtime.remove_encounter_actor(monster).map_err(debug)?;
            }
        }
        assert_eq!(
            use_axe(owner, admitted, &payload, axe).await?,
            UseDisposition::Committed
        );
        let finished = owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(finished.tracks().get(rules::DONE), Some(&1));
        assert_eq!(finished.tracks().get(rules::PROGRESS), Some(&3));
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
                .map_err(debug)?
                .position()
                .y,
            32340
        );
    } else {
        assert_eq!(
            monsters,
            [
                "oteryn:creature.blightwalker",
                "oteryn:creature.braindeath",
                "oteryn:creature.demon",
                "oteryn:creature.spectre",
            ]
        );
    }
    Ok(())
}

async fn use_axe(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    payload: &[u8],
    axe: std::num::NonZeroU32,
) -> TestResult<UseDisposition> {
    let command = admitted.continuity.next_command_id;
    let (next, result, _) =
        super::item_craft::exchange(owner, *admitted, command, payload, Some(axe)).await?;
    *admitted = next;
    Ok(result)
}

async fn health(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<u32> {
    let runtime = owner.runtime.lock().await;
    Ok(owner
        .states
        .spell_states
        .lock()
        .await
        .get(
            &runtime,
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
        )
        .ok_or("vitals")?
        .vitals()
        .health)
}

async fn area(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let mut unchanged = false;
    let mut struck = false;
    for command in 1..=256 {
        let before = health(owner, admitted).await?;
        let expected = next_area_damage(owner, admitted).await?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("clock")?;
        let direction = if command % 2 == 1 {
            StepDirection::West
        } else {
            StepDirection::East
        };
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            command,
            1,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        assert_eq!(before - health(owner, admitted).await?, expected);
        unchanged |= expected == 0;
        struck |= expected > 0;
        if unchanged && struck {
            return Ok(());
        }
    }
    Err("the fixed scenario never observed both StepIn outcomes".into())
}

async fn next_area_damage(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<u32> {
    use oteryn_simulation_determinism::{
        DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
    };
    use sha2::{Digest, Sha256};

    let actor = admitted.runtime_actor.ok_or("actor")?;
    let runtime = owner.runtime.lock().await;
    let revision = runtime
        .read_actor_position(actor)
        .map_err(debug)?
        .revision()
        + 1;
    let occurrence = Sha256::new()
        .chain_update(actor.placement_identity())
        .chain_update(admitted.game_session_id.as_bytes())
        .chain_update(revision.to_be_bytes())
        .finalize();
    let occurrence = DecisionOccurrenceId::from_bytes(occurrence[..16].try_into()?);
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let draw = |index| {
        deterministic_decision_u64(&root, occurrence, "demon_oak.area", index).map_err(debug)
    };
    if draw(0)? % 24 != 0 {
        return Ok(0);
    }
    oteryn_gameplay_domain::domain::source_normal_draw::source_normal_draw(draw(1)?, 270, 310)
        .ok_or_else(|| "source Earth damage".into())
}
