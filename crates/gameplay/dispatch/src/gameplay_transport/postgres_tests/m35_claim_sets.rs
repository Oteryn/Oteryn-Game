//! Crystal 201189f8 chest.lua:917-931: both independent Alawar claims, in either order.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::{
    source_world_choices, source_world_objects, source_world_spatial, wire_use_targets_retained,
};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn m35_alawar_dispatch_completes_only_on_last_chest() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::Alawar).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 dispatch scenario panicked")??;
    println!("M35_ALAWAR_POSTGRES_CASE_COMPLETED");
    Ok(())
}
// Crystal 201189f8 chest.lua:2131-2161: these placements share ChestsWine storage.
#[test]
fn m35_dreamer_wine_alternatives_do_not_complete_other_required_groups() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::Wine).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 wine dispatch scenario panicked")??;
    println!("M35_WINE_ALTERNATIVES_POSTGRES_CASE_COMPLETED");
    Ok(())
}
fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}
#[derive(Clone, Copy)]
enum Case {
    Alawar,
    Wine,
    Fanfare,
    Key3520,
    Koshei,
    BlackKnight,
    FireAxeBag,
    FireAxeAll,
    EmperorCookies,
    Draconia,
    GoblinTemple,
    CrystalWand,
    CircleRoom,
    DemonaRing,
    DoubleHero,
}

// Crystal 201189f chest.lua: UID5018 is access; UID6142 is the final Fanfare.
#[test]
fn m35_fanfare_access_key_does_not_complete_but_final_chest_does() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for case in [Case::Key3520, Case::Fanfare] {
                scenario(false, case).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Fanfare dispatch scenario panicked")??;
    println!("M35_FANFARE_POSTGRES_CASE_COMPLETED");
    Ok(())
}
// Crystal 201189f8 chest.lua: Demona Ring's two independent rewards.
#[test]
fn m35_source_pair_demona_ring_requires_both_rewards_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::DemonaRing).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Demona Ring scenario panicked")??;
    println!("M35_DEMONA_RING_POSTGRES_CASE_COMPLETED");
    Ok(())
}
// Crystal 201189f8 chest.lua: Double Hero's two independent rewards.
#[test]
fn m35_source_pair_double_hero_requires_both_rewards_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::DoubleHero).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Double Hero scenario panicked")??;
    println!("M35_DOUBLE_HERO_POSTGRES_CASE_COMPLETED");
    Ok(())
}
// Crystal 201189f8 chest.lua: Circle Room's two independent rewards.
#[test]
fn m35_circle_room_dispatch_requires_both_rewards_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::CircleRoom).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Circle Room scenario panicked")??;
    println!("M35_CIRCLE_ROOM_POSTGRES_CASE_COMPLETED");
    Ok(())
}
// Crystal 201189f8 chest.lua:1540-1554: independent Crystal Wand rewards.
#[test]
fn m35_crystal_wand_dispatch_requires_both_rewards_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::CrystalWand).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Crystal Wand scenario panicked")??;
    println!("M35_CRYSTAL_WAND_POSTGRES_CASE_COMPLETED");
    Ok(())
}
#[test]
fn m35_goblin_temple_dispatch_requires_both_complete_bags_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::GoblinTemple).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Goblin Temple scenario panicked")??;
    println!("M35_GOBLIN_TEMPLE_POSTGRES_CASE_COMPLETED");
    Ok(())
}
#[test]
fn m35_draconia_dispatch_requires_both_complete_bags_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::Draconia).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Draconia scenario panicked")??;
    println!("M35_DRACONIA_POSTGRES_CASE_COMPLETED");
    Ok(())
}
#[test]
fn m35_emperor_cookies_and_key3802_complete_only_after_both_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::EmperorCookies).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Emperor Cookies scenario panicked")??;
    println!("M35_EMPEROR_COOKIES_POSTGRES_CASE_COMPLETED");
    Ok(())
}
#[test]
fn m35_fire_axe_two_floor_rewards_complete_only_after_both_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::FireAxeAll).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 two-floor Fire Axe scenario panicked")??;
    println!("M35_FIRE_AXE_TWO_FLOOR_POSTGRES_CASE_COMPLETED");
    Ok(())
}
// Crystal 201189f8 chest.lua:1078-1087: complete bag, not a 200-item necklace stack.
#[test]
fn m35_fire_axe_bag_grants_all_children_without_finishing_on_first_chest() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, Case::FireAxeBag).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "M35 Fire Axe dispatch scenario panicked")??;
    println!("M35_FIRE_AXE_BAG_POSTGRES_CASE_COMPLETED");
    Ok(())
}
// Crystal 201189f8 chest.lua:932-947: independent Crown Armor and Crown Shield.
#[test]
fn m35_black_knight_dispatch_requires_both_rewards_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::BlackKnight).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Black Knight dispatch scenario panicked")??;
    println!("M35_BLACK_KNIGHT_POSTGRES_CASE_COMPLETED");
    Ok(())
}
// Crystal 201189f8 chest.lua:2459-2472: the two final reward storage groups.
#[test]
fn m35_koshei_dispatch_requires_legs_and_platinum_in_either_order() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, Case::Koshei).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 Koshei dispatch scenario panicked")??;
    println!("M35_KOSHEI_POSTGRES_CASE_COMPLETED");
    Ok(())
}
fn scenario(reverse: bool, case: Case) -> TestResult {
    let wine = matches!(case, Case::Wine);
    let access_key = matches!(case, Case::Key3520);
    let (chest, appearance, quest, choices): (_, _, _, &[(u16, u16)]) = match case {
        Case::Wine => (
            (32749, 32341, -14),
            2469,
            "oteryn:quest.dreamer_s_challenge_quest",
            &[(32749, 6197), (32751, 6198), (32753, 6199), (32755, 6200)],
        ),
        Case::Alawar => (
            (32105, 31567, -9),
            2472,
            "oteryn:quest.alawar_s_vault_quest",
            &[(32105, 6045), (32109, 6046)],
        ),
        Case::Fanfare => (
            (32390, 31769, -9),
            2472,
            "oteryn:quest.fanfare_quest",
            &[(32390, 6142)],
        ),
        Case::DemonaRing => (
            (32504, 31596, -14),
            2472,
            "oteryn:quest.demona_ring_quest",
            &[(32504, 6126), (32515, 6127)],
        ),
        Case::DoubleHero => (
            (33109, 31679, -13),
            2469,
            "oteryn:quest.double_hero_quest",
            &[(33109, 6032), (33110, 6033)],
        ),
        Case::CircleRoom => (
            (32495, 31992, -14),
            2469,
            "oteryn:quest.circle_room_quest",
            &[(32495, 6122), (32497, 6123)],
        ),
        Case::CrystalWand => (
            (32479, 31611, -15),
            2472,
            "oteryn:quest.crystal_wand_quest",
            &[(32479, 6124), (32481, 6125)],
        ),
        Case::GoblinTemple => (
            (31973, 32209, -12),
            2472,
            "oteryn:quest.goblin_temple_quest",
            &[(31973, 6118), (31977, 6119)],
        ),
        Case::Draconia => (
            (32803, 31582, -2),
            2472,
            "oteryn:quest.draconia_quest",
            &[(32803, 6023), (32804, 6024)],
        ),
        Case::EmperorCookies => (
            (32648, 31905, -3),
            2472,
            "oteryn:quest.emperor_s_cookies_quest",
            &[(32648, 5004), (32599, 5005)],
        ),
        Case::FireAxeAll => (
            (33078, 31656, -11),
            2472,
            "oteryn:quest.fire_axe_quest",
            &[(33078, 6067), (33084, 6036)],
        ),
        Case::FireAxeBag => (
            (33078, 31656, -11),
            2472,
            "oteryn:quest.fire_axe_quest",
            &[(33078, 6067)],
        ),
        Case::BlackKnight => (
            (32868, 31955, -11),
            3634,
            "oteryn:quest.black_knight_quest",
            &[(32868, 6047), (32880, 6048)],
        ),
        Case::Koshei => (
            (33261, 32444, -12),
            2472,
            "oteryn:quest.koshei_the_deathless_quest",
            &[(33261, 6240), (33261, 6241)],
        ),
        Case::Key3520 => (
            (32376, 31802, -7),
            2473,
            "oteryn:quest.fanfare_quest",
            &[(32376, 5018)],
        ),
    };
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "m35_claim_sets", false, false).await?;
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
        if wine {
            for (key, weight) in [
                ("oteryn:item.tibia.i6498", 150),
                ("oteryn:item.tibia.i3602", 400),
                ("oteryn:item.tibia.i2874", 180),
            ] {
                let facts = generation_item_facts(native, &definition(key)).map_err(debug)?;
                assert_eq!(facts.weight, Some(weight));
                assert_ne!(
                    facts.stack,
                    crate::durability::item_transfer::ItemStackClass::Unknown
                );
            }
        }
        let world = if matches!(case, Case::FireAxeAll) {
            source_world_spatial(
                chest,
                &[
                    (33078, 31656, -11, 6067, 2472),
                    (33084, 31650, -12, 6036, 4024),
                ],
            )?
        } else if matches!(case, Case::EmperorCookies) {
            source_world_spatial(
                chest,
                &[
                    (32648, 31905, -3, 5004, 2472),
                    (32599, 31923, -6, 5005, 2472),
                ],
            )?
        } else if matches!(case, Case::Koshei) {
            source_world_objects(
                chest,
                &[(33261, 32444, 6240, 2472), (33261, 32448, 6241, 2480)],
            )?
        } else {
            source_world_choices(chest, choices, appearance)?
        };
        let bound = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        for claim in &bound.served {
            assert!(
                claim
                    .placements
                    .iter()
                    .all(|p| p.placement.quest_transition.is_some() != access_key),
                "source chest must bind a claim-set transition"
            );
        }
        assert_eq!(
            bound.served.len(),
            if matches!(
                case,
                Case::Alawar
                    | Case::Koshei
                    | Case::BlackKnight
                    | Case::FireAxeAll
                    | Case::EmperorCookies
                    | Case::Draconia
                    | Case::GoblinTemple
                    | Case::CrystalWand
                    | Case::CircleRoom
                    | Case::DemonaRing
                    | Case::DoubleHero
            ) {
                2
            } else {
                1
            },
            "left out: {:?}; unbound: {:?}",
            bound.left_out,
            bound.unbound
        );
        let catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let at = MovementLocalPosition {
            x: i32::from(chest.0) - 1,
            y: i32::from(chest.1),
            floor: -i16::from(chest.2),
        };
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (at.x, at.y, at.floor),
        );
        let world_id = deps.admitted.world_id;
        let channel_id = deps.admitted.channel_id;
        let game_session_id = deps.admitted.game_session_id;
        let runtime_at = |position| -> TestResult<_> {
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                world_id,
                channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                2,
                pin.clone(),
            )
            .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(game_session_id)
                .map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(actor, position)
                .map_err(debug)?;
            Ok((runtime, actor))
        };
        let (runtime, actor) = runtime_at(at)?;
        deps.actor = actor;
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

        let channel_states = std::sync::Arc::new(
            crate::gameplay_transport::channel_states::ChannelSessionStates::default(),
        );
        let clock_origin = std::time::Instant::now();
        let mut position = at;
        let mut command = 1;
        for index in 0..choices.len() {
            let position_index = if reverse {
                choices.len() - index - 1
            } else {
                index
            };
            let x = choices[position_index].0;
            let y = if matches!(case, Case::EmperorCookies) {
                if position_index == 0 { 31905 } else { 31923 }
            } else if matches!(case, Case::FireAxeAll) {
                if position_index == 0 { 31656 } else { 31650 }
            } else if matches!(case, Case::Koshei) {
                chest.1 + 4 * u16::try_from(position_index)?
            } else {
                chest.1
            };
            let floor = if matches!(case, Case::EmperorCookies) && position_index == 1 {
                -6
            } else if matches!(case, Case::FireAxeAll) && position_index == 1 {
                -12
            } else {
                chest.2
            };
            let destination = MovementLocalPosition {
                x: i32::from(x) - 1,
                y: i32::from(y),
                floor: -i16::from(floor),
            };
            if matches!(case, Case::FireAxeAll | Case::EmperorCookies) {
                // Fixture admission at each literal source endpoint; travel is not
                // under test. Keep the same SQL session fence and command stream.
                let (runtime, actor) = runtime_at(destination)?;
                deps.actor = actor;
                deps.runtime = tokio::sync::Mutex::new(runtime);
                position = destination;
            }
            // Rebuild the caller while retaining the Channel's command-stream truth.
            // Drop its quest cache so admission must reload the durable claim tracks.
            let mut owner = deps.owner(&h, &authority);
            owner.states = std::sync::Arc::clone(&channel_states);
            owner.clock_origin = clock_origin;
            owner
                .states
                .quest_sessions
                .lock()
                .map_err(|_| "quest cache")?
                .remove(&admitted.game_session_id);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.quest_catalogue = catalogue.clone();
            owner.admit_progression_session(&mut admitted).await;
            owner.admit_quest_session(&admitted).await;
            deps.initialize(&owner).await;
            walk(
                &owner,
                &mut admitted,
                &mut command,
                &mut position,
                destination,
            )
            .await?;
            let key =
                oteryn_world_bundle::bundle::placement_key(floor, x, y, 1).ok_or("placement")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let target = WorldObjectTarget {
                placement,
                expected_revision: 0,
            };
            let first_result = if wine && index > 0 {
                UseDisposition::NothingToUse
            } else {
                UseDisposition::Committed
            };
            assert_eq!(
                wire_use_targets_retained(
                    &owner,
                    &mut admitted,
                    &[
                        (command, &target),
                        (command, &target),
                        (command + 1, &target)
                    ]
                )
                .await?,
                [
                    (CommandStatus::Accepted, first_result),
                    (CommandStatus::DuplicateReplay, first_result),
                    (CommandStatus::Accepted, UseDisposition::NothingToUse)
                ]
            );
            command += 2;
            let counts: (i64, i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_reward_claims), \
                 (SELECT count(*) FROM game_character_quest_receipts), \
                 (SELECT count(*) FROM game_character_quest_obligations WHERE state='CONSUMED'), \
                 (SELECT count(*) FROM game_character_quest_obligations WHERE state <> 'CONSUMED')",
            )
            .fetch_one(&h.pool)
            .await?;
            let claims = if wine { 1 } else { index as i64 + 1 };
            let quest_claims = if access_key { 0 } else { claims };
            assert_eq!(counts, (claims, quest_claims, quest_claims, 0));
            let snapshot = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            if access_key {
                let keys: Vec<(i32, i64, bool)> = sqlx::query_as(
                    "SELECT k.key_number,i.quantity,k.minted_transaction_id=r.transaction_id \
                     FROM game_item_keys k JOIN game_item_instances i USING(item_instance_id) \
                     JOIN game_reward_claim_mint_receipts r USING(item_instance_id) LIMIT 2",
                )
                .fetch_all(&h.pool)
                .await?;
                assert_eq!(keys, vec![(3520, 1, true)]);
                assert!(
                    !snapshot
                        .states()
                        .get(quest)
                        .is_some_and(|state| state.completed)
                );
                assert!(
                    !snapshot
                        .tracks()
                        .contains_key("oteryn:quest-progress/kv/fanfare/claimed")
                );
            } else {
                let state = snapshot.states().get(quest).ok_or("quest state")?;
                assert_eq!(
                    state.completed,
                    matches!(case, Case::Fanfare)
                        || matches!(
                            case,
                            Case::Alawar
                                | Case::Koshei
                                | Case::BlackKnight
                                | Case::FireAxeAll
                                | Case::EmperorCookies
                                | Case::Draconia
                                | Case::GoblinTemple
                                | Case::CrystalWand
                                | Case::CircleRoom
                                | Case::DemonaRing
                                | Case::DoubleHero
                        ) && index == 1
                );
            }
            if matches!(case, Case::GoblinTemple) {
                let children: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT i.definition_production_key,i.quantity FROM game_item_nested_entries e \
                     JOIN game_item_instances i USING(item_instance_id) ORDER BY i.definition_production_key LIMIT 7")
                    .fetch_all(&h.pool).await?;
                let mut expected = Vec::new();
                for selected in 0..=index {
                    let pos = if reverse {
                        choices.len() - selected - 1
                    } else {
                        selected
                    };
                    let source_items = if pos == 0 {
                        [
                            ("oteryn:item.tibia.i3551", 1),
                            ("oteryn:item.tibia.i1781", 5),
                            ("oteryn:item.tibia.i3031", 50),
                        ]
                    } else {
                        [
                            ("oteryn:item.tibia.i3466", 1),
                            ("oteryn:item.tibia.i2992", 4),
                            ("oteryn:item.tibia.i2874", 1),
                        ]
                    };
                    expected.extend(
                        source_items
                            .into_iter()
                            .map(|(key, qty)| (key.to_owned(), qty)),
                    );
                }
                expected.sort();
                assert_eq!(children, expected);
                let roots: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r \
                     JOIN game_item_instances i USING(item_instance_id) LIMIT 3")
                    .fetch_all(&h.pool).await?;
                assert_eq!(
                    roots,
                    vec![("oteryn:item.tibia.i2853".to_owned(), 1); index + 1]
                );
                let fluids: Vec<(i64,i16,bool,bool)> = sqlx::query_as(
                    "SELECT i.quantity,f.fluid_subtype,m.root_transaction_id=r.transaction_id, \
                     f.last_transaction_id=m.transaction_id FROM game_reward_claim_mint_receipts r \
                     JOIN game_item_nested_entries e ON e.parent_item_instance_id=r.item_instance_id \
                     JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id \
                     JOIN game_item_fluid_states f ON f.item_instance_id=i.item_instance_id \
                     JOIN game_item_fluid_reward_mints m ON m.item_instance_id=i.item_instance_id \
                     WHERE i.definition_production_key='oteryn:item.tibia.i2874' LIMIT 2")
                    .fetch_all(&h.pool).await?;
                assert_eq!(
                    fluids,
                    if reverse || index == 1 {
                        vec![(1, 1, true, true)]
                    } else {
                        Vec::new()
                    }
                );
            }
            if matches!(case, Case::Draconia) {
                let children: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT i.definition_production_key,i.quantity FROM game_item_nested_entries e \
                     JOIN game_item_instances i USING(item_instance_id) ORDER BY i.definition_production_key LIMIT 5")
                    .fetch_all(&h.pool).await?;
                let mut expected = Vec::new();
                for selected in 0..=index {
                    let pos = if reverse {
                        choices.len() - selected - 1
                    } else {
                        selected
                    };
                    for item in if pos == 0 {
                        ["oteryn:item.tibia.i3284", "oteryn:item.tibia.i3297"]
                    } else {
                        ["oteryn:item.tibia.i3081", "oteryn:item.tibia.i3051"]
                    } {
                        expected.push((item.to_owned(), 1));
                    }
                }
                expected.sort();
                assert_eq!(children, expected);
                let roots: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r \
                     JOIN game_item_instances i USING(item_instance_id) LIMIT 3")
                    .fetch_all(&h.pool).await?;
                assert_eq!(
                    roots,
                    vec![("oteryn:item.tibia.i2853".to_owned(), 1); index + 1]
                );
            }
            if matches!(case, Case::EmperorCookies) {
                let cookies_claimed = !reverse || index == 1;
                let key_claimed = reverse || index == 1;
                let children: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT i.definition_production_key,i.quantity FROM game_item_nested_entries e \
                     JOIN game_item_instances i USING(item_instance_id) ORDER BY i.definition_production_key LIMIT 3")
                    .fetch_all(&h.pool).await?;
                let expected_children = if cookies_claimed {
                    vec![
                        ("oteryn:item.tibia.i2970".to_owned(), 1),
                        ("oteryn:item.tibia.i3598".to_owned(), 27),
                    ]
                } else {
                    Vec::new()
                };
                assert_eq!(children, expected_children);
                let keys: Vec<(i32, String, i64)> = sqlx::query_as(
                    "SELECT k.key_number,i.definition_production_key,i.quantity FROM game_item_keys k \
                     JOIN game_item_instances i USING(item_instance_id) ORDER BY k.key_number LIMIT 3")
                    .fetch_all(&h.pool).await?;
                let mut expected_keys = Vec::new();
                if cookies_claimed {
                    expected_keys.push((3801, "oteryn:item.tibia.i2970".to_owned(), 1));
                }
                if key_claimed {
                    expected_keys.push((3802, "oteryn:item.tibia.i2970".to_owned(), 1));
                }
                assert_eq!(keys, expected_keys);
                let roots: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r \
                     JOIN game_item_instances i USING(item_instance_id) ORDER BY i.definition_production_key LIMIT 3")
                    .fetch_all(&h.pool).await?;
                let mut expected_roots = Vec::new();
                if cookies_claimed {
                    expected_roots.push(("oteryn:item.tibia.i2853".to_owned(), 1));
                }
                if key_claimed {
                    expected_roots.push(("oteryn:item.tibia.i2970".to_owned(), 1));
                }
                assert_eq!(roots, expected_roots);
                if index == 1 {
                    let guard: String = sqlx::query_scalar(
                        "SELECT pg_get_functiondef('game_item_key_proven()'::regprocedure)",
                    )
                    .fetch_one(&h.pool)
                    .await?;
                    for arm in [
                        "game_reward_claim_container_item_proven",
                        "game_map_item_mint_receipts",
                        "game_reward_claim_inbox_item_proven",
                        "game_reward_claim_mint_receipts",
                    ] {
                        assert!(guard.contains(arm), "missing key provenance owner: {arm}");
                    }
                    // A completed, earlier bag MINT cannot authorize a fresh key.
                    let forged = sqlx::query(
                        "INSERT INTO game_item_keys(item_instance_id,key_number,minted_transaction_id) \
                         SELECT i.item_instance_id,3800,i.minted_transaction_id \
                         FROM game_reward_claim_mint_receipts r JOIN game_item_instances i USING(item_instance_id) \
                         WHERE i.definition_production_key='oteryn:item.tibia.i2853' LIMIT 1")
                        .execute(&h.pool).await;
                    let Err(sqlx::Error::Database(error)) = forged else {
                        return Err(format!("stale bag key proof must refuse: {forged:?}").into());
                    };
                    assert_eq!(error.code().as_deref(), Some("23514"));
                    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_keys")
                        .fetch_one(&h.pool)
                        .await?;
                    assert_eq!(count, 2);
                }
            }
            if matches!(case, Case::FireAxeBag | Case::FireAxeAll) {
                let bag_claimed = matches!(case, Case::FireAxeBag) || !reverse || index == 1;
                let axe_claimed = matches!(case, Case::FireAxeAll) && (reverse || index == 1);
                let children: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT i.definition_production_key,i.quantity FROM game_item_nested_entries e \
                     JOIN game_item_instances i USING(item_instance_id) ORDER BY i.definition_production_key LIMIT 5")
                    .fetch_all(&h.pool).await?;
                let expected_children = if bag_claimed {
                    vec![
                        ("oteryn:item.tibia.i3028".to_owned(), 7),
                        ("oteryn:item.tibia.i3085".to_owned(), 1),
                        ("oteryn:item.tibia.i3098".to_owned(), 1),
                        ("oteryn:item.tibia.i3320".to_owned(), 1),
                    ]
                } else {
                    Vec::new()
                };
                assert_eq!(children, expected_children);
                let roots: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r \
                     JOIN game_item_instances i USING(item_instance_id) ORDER BY i.definition_production_key LIMIT 3")
                    .fetch_all(&h.pool).await?;
                let mut expected_roots = Vec::new();
                if bag_claimed {
                    expected_roots.push(("oteryn:item.tibia.i2853".to_owned(), 1));
                }
                if axe_claimed {
                    expected_roots.push(("oteryn:item.tibia.i3320".to_owned(), 1));
                }
                assert_eq!(roots, expected_roots);
                for (track, claimed) in [("fireaxe1", bag_claimed), ("fireaxe2", axe_claimed)] {
                    let key = format!("oteryn:quest-progress/kv/{track}/claimed");
                    if claimed {
                        assert_eq!(snapshot.tracks().get(&key), Some(&1));
                    } else {
                        assert!(!snapshot.tracks().contains_key(&key));
                    }
                }
            }
            if matches!(
                case,
                Case::Koshei
                    | Case::BlackKnight
                    | Case::CrystalWand
                    | Case::CircleRoom
                    | Case::DemonaRing
                    | Case::DoubleHero
            ) {
                let rewards: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT i.definition_production_key,i.quantity FROM game_reward_claim_mint_receipts r \
                     JOIN game_item_instances i USING(item_instance_id) ORDER BY i.definition_production_key LIMIT 3")
                    .fetch_all(&h.pool).await?;
                let mut expected = Vec::new();
                for selected in 0..=index {
                    let pos = if reverse {
                        choices.len() - selected - 1
                    } else {
                        selected
                    };
                    let (item, count) = if matches!(case, Case::BlackKnight) {
                        (
                            if pos == 0 {
                                "oteryn:item.tibia.i3381"
                            } else {
                                "oteryn:item.tibia.i3419"
                            },
                            1,
                        )
                    } else if matches!(case, Case::DemonaRing) {
                        (
                            if pos == 0 {
                                "oteryn:item.tibia.i3049"
                            } else {
                                "oteryn:item.tibia.i3051"
                            },
                            1,
                        )
                    } else if matches!(case, Case::DoubleHero) {
                        (
                            if pos == 0 {
                                "oteryn:item.tibia.i3039"
                            } else {
                                "oteryn:item.tibia.i3093"
                            },
                            1,
                        )
                    } else if matches!(case, Case::CircleRoom) {
                        (
                            if pos == 0 {
                                "oteryn:item.tibia.i3279"
                            } else {
                                "oteryn:item.tibia.i3323"
                            },
                            1,
                        )
                    } else if matches!(case, Case::CrystalWand) {
                        (
                            if pos == 0 {
                                "oteryn:item.tibia.i3068"
                            } else {
                                "oteryn:item.tibia.i11609"
                            },
                            1,
                        )
                    } else if pos == 0 {
                        ("oteryn:item.tibia.i645", 1)
                    } else {
                        ("oteryn:item.tibia.i3035", 100)
                    };
                    expected.push((item.to_owned(), count));
                }
                expected.sort();
                assert_eq!(rewards, expected);
            }
            if wine {
                let fluids: Vec<(i64, i16, bool, bool)> = sqlx::query_as(
                    "SELECT i.quantity, f.fluid_subtype, m.root_transaction_id=r.transaction_id, \
                     f.last_transaction_id=m.transaction_id FROM game_reward_claim_mint_receipts r \
                     JOIN game_item_nested_entries e ON e.parent_item_instance_id=r.item_instance_id \
                     JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id \
                     JOIN game_item_fluid_states f ON f.item_instance_id=i.item_instance_id \
                     JOIN game_item_fluid_reward_mints m ON m.item_instance_id=i.item_instance_id \
                     WHERE i.definition_production_key='oteryn:item.tibia.i2874' LIMIT 2")
                    .fetch_all(&h.pool).await?;
                assert_eq!(fluids, vec![(1, 1, true, true)]);
                let children: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(children, 3);
                assert_eq!(
                    snapshot.tracks().get(
                        "oteryn:quest-progress/quest/u7_9/dreamers_challenge/chests_wine/claimed"
                    ),
                    Some(&1)
                );
                assert_eq!(
                    snapshot
                        .tracks()
                        .iter()
                        .filter(|(key, _)| key.ends_with("/claimed"))
                        .count(),
                    1
                );
            }
        }
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}
async fn walk(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    stream: &mut crate::gameplay_transport::connection::AdmittedSession,
    command: &mut u64,
    position: &mut MovementLocalPosition,
    destination: MovementLocalPosition,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    assert_eq!(position.floor, destination.floor);
    while *position != destination {
        let (direction, dx, dy) = if position.x < destination.x {
            (StepDirection::East, 1, 0)
        } else if position.x > destination.x {
            (StepDirection::West, -1, 0)
        } else if position.y < destination.y {
            (StepDirection::South, 0, 1)
        } else {
            (StepDirection::North, 0, -1)
        };
        let reply = super::quest_world_callbacks::wire(
            owner,
            stream,
            *command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        *command += 1;
        position.x += dx;
        position.y += dy;
    }
    Ok(())
}
