//! Thieves Guild reward claims through literal USE on the committed map.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::wire_use_targets_retained;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

// Wiki base (curid 25436, oldid 1090720) and /Spoiler (curid 25437, oldid 1086330).
// Crystal 201189f8, data-global/startup/tables/chest.lua:2630-2670.
macro_rules! source_claim {
    ($name:ident, $position:expr, $appearance:literal, $uid:literal, $item:literal) => {
        #[test]
        fn $name() -> TestResult {
            if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
                return Ok(());
            }
            let started = std::time::Instant::now();
            let result = super::kill_reward_e2e::scenario_thread::run(stringify!($name), || {
                claim(
                    $position,
                    $appearance,
                    $uid,
                    concat!("oteryn:item.tibia.i", $item),
                )
            });
            eprintln!(
                "POSTGRES_CASE_COMPLETED {} duration={:?} success={}",
                stringify!($name),
                started.elapsed(),
                result.is_ok()
            );
            result
        }
    };
}

source_claim!(modified_crossbow, (32309, 32209, -8), 2434, 6275, "8021");
source_claim!(assassin_dagger, (32309, 32211, -8), 2434, 6276, "7404");
source_claim!(spellbook_of_warding, (32309, 32213, -8), 2434, 6277, "8073");
source_claim!(theatre_script, (32367, 31781, -8), 2473, 6272, "8117");
source_claim!(
    stolen_golden_goblet,
    (32551, 32652, -10),
    2469,
    6273,
    "7870"
);
source_claim!(compromising_letter, (32902, 32143, -4), 2433, 6274, "7935");

fn source_unique((x, y, floor): (u16, u16, i8), appearance: u16) -> TestResult<Option<u16>> {
    use oteryn_world_bundle::bundle;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let path = format!(
        "content/world/placements/region-z{:02}-x{:03}-y{:03}.b3",
        -floor,
        x / 256,
        y / 256
    );
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let mut budget = bundle::BUNDLE_BUDGET;
    let region = oteryn_world_bundle_compiler::b3::decode_region(
        &std::fs::read(root.join(path))?,
        bundle::TILE_LIMITS,
        &mut budget,
    )?;
    let mut matches = region
        .sectors
        .iter()
        .flat_map(|(_, _, tiles)| tiles)
        .filter(|tile| (tile.x, tile.y) == (x, y))
        .flat_map(|tile| &tile.items)
        .filter(|item| {
            item.depth == 0
                && index["palette"][item.palette as usize]["source_item_id"] == appearance
        });
    let item = matches.next().ok_or("missing source chest")?;
    assert!(matches.next().is_none(), "ambiguous source chest");
    Ok(item.attrs.unique)
}

fn claim(position: (u16, u16, i8), appearance: u16, expected_uid: u16, reward: &str) -> TestResult {
    let (x, y, floor) = position;
    let uid = source_unique(position, appearance)?;
    let single = (x, y);
    // The final claim binds only when all three of its placements are present.
    let positions = if appearance == 2434 {
        [(32309, 32209), (32309, 32211), (32309, 32213)].as_slice()
    } else {
        std::slice::from_ref(&single)
    };
    let choices: Vec<_> = positions
        .iter()
        .map(|&(x, y)| {
            Ok((
                x,
                y,
                source_unique((x, y, floor), appearance)?.unwrap_or_default(),
            ))
        })
        .collect::<TestResult<_>>()?;
    // Keep actual source placement/appearance/UID. Reduce the surrounding
    // geometry so this tests a claim from inside its room.
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "qcx_thieves_guild", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let world = super::quest_callbacks::source_world_positions(position, &choices, appearance)?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let at = MovementLocalPosition {
            x: i32::from(x) - 1,
            y: i32::from(y),
            floor: -i16::from(floor),
        };
        deps.combat_room = Some(
            activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    activated.frame_binding().digest(),
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: at.x,
                        y: at.y,
                        floor: at.floor,
                    },
                )
                .ok_or("qualified source reward Bundle room")?,
        );
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (at.x, at.y, at.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, at)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
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
                &NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: definition("oteryn:item.tibia.i2854"),
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let key =
            oteryn_world_bundle::bundle::placement_key(floor, x, y, 1).ok_or("reward placement")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        assert_eq!(
            wire_use_targets_retained(&owner, &mut admitted, &[(1, &target)]).await?,
            [(CommandStatus::Accepted, UseDisposition::Committed)],
            "source USE at ({x}, {y}, {floor})"
        );
        assert_eq!(uid, Some(expected_uid));
        let granted: i64 = sqlx::query_scalar("SELECT coalesce(sum(quantity), 0)::bigint FROM game_item_instances WHERE lifecycle = 1 AND definition_production_key = $1")
            .bind(reward)
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(granted, 1);
        Ok(())
    })
}
