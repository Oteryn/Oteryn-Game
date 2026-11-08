//! QREADY2: owned reward-only placements through literal dispatch USE.
//! Reuses the QUESTPC2/3/4/7 source-selector and once/replay scenario shape.
//! Knight Outfits remains owned by QENGINE2 and has no case here.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::{source_world, wire_use_targets, wire_uses};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[derive(Clone, Copy)]
struct Case {
    claim: &'static str,
    chest: (u16, u16, i8),
    unique: u16,
    appearance: u16,
    reward: &'static str,
    quantity: u32,
}

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

fn execute(case: Case, marker: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "qready2 scenario panicked")??;
    println!(
        "POSTGRES_CASE_COMPLETED {marker} duration={:.3}s",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

fn scenario(case: Case) -> TestResult {
    let Case {
        claim,
        chest,
        unique,
        appearance,
        reward,
        quantity,
    } = case;
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "qready2_claim", false, false).await?;
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
        let world = source_world(chest, unique, appearance, 0)?;
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        assert!(
            report.served.iter().any(|row| row.key.as_str() == claim),
            "source claim did not bind"
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
                i32::from(chest.0) - 1,
                i32::from(chest.1),
                -i16::from(chest.2),
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
                    x: i32::from(chest.0) - 1,
                    y: i32::from(chest.1),
                    floor: -i16::from(chest.2),
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
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated_pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
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
            definition: definition("oteryn:item.tibia.i2854"),
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
        let target_at = |x| -> TestResult<WorldObjectTarget> {
            let key = oteryn_world_bundle::bundle::placement_key(chest.2, x, chest.1, 1)
                .ok_or("placement")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            Ok(WorldObjectTarget {
                placement,
                expected_revision: 0,
            })
        };
        let target = target_at(chest.0)?;
        {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let mut stale = admitted;
            let mut stale_fence = fence;
            stale_fence.character_lease_generation += 1;
            stale.item_fence = Some(stale_fence);
            assert_eq!(
                wire_uses(&owner, stale, &[1], &target).await?,
                [UseDisposition::Rejected]
            );
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                count, 0,
                "stale authority cannot reserve any part of the reward"
            );
        }
        let mut original = None;
        for cold in [false, true] {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            let commands = if cold {
                vec![(1, &target)]
            } else {
                vec![(1, &target), (1, &target), (2, &target)]
            };
            let expected = if cold {
                vec![(CommandStatus::Accepted, UseDisposition::Committed)]
            } else {
                vec![
                    (CommandStatus::Accepted, UseDisposition::Committed),
                    (CommandStatus::DuplicateReplay, UseDisposition::Committed),
                    (CommandStatus::Accepted, UseDisposition::NothingToUse),
                ]
            };
            assert_eq!(
                wire_use_targets(&owner, admitted, &commands).await?,
                expected
            );
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("backpack")?;
            assert_eq!(bag.entries.len(), 1, "exactly one reward stack");
            let item = &bag.entries[0].item;
            assert_eq!(item.definition.production_key, reward);
            assert_eq!(item.quantity, quantity);
            if let Some(first) = original {
                assert_eq!(item.item_instance_id, first);
            } else {
                original = Some(item.item_instance_id);
            }
            let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE claim_production_key=$1), (SELECT count(*) FROM game_reward_claim_mint_receipts), (SELECT count(*) FROM game_item_audit_outbox WHERE item_instance_id=encode($2,'hex')::uuid)")
                .bind(claim).bind(item.item_instance_id.as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(
                counts,
                (1, 1, 1),
                "claim, receipt and mint occur exactly once"
            );
        }
        Ok(())
    })
}

#[test]
fn qready2_heaven_blossom_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.heavenblossom",
            chest: (33104, 32154, -8),
            unique: 6143,
            appearance: 2523,
            reward: "oteryn:item.tibia.i5921",
            quantity: 1,
        },
        "QREADY2_HEAVEN_BLOSSOM_QUEST_1",
    )
}

#[test]
fn qready2_iron_hammer_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.ironhamer",
            chest: (32434, 31938, -8),
            unique: 6172,
            appearance: 2473,
            reward: "oteryn:item.tibia.i3310",
            quantity: 1,
        },
        "QREADY2_IRON_HAMMER_QUEST_1",
    )
}

#[test]
fn qready2_iron_ore_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.ironore",
            chest: (32532, 31873, -8),
            unique: 6144,
            appearance: 2472,
            reward: "oteryn:item.tibia.i5880",
            quantity: 1,
        },
        "QREADY2_IRON_ORE_QUEST_1",
    )
}

#[test]
fn qready2_isle_of_the_mists_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.isleofthemists",
            chest: (32852, 32332, -7),
            unique: 6145,
            appearance: 2469,
            reward: "oteryn:item.tibia.i3032",
            quantity: 1,
        },
        "QREADY2_ISLE_OF_THE_MISTS_QUEST_1",
    )
}

#[test]
fn qready2_lone_medusa_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.quest.u8_5.lone_medusa",
            chest: (32406, 32764, -1),
            unique: 6286,
            appearance: 3990,
            reward: "oteryn:item.tibia.i3027",
            quantity: 4,
        },
        "QREADY2_LONE_MEDUSA_QUEST_1",
    )
}

#[test]
fn qready2_minotaur_leather_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.minotaurleatherquest",
            chest: (32302, 32415, -7),
            unique: 6149,
            appearance: 2469,
            reward: "oteryn:item.tibia.i5878",
            quantity: 1,
        },
        "QREADY2_MINOTAUR_LEATHER_QUEST_1",
    )
}

#[test]
fn qready2_mintwallin_cyclops_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.mintwallincyclops",
            chest: (32591, 32097, -14),
            unique: 6138,
            appearance: 2472,
            reward: "oteryn:item.tibia.i3028",
            quantity: 1,
        },
        "QREADY2_MINTWALLIN_CYCLOPS_QUEST_1",
    )
}

#[test]
fn qready2_naginata_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.naginata",
            chest: (32346, 32063, -12),
            unique: 6135,
            appearance: 2472,
            reward: "oteryn:item.tibia.i3314",
            quantity: 1,
        },
        "QREADY2_NAGINATA_QUEST_1",
    )
}

#[test]
fn qready2_nightmare_teddy_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.nightmare_teddy_quest",
            chest: (33444, 32605, -11),
            unique: 6305,
            appearance: 2433,
            reward: "oteryn:item.tibia.i21982",
            quantity: 1,
        },
        "QREADY2_NIGHTMARE_TEDDY_QUEST_1",
    )
}

#[test]
fn qready2_noble_armor_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.noblearmor1",
            chest: (32451, 32048, -8),
            unique: 6132,
            appearance: 2472,
            reward: "oteryn:item.tibia.i3380",
            quantity: 1,
        },
        "QREADY2_NOBLE_ARMOR_QUEST_1",
    )
}

#[test]
fn qready2_noble_armor_quest_2() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.noblearmor2",
            chest: (32455, 32048, -8),
            unique: 6133,
            appearance: 2472,
            reward: "oteryn:item.tibia.i3385",
            quantity: 1,
        },
        "QREADY2_NOBLE_ARMOR_QUEST_2",
    )
}

#[test]
fn qready2_orc_fortress_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.orcfortress1",
            chest: (32980, 31727, -9),
            unique: 6020,
            appearance: 2469,
            reward: "oteryn:item.tibia.i3318",
            quantity: 1,
        },
        "QREADY2_ORC_FORTRESS_QUEST_1",
    )
}

#[test]
fn qready2_orc_fortress_quest_2() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.orcfortress2",
            chest: (32981, 31727, -9),
            unique: 6021,
            appearance: 2469,
            reward: "oteryn:item.tibia.i3370",
            quantity: 1,
        },
        "QREADY2_ORC_FORTRESS_QUEST_2",
    )
}

#[test]
fn qready2_orc_fortress_quest_3() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.orcfortress3",
            chest: (32985, 31727, -9),
            unique: 6022,
            appearance: 2469,
            reward: "oteryn:item.tibia.i3280",
            quantity: 1,
        },
        "QREADY2_ORC_FORTRESS_QUEST_3",
    )
}

#[test]
fn qready2_plate_armor_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.platearmor",
            chest: (33327, 32182, -7),
            unique: 6173,
            appearance: 2474,
            reward: "oteryn:item.tibia.i3357",
            quantity: 1,
        },
        "QREADY2_PLATE_ARMOR_QUEST_1",
    )
}




#[test]
fn qready2_six_rubies_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.sixrubies",
            chest: (32371, 32262, -12),
            unique: 6156,
            appearance: 387,
            reward: "oteryn:item.tibia.i3030",
            quantity: 6,
        },
        "QREADY2_SIX_RUBIES_QUEST_1",
    )
}

#[test]
fn qready2_spike_sword_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.spikesword",
            chest: (32568, 32085, -12),
            unique: 6171,
            appearance: 3990,
            reward: "oteryn:item.tibia.i3271",
            quantity: 1,
        },
        "QREADY2_SPIKE_SWORD_QUEST_1",
    )
}

#[test]
fn qready2_studded_legs_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.honeyflower",
            chest: (32005, 32139, -3),
            unique: 6182,
            appearance: 9226,
            reward: "oteryn:item.tibia.i2984",
            quantity: 1,
        },
        "QREADY2_STUDDED_LEGS_QUEST_1",
    )
}

#[test]
fn qready2_studded_shield_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.bananafree",
            chest: (32172, 32169, -7),
            unique: 6016,
            appearance: 3639,
            reward: "oteryn:item.tibia.i3587",
            quantity: 1,
        },
        "QREADY2_STUDDED_SHIELD_QUEST_1",
    )
}

#[test]
fn qready2_studded_shield_quest_2() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.bananapremium",
            chest: (31983, 32193, -5),
            unique: 6017,
            appearance: 3639,
            reward: "oteryn:item.tibia.i3587",
            quantity: 1,
        },
        "QREADY2_STUDDED_SHIELD_QUEST_2",
    )
}

#[test]
fn qready2_throwing_star_quest_1() -> TestResult {
    execute(
        Case {
            claim: "oteryn:reward-claim.kv.throwingstar",
            chest: (32522, 32111, -15),
            unique: 6159,
            appearance: 2473,
            reward: "oteryn:item.tibia.i3287",
            quantity: 10,
        },
        "QREADY2_THROWING_STAR_QUEST_1",
    )
}
