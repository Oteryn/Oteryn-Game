//! QREADY1: source-bound USE, exact native backpack reward, second-claim refusal and durable replay.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

// Compile the retained repository regions without injecting chest UIDs or appearances.
fn retained_world(
    chest: (u16, u16, i8),
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle_compiler::{bundle, compile, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let project = source::project_regions(&root, &[(chest.0 / 256, chest.1 / 256)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let (_, mut pins) = crate::map::boot::tests::bundle();
    let identity = oteryn_world_bundle::bundle::read(&crate::map::boot::tests::bundle().0)?
        .manifest
        .identity;
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &registry,
    )?;
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    for (dx, dy) in [
        (-1, 0),
        (0, 1),
        (1, 0),
        (0, -1),
        (-1, 1),
        (1, 1),
        (-1, -1),
        (1, -1),
    ] {
        let start = crate::map::overlay::TilePos {
            x: u16::try_from(i32::from(chest.0) + dx)?,
            y: u16::try_from(i32::from(chest.1) + dy)?,
            floor: chest.2,
        };
        if !base
            .tile(start.x, start.y, start.floor)
            .is_some_and(|tile| tile.walkable())
        {
            continue;
        }
        match crate::map::boot::boot(
            &compiled.bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins.clone(),
                start,
            },
            crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            |key| {
                use crate::content::{ReferenceItemField, ReferenceItemType};
                let index = native.item_index()?;
                let revision = index.revision(key)?;
                let reference = index.definition_ref("Item", key, revision)?;
                let profile = native
                    .item_policy(key, revision)
                    .map(|policy| policy.record());
                let pickupable = profile.is_some_and(|profile| {
                    matches!(
                    &profile.semantics.physical, ReferenceItemField::Known(physical)
                    if physical.pickupable == ReferenceItemField::Known(true))
                });
                let door = profile.is_some_and(|profile| matches!(
                    &profile.semantics.classification, ReferenceItemField::Known(classification)
                    if classification.item_type == ReferenceItemField::Known(ReferenceItemType::Door)));
                let attributes = profile.map(|profile| &profile.attributes);
                Some(crate::map::facts::ItemDefinition {
                    reference,
                    pickupable,
                    door,
                    solid: attributes.and_then(|a| a.blocks_movement),
                    blocks_projectile: attributes.and_then(|a| a.blocks_projectile).unwrap_or(true),
                })
            },
        ) {
            Ok(world) => return Ok(world),
            Err(crate::map::boot::BootRefusal::StartNotEnterable) => continue,
            Err(error) => return Err(debug(error)),
        }
    }
    Err("source chest has no enterable adjacent tile".into())
}

fn claim_case(
    claim: &str,
    chest: (u16, u16, i8),
    choices: &[(u16, u16)],
    appearance: u16,
    reward: &str,
    quantity: u32,
) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "qready1_claim", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        // Doublet binds the real ground entry (ordinal zero). Other source
        // chests reuse the production dispatch fixture; the companion Python
        // check verifies every UID/appearance against the shipped region files.
        let world = if claim == "oteryn:reward-claim.kv.doublet" {
            retained_world(chest, native)?
        } else {
            super::quest_callbacks::source_world_choices(chest, choices, appearance)?
        };
        let start = world.start();
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        assert!(
            report
                .served
                .iter()
                .any(|entry| entry.key.as_str() == claim),
            "{claim}: {:?}",
            report.unbound
        );
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
            ),
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
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: i32::from(start.x),
                    y: i32::from(start.y),
                    floor: -i16::from(start.floor),
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        let weights = NativeItemWeights(native);
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        harness
            .root
            .initialize_admission_equipment(
                &authority,
                &harness.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        harness
            .root
            .grant_starter_items(&authority, &harness.node, &fence, &weights, &template)
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let key = report
            .served
            .iter()
            .find(|entry| entry.key.as_str() == claim)
            .and_then(|entry| {
                entry
                    .placements
                    .iter()
                    .find(|p| p.cell.x == chest.0 && p.cell.y == chest.1)
            })
            .ok_or("bound source placement")?
            .bundle_placement_key;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // The second USE is a new command on the live connection. Rebuild the owner
        // for the replay so no Foundation command-result cache can satisfy it.
        for (commands, expected) in [
            (
                &[1, 2][..],
                &[UseDisposition::Committed, UseDisposition::NothingToUse][..],
            ),
            (&[1][..], &[UseDisposition::Committed][..]),
        ] {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                super::quest_callbacks::wire_uses(&owner, admitted, commands, &target)
                    .await
                    .map_err(|e| format!("claim={claim} commands={commands:?}: {e}"))?,
                expected
            );
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("bag")?;
            assert_eq!(bag.entries.len(), 1);
            assert_eq!(bag.entries[0].item.quantity, quantity);
            assert_eq!(bag.entries[0].item.definition.production_key, reward);
        }
        let claims: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2), (SELECT count(*) FROM game_reward_claim_mint_receipts)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(claim).fetch_one(&harness.pool).await?;
        assert_eq!(claims, (1, 1));
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn qready1_adorned_uh_rune_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.adoreduh",
                (33136, 31601, -15),
                &[(33136, 6025)],
                2469,
                "oteryn:item.tibia.i11603",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_adorned_uh_rune_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_barbarian_axe_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.barbarianaxe1",
                (33184, 31945, -11),
                &[(33184, 6026), (33185, 6027)],
                2469,
                "oteryn:item.tibia.i3317",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.barbarianaxe1",
                (33185, 31945, -11),
                &[(33184, 6026), (33185, 6027)],
                2469,
                "oteryn:item.tibia.i3307",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_barbarian_axe_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_battle_axe_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.battleaxe",
                (32305, 32254, -9),
                &[(32305, 6180)],
                4285,
                "oteryn:item.tibia.i3266",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_battle_axe_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_blood_herb_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.bloodherb",
                (32769, 31968, -7),
                &[(32769, 6111)],
                3634,
                "oteryn:item.tibia.i3734",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_blood_herb_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_captain_iglues_treasure_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.captain_iglues",
                (32039, 32121, -13),
                &[(32039, 6117)],
                2472,
                "oteryn:item.tibia.i3579",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_captain_iglues_treasure_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_circle_room_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.circleroom1",
                (32495, 31992, -14),
                &[(32495, 6122)],
                2469,
                "oteryn:item.tibia.i3279",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.circleroom2",
                (32497, 31992, -14),
                &[(32497, 6123)],
                2469,
                "oteryn:item.tibia.i3323",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_circle_room_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_combat_knife_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.combatknife",
                (32102, 32235, -8),
                &[(32102, 5025)],
                2473,
                "oteryn:item.tibia.i3292",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_combat_knife_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_crusader_helmet_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.crusaderhelmet",
                (32427, 31943, -14),
                &[(32427, 6164)],
                3990,
                "oteryn:item.tibia.i3391",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_crusader_helmet_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_crystal_wand_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.crystalwand1",
                (32479, 31611, -15),
                &[(32479, 6124)],
                2472,
                "oteryn:item.tibia.i3068",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.crystalwand2",
                (32481, 31611, -15),
                &[(32481, 6125)],
                2472,
                "oteryn:item.tibia.i11609",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_crystal_wand_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_darashia_dragon_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.quest.u8_5.darashia_dragon",
                (33233, 32280, -12),
                &[(33233, 6285)],
                4286,
                "oteryn:item.tibia.i3052",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_darashia_dragon_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_dark_armor_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.darkarmor",
                (33178, 31870, -12),
                &[(33178, 6028)],
                4311,
                "oteryn:item.tibia.i3383",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_dark_armor_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_demona_ring_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.demonaring1",
                (32504, 31596, -14),
                &[(32504, 6126)],
                2472,
                "oteryn:item.tibia.i3049",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.demonaring2",
                (32515, 31596, -14),
                &[(32515, 6127)],
                2472,
                "oteryn:item.tibia.i3051",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_demona_ring_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_double_hero_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.doublehero1",
                (33109, 31679, -13),
                &[(33109, 6032)],
                2469,
                "oteryn:item.tibia.i3039",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.doublehero2",
                (33110, 31679, -13),
                &[(33110, 6033)],
                2469,
                "oteryn:item.tibia.i3093",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_double_hero_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_doublet_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.doublet",
                (32084, 32181, -8),
                &[(32084, 6181)],
                408,
                "oteryn:item.tibia.i3379",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_doublet_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_elephant_tusk_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.quest.u7_5.elephant_tusk",
                (32922, 32755, -7),
                &[(32922, 6185)],
                2472,
                "oteryn:item.tibia.i3044",
                2,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_elephant_tusk_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_explorer_brooch_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.explorerbrooch",
                (32636, 31873, -10),
                &[(32636, 6019)],
                4240,
                "oteryn:item.tibia.i4871",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_explorer_brooch_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_griffin_shield_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.griffinshield1",
                (32498, 31721, -15),
                &[(32498, 6165)],
                4240,
                "oteryn:item.tibia.i3433",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.griffinshield2",
                (32500, 31721, -15),
                &[(32500, 6166)],
                4024,
                "oteryn:item.tibia.i3313",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.griffinshield3",
                (32503, 31724, -15),
                &[(32503, 6167)],
                4024,
                "oteryn:item.tibia.i3323",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_griffin_shield_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
