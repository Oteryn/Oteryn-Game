//! R–S source chests, literal production USE, exact rewards and durable once/replay.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::wire_use_targets;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

struct ChestCase {
    claim: &'static str,
    chest: (u16, u16, i8),
    unique: u16,
    appearance: u16,
    reward: &'static str,
    quantity: u32,
    contents: &'static [(&'static str, i64)],
    text: Option<&'static str>,
    charges: Option<u32>,
    written_children: bool,
}

fn plain(
    claim: &'static str,
    chest: (u16, u16, i8),
    unique: u16,
    appearance: u16,
    reward: &'static str,
    quantity: u32,
) -> ChestCase {
    ChestCase {
        claim,
        chest,
        unique,
        appearance,
        reward,
        quantity,
        contents: &[],
        text: None,
        charges: None,
        written_children: false,
    }
}

fn execute(case: ChestCase) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let claim = case.claim;
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "questpc3 source chest scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc3 claim={claim} duration_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

#[test]
fn questpc3_rapier_claim_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.rapier",
        (32099, 32198, -9),
        14042,
        2473,
        "oteryn:item.tibia.i3272",
        1,
    ))
}

#[test]
fn questpc3_ring_time_ring_claim_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.ringquest1",
        (33131, 31624, -15),
        6037,
        2469,
        "oteryn:item.tibia.i3053",
        1,
    ))
}

#[test]
fn questpc3_ring_sword_ring_claim_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.ringquest2",
        (33134, 31624, -15),
        6038,
        2469,
        "oteryn:item.tibia.i3091",
        1,
    ))
}

#[test]
fn questpc3_scale_armor_bag_once_and_replay() -> TestResult {
    execute(ChestCase {
        contents: &[("oteryn:item.tibia.i3377", 1)],
        ..plain(
            "oteryn:reward-claim.kv.scalearmor",
            (32357, 32130, -9),
            6153,
            2472,
            "oteryn:item.tibia.i2853",
            1,
        )
    })
}

#[test]
fn questpc3_shaman_treasure_bag_once_and_replay() -> TestResult {
    execute(ChestCase {
        contents: &[("oteryn:item.tibia.i3147", 3)],
        ..plain(
            "oteryn:reward-claim.kv.shamantreasure",
            (33127, 31885, -9),
            6069,
            4024,
            "oteryn:item.tibia.i2853",
            1,
        )
    })
}

#[test]
fn questpc3_short_sword_notebook_once_and_replay() -> TestResult {
    execute(ChestCase {
        text: Some(
            "Hardek *\nBozo *\nSam ****\nOswald\nPartos ***\nQuentin *\nTark ***\nHarsky ***\nStutch *\nFerumbras *\nFrodo **\nNoodles ****",
        ),
        ..plain(
            "oteryn:reward-claim.kv.shortswordbook",
            (32171, 32197, -7),
            6013,
            2472,
            "oteryn:item.tibia.i2821",
            1,
        )
    })
}

#[test]
fn questpc3_silver_brooch_full_bag_once_and_replay() -> TestResult {
    execute(ChestCase {
        contents: &[
            ("oteryn:item.tibia.i3017", 1),
            ("oteryn:item.tibia.i3030", 2),
            ("oteryn:item.tibia.i3028", 3),
        ],
        ..plain(
            "oteryn:reward-claim.kv.silverbrooch",
            (32775, 32006, -11),
            6155,
            2476,
            "oteryn:item.tibia.i2853",
            1,
        )
    })
}

#[test]
fn questpc3_six_rubies_claim_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.sixrubies",
        (32371, 32262, -12),
        6156,
        387,
        "oteryn:item.tibia.i3030",
        6,
    ))
}

#[test]
fn questpc3_spike_sword_claim_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.spikesword",
        (32568, 32085, -12),
        6171,
        3990,
        "oteryn:item.tibia.i3271",
        1,
    ))
}

// Crystal's storage namespace is ExplorerSociety; these two boxes are the
// independent Skull of Ratha quest rewards (wiki walkthrough, Witch Hill).
#[test]
fn questpc3_skull_of_ratha_skull_and_pearl_once_and_replay() -> TestResult {
    execute(ChestCase {
        contents: &[
            ("oteryn:item.tibia.i3026", 1),
            ("oteryn:item.tibia.i3207", 1),
        ],
        ..plain(
            "oteryn:reward-claim.quest.u7_6.explorer_society.skull_of_ratha.bag1",
            (32845, 31917, -6),
            6083,
            2469,
            "oteryn:item.tibia.i2853",
            1,
        )
    })
}

#[test]
fn questpc3_skull_of_ratha_chain_and_ring_once_and_replay() -> TestResult {
    execute(ChestCase {
        contents: &[
            ("oteryn:item.tibia.i3012", 1),
            ("oteryn:item.tibia.i3097", 1),
        ],
        ..plain(
            "oteryn:reward-claim.quest.u7_6.explorer_society.skull_of_ratha.bag2",
            (32847, 31917, -6),
            6084,
            2469,
            "oteryn:item.tibia.i2853",
            1,
        )
    })
}

#[test]
fn questpc3_skull_of_ratha_witch_house_full_bag_once_and_replay() -> TestResult {
    // The second part of Skull of Ratha's wiki walkthrough. No separate
    // Witch House quest title is present in the pinned 373-title catalogue.
    execute(ChestCase {
        contents: &[
            ("oteryn:item.tibia.i3027", 2),
            ("oteryn:item.tibia.i3008", 1),
            ("oteryn:item.tibia.i3031", 100),
        ],
        ..plain(
            "oteryn:reward-claim.kv.witch_house_quest",
            (32867, 31909, -8),
            6188,
            2472,
            "oteryn:item.tibia.i2853",
            1,
        )
    })
}

#[test]
fn questpc3_stealth_ring_ring_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.stealhring2",
        (33315, 32282, -11),
        6175,
        1983,
        "oteryn:item.tibia.i3049",
        1,
    ))
}

#[test]
fn questpc3_steel_helmet_first_coins_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.steelhelmet1",
        (32460, 31951, -5),
        6176,
        2434,
        "oteryn:item.tibia.i3031",
        56,
    ))
}

#[test]
fn questpc3_steel_helmet_second_coins_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.steelhelmet2",
        (32464, 31957, -5),
        6177,
        2469,
        "oteryn:item.tibia.i3031",
        47,
    ))
}

#[test]
fn questpc3_steel_helmet_helmet_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.steelhelmet3",
        (32462, 31947, -4),
        6178,
        2473,
        "oteryn:item.tibia.i3351",
        1,
    ))
}

#[test]
fn questpc3_steel_helmet_letter_once_and_replay() -> TestResult {
    execute(ChestCase {
        text: Some("Looks like the fox is out!\nMore luck next time!\nSigned:\nthe horned fox"),
        ..plain(
            "oteryn:reward-claim.kv.steelhelmet4",
            (32467, 31962, -4),
            6179,
            2472,
            "oteryn:item.tibia.i2815",
            1,
        )
    })
}

#[test]
fn questpc3_strong_potions_full_bag_once_and_replay() -> TestResult {
    execute(ChestCase {
        contents: &[("oteryn:item.tibia.i237", 5)],
        ..plain(
            "oteryn:reward-claim.quest.u8_1.strong_potions.reward",
            (33163, 31603, -15),
            6070,
            2469,
            "oteryn:item.tibia.i2857",
            1,
        )
    })
}

#[test]
fn questpc3_studded_shield_free_banana_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.bananafree",
        (32172, 32169, -7),
        6016,
        3639,
        "oteryn:item.tibia.i3587",
        1,
    ))
}

#[test]
fn questpc3_studded_shield_premium_banana_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.bananapremium",
        (31983, 32193, -5),
        6017,
        3639,
        "oteryn:item.tibia.i3587",
        1,
    ))
}

#[test]
fn questpc3_small_sapphire_full_recipe_once_and_replay() -> TestResult {
    execute(ChestCase {
        written_children: true,
        contents: &[
            ("oteryn:item.tibia.i3029", 1),
            ("oteryn:item.tibia.i3031", 50),
            ("oteryn:item.tibia.i2822", 1),
            ("oteryn:item.tibia.i637", 1),
        ],
        ..plain(
            "oteryn:reward-claim.kv.smallsapphire",
            (32962, 32429, -7),
            64993,
            2472,
            "oteryn:item.tibia.i2853",
            1,
        )
    })
}

#[test]
fn questpc3_studded_legs_honey_flower_once_and_replay() -> TestResult {
    execute(plain(
        "oteryn:reward-claim.kv.honeyflower",
        (32005, 32139, -3),
        6182,
        9226,
        "oteryn:item.tibia.i2984",
        1,
    ))
}

#[test]
fn questpc3_charged_amulet_cohort_once_and_replay() -> TestResult {
    for (claim, chest, unique, appearance, reward, charges) in [
        (
            "oteryn:reward-claim.kv.silveramulet",
            (32506, 32269, -9),
            6154,
            2469,
            "oteryn:item.tibia.i3054",
            200,
        ),
        (
            "oteryn:reward-claim.kv.stealhring1",
            (33315, 32277, -11),
            6174,
            1983,
            "oteryn:item.tibia.i3084",
            250,
        ),
    ] {
        execute(ChestCase {
            charges: Some(charges),
            ..plain(claim, chest, unique, appearance, reward, 1)
        })?;
    }
    Ok(())
}

fn scenario(case: ChestCase) -> TestResult {
    let ChestCase {
        claim,
        chest,
        unique,
        appearance,
        reward,
        quantity,
        contents,
        text,
        charges,
        written_children,
    } = case;
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "questpc3_reward_chests", false, false)
                .await?;
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
        let (world, source_ordinal) = committed_source_world(chest, unique, appearance)?;
        let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        assert!(
            report
                .served
                .iter()
                .any(|entry| entry.key.as_str() == claim),
            "{claim} did not bind: left_out={:?}, unbound={:?}",
            report.left_out.iter().find(|(key, _)| key == claim),
            report.unbound.iter().find(|entry| entry.claim == claim),
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
        let key =
            oteryn_world_bundle::bundle::placement_key(chest.2, chest.0, chest.1, source_ordinal)
                .ok_or("key")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // The second USE is a new command on the live connection. Rebuild the owner
        // for the replay so no Foundation command-result cache can satisfy it.
        let mut first_root = None;
        for (commands, expected) in [
            (
                &[1, 1, 2][..],
                &[
                    (CommandStatus::Accepted, UseDisposition::Committed),
                    (CommandStatus::DuplicateReplay, UseDisposition::Committed),
                    (CommandStatus::Accepted, UseDisposition::NothingToUse),
                ][..],
            ),
            (
                &[1][..],
                &[(CommandStatus::Accepted, UseDisposition::Committed)][..],
            ),
        ] {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = &cells;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            assert_eq!(
                wire_use_targets(
                    &owner,
                    admitted,
                    &commands.iter().map(|id| (*id, &target)).collect::<Vec<_>>()
                )
                .await
                .map_err(|e| format!("{claim} commands={commands:?}: {e}"))?,
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
            let root = &bag.entries[0].item;
            if let Some(charges) = charges {
                let policy = native
                    .item_policy_for_source_id(
                        reward
                            .strip_prefix("oteryn:item.tibia.i")
                            .ok_or("Item key")?
                            .parse()?,
                    )
                    .ok_or("charged Item")?;
                let definition = crate::content::ReferenceItemTarget::new(
                    &root.definition.production_key,
                    &root.definition.revision_ref,
                )
                .map_err(debug)?;
                let timed =
                    oteryn_gameplay_domain::domain::timed_item_host::resolve_timed_definition(
                        &definition,
                        &policy.record().semantics,
                    )
                    .map_err(debug)?
                    .ok_or("timed definition")?;
                assert_eq!(timed.full.charges(), Some(charges));
                // TIMED-ITEM-0 opens an absent row at revision zero with full
                // definition values, including after a cold reward replay.
                let rows: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid",
                )
                .bind(root.item_instance_id.as_slice())
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(rows, 0);
            }
            if let Some(original) = first_root {
                assert_eq!(root.item_instance_id, original);
            } else {
                first_root = Some(root.item_instance_id);
            }
            // A source recipe has at most twenty direct children (BAGS-0).
            let children: Vec<(String, i64)> = sqlx::query_as("SELECT i.definition_production_key,i.quantity \
                FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) \
                WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal LIMIT 21")
                .bind(root.item_instance_id.as_slice()).fetch_all(&harness.pool).await?;
            assert_eq!(
                children,
                contents
                    .iter()
                    .map(|(key, count)| ((*key).into(), *count))
                    .collect::<Vec<_>>()
            );
            let stored_text: Option<String> = sqlx::query_scalar(
                "SELECT text FROM game_item_texts \
                WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(root.item_instance_id.as_slice())
            .fetch_optional(&harness.pool)
            .await?;
            assert_eq!(stored_text.as_deref(), text);
            let child_texts: Vec<(String, String)> = sqlx::query_as(
                "SELECT i.definition_production_key,t.text FROM game_item_nested_entries e \
                 JOIN game_item_instances i USING(item_instance_id) \
                 JOIN game_item_texts t USING(item_instance_id) \
                 WHERE e.parent_item_instance_id=encode($1,'hex')::uuid \
                 ORDER BY e.placement_ordinal LIMIT 21",
            )
            .bind(root.item_instance_id.as_slice())
            .fetch_all(&harness.pool)
            .await?;
            let expected_texts = if written_children {
                let source: serde_json::Value = serde_json::from_str(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../tools/content-schema/reward-claim-authoring/small_sapphire_source.json"
                )))?;
                source["rewards"]
                    .as_array()
                    .ok_or("source rewards")?
                    .iter()
                    .filter_map(|row| {
                        Some((
                            format!("oteryn:item.tibia.i{}", row["item"].as_u64()?),
                            row.get("text")?.as_str()?.to_owned(),
                        ))
                    })
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            assert_eq!(child_texts, expected_texts);
        }
        let claims: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2), (SELECT count(*) FROM game_reward_claim_mint_receipts)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(claim).fetch_one(&harness.pool).await?;
        assert_eq!(claims, (1, 1));
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// The target tile and all its attributes come from the committed production B3
/// region. Only a walkable approach tile and a small bundle envelope are fixtures.
fn committed_source_world(
    chest: (u16, u16, i8),
    unique: u16,
    appearance: u16,
) -> TestResult<(crate::map::boot::BundleWorld, u8)> {
    use oteryn_world_bundle::{bundle, sector};
    use sha2::{Digest, Sha256};
    use std::fmt::Write;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/placements/index.json"),
    )?)?;
    let path = format!(
        "content/world/placements/region-z{:02}-x{:03}-y{:03}.b3",
        -chest.2,
        chest.0 / 256,
        chest.1 / 256
    );
    let recorded = index["regions"]
        .as_array()
        .ok_or("map regions")?
        .iter()
        .find(|row| row["path"].as_str() == Some(path.as_str()))
        .ok_or("source region")?;
    let raw = std::fs::read(root.join(&path))?;
    let mut digest = String::with_capacity(64);
    for byte in Sha256::digest(&raw) {
        write!(&mut digest, "{byte:02x}")?;
    }
    assert_eq!(recorded["sha256"].as_str(), Some(digest.as_str()));
    let mut budget = bundle::BUNDLE_BUDGET;
    let region =
        oteryn_world_bundle_compiler::b3::decode_region(&raw, bundle::TILE_LIMITS, &mut budget)?;
    let mut tile = region
        .sectors
        .into_iter()
        .flat_map(|(_, _, tiles)| tiles)
        .find(|tile| (tile.x, tile.y) == (chest.0, chest.1))
        .ok_or("source tile")?;
    let matches = |item: &sector::Item| {
        item.attrs.unique == Some(unique)
            && usize::try_from(item.palette)
                .ok()
                .and_then(|at| index["palette"].get(at))
                .and_then(|row| row["source_item_id"].as_u64())
                == Some(u64::from(appearance))
    };
    assert_eq!(
        tile.items
            .iter()
            .filter(|item| item.depth == 0 && matches(item))
            .count(),
        1
    );
    let ordinal = tile
        .items
        .iter()
        .filter(|item| item.depth == 0)
        .position(matches)
        .ok_or("chest marker")?;
    let source = tile
        .items
        .iter()
        .filter(|item| item.depth == 0)
        .nth(ordinal)
        .ok_or("source chest entry")?;
    assert_eq!(
        index["palette"][usize::try_from(source.palette)?]["source_item_id"].as_u64(),
        Some(u64::from(appearance))
    );
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let mut palette = std::collections::BTreeMap::new();
    for item in &mut tile.items {
        let source_id = index["palette"][usize::try_from(item.palette)?]["source_item_id"]
            .as_u64()
            .ok_or("source appearance")?;
        let local = match palette.get(&source_id) {
            Some(local) => *local,
            None => {
                let local = u32::try_from(data.manifest.palette.len())?;
                data.manifest.palette.push(bundle::PaletteEntry {
                    key: format!("oteryn:terrain.tibia.i{source_id}"),
                    family: bundle::Family::Terrain,
                    id: local,
                    terrain: Some(bundle::Terrain {
                        kind: bundle::TerrainKind::Common,
                        walkable: None,
                        ground_speed: None,
                    }),
                });
                palette.insert(source_id, local);
                local
            }
        };
        item.palette = local;
    }
    data.manifest.world = bundle::Extent {
        min_x: chest.0 - 1,
        min_y: chest.1,
        max_x: chest.0 + 1,
        max_y: chest.1 + 1,
        floors: vec![chest.2],
    };
    let approach = sector::Tile {
        x: chest.0 - 1,
        y: chest.1,
        flags: 0,
        house: 0,
        zones: Vec::new(),
        items: vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: sector::Attrs::default(),
        }],
    };
    let mut grouped = std::collections::BTreeMap::<u16, Vec<sector::Tile>>::new();
    for tile in [approach, tile] {
        grouped.entry(tile.x / 32).or_default().push(tile);
    }
    let sectors = grouped
        .into_iter()
        .map(|(sx, tiles)| bundle::Sector {
            floor: chest.2,
            sx,
            sy: chest.1 / 32,
            tiles,
        })
        .collect::<Vec<_>>();
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let world = crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: chest.0 - 1,
                y: chest.1,
                floor: chest.2,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)?;
    Ok((world, u8::try_from(ordinal)?))
}
