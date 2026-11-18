//! Native USE and death/clock dispatch for the Grave Danger ritual.
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult,
    fixture::{Harness, debug},
};
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, EncounterPhase, Position};

const KNIGHTS: [&str; 4] = [
    "oteryn:creature.magnor_mournbringer",
    "oteryn:creature.nargol_the_impaler",
    "oteryn:creature.rewar_the_bloody",
    "oteryn:creature.the_red_knight",
];

#[test]
fn gravedg_zelos_two_players_regeneration_and_timeout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::engenc2_cults::phase_scenario(super::engenc2_cults::PhaseScenario::Zelos)
}

pub(super) fn configure(callback: &mut Callback) -> TestResult {
    callback.key = "oteryn:quest-callback/grave-danger/zelos-ritual".into();
    callback.storage.clear();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(EncounterPhase::GraveDangerZelos {
        exit: Position {
            x: 103,
            y: 100,
            floor: -7,
        },
        north_gate: Position {
            x: 116,
            y: 116,
            floor: -7,
        },
        passages: None,
        north_arrival: Position {
            x: 115,
            y: 114,
            floor: -7,
        },
    });
    encounter.minimum_level = 1;
    let entry = encounter.players.first().ok_or("party arrival")?.to;
    encounter.players = (0..10)
        .map(
            |i| oteryn_gameplay_quests::quest::world_callbacks::EncounterPlayer {
                from: Position {
                    x: 100 + i / 5,
                    y: 100 + i % 5,
                    floor: -7,
                },
                to: entry,
            },
        )
        .collect();
    encounter.monsters.truncate(4);
    for (monster, name) in encounter.monsters.iter_mut().zip(KNIGHTS) {
        monster.creature = name.into();
    }
    encounter.monsters.push(
        oteryn_gameplay_quests::quest::world_callbacks::EncounterMonster {
            creature: "oteryn:creature.king_zelos".into(),
            position: Position {
                x: 115,
                y: 113,
                floor: -7,
            },
        },
    );
    Ok(())
}

pub(super) async fn admit_partner(
    h: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: &AdmittedSession,
) -> TestResult<AdmittedSession> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    use crate::world_party_runtime_postgres_cases::{
        FreshPlayerOwners, committed_fresh_player_owned,
    };
    let current = committed_fresh_player_owned(
        &FreshPlayerOwners {
            root: h.root.clone(),
            node: h.node.clone(),
            recovery: &h.recovery,
        },
        // The first peer opens the independent platform source once per harness.
        60,
        62,
        70,
        now,
    )
    .await
    .map_err(|error| format!("second player's durable admission: {error}"))?;
    let session = current.current_game_session_id();
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                actor,
                MovementLocalPosition {
                    x: 100,
                    y: 101,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        actor
    };
    let mut admitted = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        controller: None,
        item_fence: None,
        ..*leader
    };
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize_player(owner, actor, session).await;
    owner.admit_quest_session(&admitted).await;
    assert!(owner.refresh_source_party_presence(actor, session).await);
    Ok(admitted)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    partner: ExactActorRef,
) -> TestResult {
    let find = |runtime: &crate::foundation::ChannelRuntimeV1, name: &str| {
        runtime
            .current_live_creature_census()
            .map(|actors| {
                actors
                    .into_iter()
                    .find(|actor| runtime.matches_live_creature_identity(*actor, name.as_bytes()))
            })
            .map_err(debug)
    };
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let nargol = {
        let runtime = owner.runtime.lock().await;
        for member in [player, partner] {
            let at = runtime
                .read_actor_position(member)
                .map_err(debug)?
                .position();
            assert_eq!((at.x, at.y, at.floor), (116, 117, 7));
        }
        find(&runtime, KNIGHTS[1])?.ok_or("Nargol spawned by native USE")?
    };
    use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
    let (step, _) = owner
        .ordinary_paced_step(
            player,
            admitted.game_session_id,
            StepDirection::North,
            None,
            None,
        )
        .await;
    assert_eq!(step.disposition, StepDisposition::Moved);
    {
        let runtime = owner.runtime.lock().await;
        let at = runtime
            .read_actor_position(player)
            .map_err(debug)?
            .position();
        assert_eq!(
            (at.x, at.y, at.floor),
            (116, 117, 7),
            "the locked vortex returns the entrant"
        );
    }
    super::engenc2_cults::kill(owner, admitted, nargol, KNIGHTS[1], 101).await?;
    let now = owner.owner_now().get();
    tick_at(owner, now).await;
    let mass = "oteryn:creature.regenerating_mass";
    {
        let runtime = owner.runtime.lock().await;
        assert!(find(&runtime, mass)?.is_some(), "mass replaces Nargol");
        assert!(
            find(&runtime, "oteryn:creature.fetter")?.is_some(),
            "Rewar room starts Fetter waves"
        );
    }
    tick_at(owner, now + 29_999_999).await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            find(&runtime, mass)?.is_some(),
            "mass survives until the deadline"
        );
        for add in [
            "oteryn:creature.vampiric_blood",
            "oteryn:creature.risen_soldier",
        ] {
            assert!(find(&runtime, add)?.is_some(), "ritual replenishes {add}");
        }
    }
    let blood = {
        let runtime = owner.runtime.lock().await;
        find(&runtime, "oteryn:creature.vampiric_blood")?.ok_or("vampiric blood")?
    };
    let red_before = {
        let runtime = owner.runtime.lock().await;
        let red = find(&runtime, KNIGHTS[3])?.ok_or("red knight")?;
        runtime.companion_snapshot(red).map_err(debug)?.health
    };
    super::engenc2_cults::kill(
        owner,
        admitted,
        blood,
        "oteryn:creature.vampiric_blood",
        102,
    )
    .await?;
    {
        let runtime = owner.runtime.lock().await;
        let red = find(&runtime, KNIGHTS[3])?.ok_or("red survives the first drowning hit")?;
        let red_after = runtime.companion_snapshot(red).map_err(debug)?.health;
        assert!((20_000..=25_000).contains(&(red_before - red_after)));
    }
    tick_at(owner, now + 30_000_000).await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            find(&runtime, mass)?.is_none(),
            "mass expires at 30 seconds"
        );
        assert_ne!(
            find(&runtime, KNIGHTS[1])?.ok_or("regenerated Nargol")?,
            nargol
        );
        let king = find(&runtime, "oteryn:creature.king_zelos")?.ok_or("admitted king")?;
        assert!(
            runtime
                .encounter_hooks()
                .is_some_and(|hooks| hooks.contains(king))
        );
    }
    let magnor = {
        let runtime = owner.runtime.lock().await;
        find(&runtime, KNIGHTS[0])?.ok_or("admitted Magnor")?
    };
    super::engenc2_cults::kill(owner, admitted, magnor, KNIGHTS[0], 103).await?;
    tick_at(owner, now + 31_000_000).await;
    let shard_key = "oteryn:creature.shard_of_magnor";
    let shard = {
        let runtime = owner.runtime.lock().await;
        let shards: Vec<_> = runtime
            .current_live_creature_census()
            .map_err(debug)?
            .into_iter()
            .filter(|&actor| runtime.matches_live_creature_identity(actor, shard_key.as_bytes()))
            .collect();
        assert_eq!(shards.len(), 4, "Magnor splits into four linked shards");
        shards[0]
    };
    super::engenc2_cults::kill(owner, admitted, shard, shard_key, 104).await?;
    tick_at(owner, now + 32_000_000).await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            find(&runtime, shard_key)?.is_none(),
            "one hit kills the linked shards"
        );
    }
    tick_at(owner, now + 24 * 60 * 1_000_000).await;
    let runtime = owner.runtime.lock().await;
    for knight in KNIGHTS.into_iter().chain([
        "oteryn:creature.king_zelos",
        "oteryn:creature.rewar_the_bloody_inv",
    ]) {
        assert!(
            find(&runtime, knight)?.is_none(),
            "timeout cleans up {knight}"
        );
    }
    for member in [player, partner] {
        let exit = runtime
            .read_actor_position(member)
            .map_err(debug)?
            .position();
        assert_eq!((exit.x, exit.y, exit.floor), (103, 100, 7));
    }
    Ok(())
}

pub(super) async fn tick_at(owner: &ComposedFreshAdmission<'_, '_, '_>, now: u64) {
    use std::sync::atomic::Ordering;
    owner
        .states
        .test_owner_time
        .store(now + 1, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
}

/// A single-sector qualified room shared by native phase lever tests.
pub(super) fn lever_world(
    callback: &Callback,
    trigger: Position,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::fixture::id;
    use oteryn_world_bundle::{bundle, sector};

    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let lever = callback.lever.ok_or("phase lever pair")?;
    for (i, appearance) in lever.into_iter().enumerate() {
        data.manifest.palette.push(bundle::PaletteEntry {
            id: u32::try_from(i + 1)?,
            key: format!("oteryn:terrain.tibia.i{appearance}"),
            family: bundle::Family::Terrain,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
    }

    let first = 96;
    let end = 127;
    data.manifest.world = bundle::Extent {
        min_x: first,
        min_y: first,
        max_x: end + 1,
        max_y: end + 1,
        floors: vec![trigger.floor],
    };
    let mut tiles = Vec::new();
    for y in first..=end {
        for x in first..=end {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if (x, y) == (trigger.x, trigger.y) {
                items.push(sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs: sector::Attrs {
                        unique: callback.selector.unique,
                        action: callback.selector.action,
                        ..Default::default()
                    },
                });
            }
            tiles.push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
        }
    }
    let sector = bundle::Sector {
        floor: trigger.floor,
        sx: first / 32,
        sy: first / 32,
        tiles,
    };
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &[sector], &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: trigger.x - 1,
                y: trigger.y,
                floor: trigger.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}
