//! Native USE and death/clock dispatch for the Grave Danger ritual.
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, fixture::{Harness, debug}};
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
    super::engenc2_cults::phase_scenario(false, true)
}

pub(super) fn configure(callback: &mut Callback) -> TestResult {
    callback.key = "oteryn:quest-callback/grave-danger/zelos-ritual".into();
    callback.storage.clear();
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(EncounterPhase::GraveDangerZelos {
        king: Position { x: 115, y: 113, floor: -7 },
        exit: Position { x: 103, y: 100, floor: -7 },
    });
    encounter.minimum_level = 1;
    let entry = encounter.players.first().ok_or("party arrival")?.to;
    encounter.players = (0..10)
        .map(|i| oteryn_gameplay_quests::quest::world_callbacks::EncounterPlayer {
            from: Position { x: 100 + i / 5, y: 100 + i % 5, floor: -7 },
            to: entry,
        })
        .collect();
    encounter.monsters.truncate(4);
    for (monster, name) in encounter.monsters.iter_mut().zip(KNIGHTS) {
        monster.creature = name.into();
    }
    Ok(())
}

pub(super) async fn admit_partner(
    h: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: &AdmittedSession,
) -> TestResult<ExactActorRef> {
    let now: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
    )
    .fetch_one(&h.pool)
    .await?;
    let current =
        crate::world_party_runtime_postgres_cases::committed_fresh_player(h, 61, 63, 71, now).await?;
    let session = current.current_game_session_id();
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime.bind_attacker_lease(actor, session, current.current_character_lease()).map_err(debug)?;
        runtime.initialize_pinned_test_position(actor, MovementLocalPosition {
            x: 100, y: 101, floor: 7,
        }).map_err(debug)?;
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
    Ok(actor)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    partner: ExactActorRef,
) -> TestResult {
    let find = |runtime: &crate::foundation::ChannelRuntimeV1, name: &str| {
        runtime
            .current_live_creature_census()
            .map(|actors| actors.into_iter().find(|actor| {
                runtime.matches_live_creature_identity(*actor, name.as_bytes())
            }))
            .map_err(debug)
    };
    let player = admitted.runtime_actor.ok_or("admitted player")?;
    let nargol = {
        let runtime = owner.runtime.lock().await;
        for member in [player, partner] {
            let at = runtime.read_actor_position(member).map_err(debug)?.position();
            assert_eq!((at.x, at.y, at.floor), (116, 117, 7));
        }
        find(&runtime, KNIGHTS[1])?.ok_or("Nargol spawned by native USE")?
    };
    super::engenc2_cults::kill(owner, admitted, nargol, KNIGHTS[1], 101).await?;
    let now = owner.owner_now().get();
    tick_at(owner, now).await;
    let mass = "oteryn:creature.regenerating_mass";
    {
        let runtime = owner.runtime.lock().await;
        assert!(find(&runtime, mass)?.is_some(), "mass replaces Nargol");
    }
    tick_at(owner, now + 29_999_999).await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(find(&runtime, mass)?.is_some(), "mass survives until the deadline");
    }
    tick_at(owner, now + 30_000_000).await;
    {
        let runtime = owner.runtime.lock().await;
        assert!(find(&runtime, mass)?.is_none(), "mass expires at 30 seconds");
        assert_ne!(find(&runtime, KNIGHTS[1])?.ok_or("regenerated Nargol")?, nargol);
        assert!(find(&runtime, "oteryn:creature.king_zelos")?.is_none());
    }
    tick_at(owner, now + 24 * 60 * 1_000_000).await;
    let runtime = owner.runtime.lock().await;
    for knight in KNIGHTS {
        assert!(find(&runtime, knight)?.is_none(), "timeout cleans up {knight}");
    }
    for member in [player, partner] {
        let exit = runtime.read_actor_position(member).map_err(debug)?.position();
        assert_eq!((exit.x, exit.y, exit.floor), (103, 100, 7));
    }
    Ok(())
}

async fn tick_at(owner: &ComposedFreshAdmission<'_, '_, '_>, now: u64) {
    use std::sync::atomic::Ordering;
    owner.states.test_owner_time.store(now + 1, Ordering::Relaxed);
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
            tiles.push(sector::Tile { x, y, flags: 0, house: 0, zones: vec![], items });
        }
    }
    let sector = bundle::Sector {
        floor: trigger.floor,
        sx: first / 32,
        sy: first / 32,
        tiles,
    };
    let bytes = oteryn_world_bundle::writer::write(
        &data.manifest, &[sector], &Default::default(),
    )?;
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
    ).map_err(debug)
}
