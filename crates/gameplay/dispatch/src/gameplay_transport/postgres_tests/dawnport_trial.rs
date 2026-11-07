//! S16c: Dawnport trial tiles switch the vocation on the production paced step.
//!
//! A bundle row at native floor -7 carries the five Crystal trial action ids (x 2..=6). Each
//! step commits a `vocation_trial` build receipt and installs the converted live build; the
//! current vocation's tile and level 20 refuse without a receipt.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::admit;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceRewardOccurrence,
};
use crate::foundation::{ChannelContentPin, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::boot::{BootPins, boot};
use crate::map::overlay::TilePos;
use crate::spell::Vocation;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_simulation_determinism::ExactI64;
use oteryn_world_bundle::writer::*;
use std::time::Duration;

#[test]
fn dawnport_trial_tiles_switch_all_five_vocations_and_back_and_refuse_level_twenty_postgres()
-> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("dawnport-trial", scenario)?;
    eprintln!(
        "S16C_PG_CASE_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

/// Ground at x 0..=7; x 2..=6 carry the trial action ids 25008, 25005, 25000, 25006, 25007.
fn trial_row() -> TestResult<(Vec<u8>, crate::map::BundlePins)> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    let manifest = Manifest {
        subareas: Default::default(),
        format: FORMAT.into(),
        min_reader_version: VERSION,
        projection_class: "server".into(),
        compiler_version: "dawnport-trial-test".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "content-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        },
        palette: vec![PaletteEntry {
            key: "oteryn:terrain.tibia.i100".into(),
            family: Family::Terrain,
            id: 0,
            terrain: Some(Terrain {
                kind: TerrainKind::Ground,
                walkable: Some(true),
                ground_speed: Some(150),
            }),
        }],
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let tile = |x, action| Tile {
        x,
        y: 0,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![Item {
            palette: 0,
            depth: 0,
            attrs: Attrs {
                action,
                ..Attrs::default()
            },
        }],
    };
    let bytes = write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: vec![
                tile(0, None),
                tile(1, None),
                tile(2, Some(25008)),
                tile(3, Some(25005)),
                tile(4, Some(25000)),
                tile(5, Some(25006)),
                tile(6, Some(25007)),
                tile(7, None),
            ],
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    Ok((
        bytes,
        crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "content-1".into(),
            production: false,
        },
    ))
}

/// One production paced step onto `x`, then the owner clock moves past its pacing window.
/// Returns the live vocation and the durable build vocation after the step.
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    actor: ExactActorRef,
    session: GameSessionId,
    direction: StepDirection,
    x: i32,
) -> TestResult<(Vocation, String)> {
    let (step, _) = owner.paced_step(actor, session, direction).await;
    assert_eq!(step.disposition, StepDisposition::Moved, "step to x {x}");
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(2))
        .ok_or("clock")?;
    let runtime = owner.runtime.lock().await;
    assert_eq!(
        runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        MovementLocalPosition { x, y: 0, floor: 7 }
    );
    let states = owner.states.spell_states.lock().await;
    let state = states.get(&runtime, actor, session).ok_or("spell state")?;
    let vitals = state.vitals();
    assert!(vitals.health <= vitals.max_health && vitals.mana <= vitals.max_mana);
    let live = state.character_facts().vocation;
    // No build row is the seeded vocationless build.
    let durable: String = sqlx::query_scalar("SELECT vocation FROM game_character_build_state")
        .fetch_optional(&h.pool)
        .await?
        .unwrap_or_else(|| "none".into());
    Ok((live, durable))
}

async fn receipts(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_build_receipts WHERE cause='vocation_trial'",
    )
    .fetch_one(&h.pool)
    .await?)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "dawnport_trial", false, false).await?;
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = Dependencies::new()?.admitted.world_id;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active generation")?;
        let channel = Dependencies::new()?.admitted.channel_id;
        let (bytes, pins) = trial_row()?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: TilePos {
                    x: 1,
                    y: 0,
                    floor: -7,
                },
            },
            world,
            channel,
            |_| None,
        )
        .map_err(debug)?;
        let deps = Dependencies::with_content(
            ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                (1, 0, 7),
            ),
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let (actor, session) = (deps.actor, deps.admitted.game_session_id);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = &cells;
        let mut admitted = deps.admitted;
        Box::pin(owner.admit_progression_session(&mut admitted)).await;
        let progression = owner
            .progression_binding(session)
            .ok_or("progression binding")?;
        // The fixture has no durably activated equipment catalogue (as in the first-vocation
        // case); admit the Character and its vitals through the production first entry.
        owner.active_generation = None;
        admit(&owner, &deps, &request).await?;
        owner.active_generation = Some(active);
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence read")?
            .ok_or("fence")?;
        assert!(owner.track_restored(
            session,
            actor,
            &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
            fence
        ));
        {
            let runtime = deps.runtime.lock().await;
            assert_eq!(
                runtime
                    .read_actor_position(actor)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 7
                }
            );
        }
        use StepDirection::{East, West};
        // The first step needs no vocation (Crystal tileStep's first-step branch).
        let mut expected = 0;
        for (x, vocation, key) in [
            (2, Vocation::Knight, "knight"),
            (3, Vocation::Sorcerer, "sorcerer"),
            (4, Vocation::Monk, "monk"),
            (5, Vocation::Druid, "druid"),
            (6, Vocation::Paladin, "paladin"),
        ] {
            expected += 1;
            assert_eq!(
                step(&mut owner, &h, actor, session, East, x).await?,
                (vocation, key.into()),
                "trial tile x {x}"
            );
            assert_eq!(receipts(&h).await?, expected);
        }
        // Off the tile and back onto the current vocation's tile changes nothing.
        step(&mut owner, &h, actor, session, East, 7).await?;
        assert_eq!(
            step(&mut owner, &h, actor, session, West, 6).await?,
            (Vocation::Paladin, "paladin".into())
        );
        assert_eq!(receipts(&h).await?, expected);
        // Another trial tile switches back.
        assert_eq!(
            step(&mut owner, &h, actor, session, West, 5).await?,
            (Vocation::Druid, "druid".into())
        );
        assert_eq!(receipts(&h).await?, expected + 1);
        // 98 800 experience is level 20: the next trial tile is refused.
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "fence read")?
            .ok_or("fence")?;
        h.root
            .commit_character_experience(
                &authority,
                &h.node,
                fence,
                ExperienceAwardRequest {
                    occurrence: ExperienceRewardOccurrence::from_bytes(id(91)).map_err(debug)?,
                    amount: ExactI64::new(98_800),
                    context: progression.context.clone(),
                    policy_revision: progression.policy_revision.clone(),
                    reward_revision: progression.reward_revision.clone(),
                    policy: progression.policy.clone(),
                },
            )
            .await
            .map_err(debug)?;
        assert_eq!(
            step(&mut owner, &h, actor, session, West, 4).await?,
            (Vocation::Druid, "druid".into())
        );
        assert_eq!(receipts(&h).await?, expected + 1);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
