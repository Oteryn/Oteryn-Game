//! T..Z reward claims: source placements -> wire USE -> durable custody and replay.
//! This certifies the reward path, not any untested access puzzle or NPC mission.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::{source_world_positions, wire_use_targets_retained};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[derive(serde::Deserialize)]
struct Chest {
    claim: String,
    position: (u16, u16, i8),
    unique: u16,
    appearance: u16,
    root: String,
    quantity: u32,
    children: Vec<(String, i64)>,
    charges: Vec<(String, u32)>,
    key_number: Option<u16>,
    root_text: Option<String>,
    achievement: Option<String>,
    source: String,
}

#[derive(serde::Deserialize)]
struct Quest {
    title: String,
    chests: Vec<Chest>,
}

fn cases() -> TestResult<Vec<Quest>> {
    Ok(serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/quest-authoring/questpc4/reward-cases.json"
    )))?)
}

fn execute(title: &str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let quest = cases()?
        .into_iter()
        .find(|q| q.title == title)
        .ok_or("quest case")?;
    assert!(!quest.chests.is_empty(), "{title}: no execution cases");
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .name("source-quest-rewards".into())
        .stack_size(256 << 20)
        .spawn(move || {
            for chest in &quest.chests {
                scenario(chest, &quest.chests).map_err(|error| error.to_string())?;
            }
            Ok::<(), String>(())
        })?
        .join()
        .map_err(|_| "source quest reward scenario panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED source_quest_rewards {title} duration={:?}",
        started.elapsed()
    );
    Ok(())
}

macro_rules! quest_case {
    ($test:ident, $title:literal) => {
        #[test]
        fn $test() -> TestResult {
            execute($title)
        }
    };
}

quest_case!(thais_lighthouse_rewards, "Thais Lighthouse Quest");
quest_case!(the_annihilator_rewards, "The Annihilator Quest");
quest_case!(the_ape_city_rewards, "The Ape City Quest");
quest_case!(the_desert_dungeon_rewards, "The Desert Dungeon Quest");
quest_case!(the_explorer_society_rewards, "The Explorer Society Quest");
quest_case!(
    the_hidden_city_of_beregar_rewards,
    "The Hidden City of Beregar Quest"
);
quest_case!(the_ice_islands_rewards, "The Ice Islands Quest");
quest_case!(the_inquisition_rewards, "The Inquisition Quest");
quest_case!(the_new_frontier_rewards, "The New Frontier Quest");
quest_case!(the_outlaw_camp_rewards, "The Outlaw Camp Quest");
quest_case!(the_paradox_tower_rewards, "The Paradox Tower Quest");
quest_case!(the_pits_of_inferno_rewards, "The Pits of Inferno Quest");
quest_case!(the_shattered_isles_rewards, "The Shattered Isles Quest");
quest_case!(the_thieves_guild_rewards, "The Thieves Guild Quest");
quest_case!(
    the_white_raven_monastery_rewards,
    "The White Raven Monastery Quest"
);
quest_case!(threatened_dreams_rewards, "Threatened Dreams Quest");
quest_case!(throwing_star_rewards, "Throwing Star Quest");
quest_case!(time_ring_rewards, "Time Ring Quest");
quest_case!(to_blind_the_enemy_rewards, "To Blind the Enemy Quest");
quest_case!(to_outfox_a_fox_rewards, "To Outfox a Fox Quest");
quest_case!(tomes_of_knowledge_rewards, "Tomes of Knowledge Quest");
quest_case!(triangle_tower_rewards, "Triangle Tower Quest");
quest_case!(troll_cave_rewards, "Troll Cave Quest");
quest_case!(vampire_hunter_rewards, "Vampire Hunter Quest");
quest_case!(vampire_shield_rewards, "Vampire Shield Quest");
quest_case!(waterfall_rewards, "Waterfall Quest");
quest_case!(wedding_ring_rewards, "Wedding Ring Quest");
quest_case!(what_a_foolish_rewards, "What a Foolish Quest");

fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

fn scenario(chest: &Chest, cohort: &[Chest]) -> TestResult {
    let choices: Vec<_> = cohort
        .iter()
        .filter(|other| other.claim == chest.claim)
        .map(|other| {
            assert_eq!(other.position.2, chest.position.2);
            assert_eq!(other.appearance, chest.appearance);
            (other.position.0, other.position.1, other.unique)
        })
        .collect();
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "source_quest_rewards", false, false)
            .await?;
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
        let world = source_world_positions(chest.position, &choices, chest.appearance)?;
        let bound = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        assert!(
            bound.served.iter().any(|c| c.key.as_str() == chest.claim),
            "{} did not bind; left out: {:?}",
            chest.claim,
            bound
                .left_out
                .iter()
                .filter(|(key, _)| key == &chest.claim)
                .collect::<Vec<_>>()
        );
        use oteryn_gameplay_quests::quest::world_callbacks;
        let mut world_claim_callback = None;
        for callback in world_callbacks::load()? {
            if callback.selector.position
                != Some(world_callbacks::Position {
                    x: chest.position.0,
                    y: chest.position.1,
                    floor: chest.position.2,
                })
                || callback.selector.appearance != Some(chest.appearance)
            {
                continue;
            }
            if world_callbacks::legacy_world_claim(&callback)?
                .is_some_and(|claim| claim.key == chest.claim)
                && world_claim_callback.replace(callback).is_some()
            {
                return Err("source claim has more than one canonical World callback".into());
            }
        }
        let has_claim_obligation = bound.served.iter().any(|claim| {
            claim.key.as_str() == chest.claim
                && claim
                    .placements
                    .iter()
                    .any(|placement| placement.placement.quest_transition.is_some())
        });
        let catalogue = if has_claim_obligation || world_claim_callback.is_some() {
            let lowered =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            if let Some(callback) = &world_claim_callback {
                let transition = world_callbacks::primary_transition(callback)?;
                assert_eq!(
                    lowered.catalogue().transition(&callback.key),
                    Some(&transition)
                );
            }
            Some(std::sync::Arc::new(lowered.catalogue().clone()))
        } else {
            None
        };
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let at = MovementLocalPosition {
            x: i32::from(chest.position.0) - 1,
            y: i32::from(chest.position.1),
            floor: -i16::from(chest.position.2),
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
        let key = oteryn_world_bundle::bundle::placement_key(
            chest.position.2,
            chest.position.0,
            chest.position.1,
            1,
        )
        .ok_or("placement")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        {
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.quest_catalogue = catalogue.clone();
            owner.admit_progression_session(&mut admitted).await;
            if catalogue.is_some() {
                owner.admit_quest_session(&admitted).await;
            }
            deps.initialize(&owner).await;
            let mut stale = admitted;
            let mut stale_fence = fence;
            stale_fence.character_lease_generation += 1;
            stale.item_fence = Some(stale_fence);
            assert_eq!(
                super::quest_callbacks::wire_uses(&owner, stale, &[1], &target).await?,
                [UseDisposition::Rejected],
                "{} UID{} {}",
                chest.claim,
                chest.unique,
                chest.source
            );
            let reservations: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(reservations, 0);
        }
        let mut first_root = None;
        for cold in [false, true] {
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.quest_catalogue = catalogue.clone();
            let mut stream = admitted;
            owner.admit_progression_session(&mut stream).await;
            if catalogue.is_some() {
                owner.admit_quest_session(&stream).await;
            }
            deps.initialize(&owner).await;
            use oteryn_protocol_oteryn::world_spatial::{
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition,
                decode_step_result, encode_step_intent,
            };
            for (command, direction, x) in [
                (1, StepDirection::West, at.x - 1),
                (2, StepDirection::East, at.x),
            ] {
                let reply = super::quest_world_callbacks::wire(
                    &owner,
                    &mut stream,
                    command,
                    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                    encode_step_intent(direction),
                )
                .await?;
                assert_eq!(
                    decode_step_result(&reply).map_err(debug)?,
                    StepDisposition::Moved
                );
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    MovementLocalPosition { x, ..at }
                );
            }
            let expected = if cold {
                vec![(CommandStatus::Accepted, UseDisposition::Committed)]
            } else {
                vec![
                    (CommandStatus::Accepted, UseDisposition::Committed),
                    (CommandStatus::DuplicateReplay, UseDisposition::Committed),
                    (
                        CommandStatus::Accepted,
                        if world_claim_callback.is_some() {
                            UseDisposition::Rejected
                        } else {
                            UseDisposition::NothingToUse
                        },
                    ),
                ]
            };
            let commands = if cold {
                vec![(3, &target)]
            } else {
                vec![(3, &target), (3, &target), (4, &target)]
            };
            assert_eq!(
                wire_use_targets_retained(&owner, &mut stream, &commands).await?,
                expected,
                "{} uid={} cold={cold}",
                chest.claim,
                chest.unique
            );
            if !cold {
                let mut command = 5;
                let mut position = at;
                // Visit every other exact source choice, then return for cold replay.
                for &(x, y, unique) in choices
                    .iter()
                    .filter(|(_, _, unique)| *unique != chest.unique)
                {
                    walk(
                        &owner,
                        &mut stream,
                        &mut command,
                        &mut position,
                        MovementLocalPosition {
                            x: i32::from(x) - 1,
                            y: i32::from(y),
                            ..at
                        },
                    )
                    .await?;
                    assert_eq!(
                        deps.runtime
                            .lock()
                            .await
                            .read_actor_position(deps.actor)
                            .map_err(debug)?
                            .position(),
                        position,
                        "{} approach UID{unique}",
                        chest.claim
                    );
                    let key = oteryn_world_bundle::bundle::placement_key(chest.position.2, x, y, 1)
                        .ok_or("alternate placement")?;
                    let mut placement = world.base().digest().to_vec();
                    placement.extend_from_slice(&key.to_be_bytes());
                    let alternate = WorldObjectTarget {
                        placement,
                        expected_revision: 0,
                    };
                    assert_eq!(
                        wire_use_targets_retained(
                            &owner,
                            &mut stream,
                            &[(command, &alternate), (command, &alternate)]
                        )
                        .await?,
                        [
                            (CommandStatus::Accepted, UseDisposition::NothingToUse),
                            (CommandStatus::DuplicateReplay, UseDisposition::NothingToUse),
                        ],
                        "{} source choice UID{unique}",
                        chest.claim
                    );
                    command += 1;
                }
                walk(&owner, &mut stream, &mut command, &mut position, at).await?;
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    at
                );
            }
            let backpack = h
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("backpack")?;
            assert_eq!(backpack.entries.len(), 1);
            let root = &backpack.entries[0].item;
            assert_eq!(root.definition.production_key, chest.root);
            assert_eq!(root.quantity, chest.quantity);
            if let Some(number) = chest.key_number {
                let key: (i32, bool, bool) = sqlx::query_as("SELECT k.key_number, \
                    k.minted_transaction_id=r.transaction_id,k.created_xact_id=r.created_xact_id \
                    FROM game_item_keys k JOIN game_reward_claim_mint_receipts r USING(item_instance_id) \
                    WHERE k.item_instance_id=encode($1,'hex')::uuid")
                    .bind(root.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
                assert_eq!(key, (i32::from(number), true, true), "{}", chest.source);
            }
            if let Some(expected) = &chest.root_text {
                let text: (String, i64, bool) = sqlx::query_as("SELECT text,text_revision::bigint,writer_character_id IS NULL FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid")
                    .bind(root.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
                assert_eq!((&text.0, text.1, text.2), (expected, 1, true));
            }
            for (key, expected) in &chest.charges {
                use crate::content::ReferenceItemField::Known;
                let policy = native
                    .item_policy(key, "definition-r1")
                    .ok_or("charge profile")?;
                assert!(
                    matches!(&policy.record().semantics.charges,
                    Known(charges) if charges.count == Known(*expected)),
                    "{}",
                    chest.source
                );
                let charged: Vec<(i64, bool)> = sqlx::query_as("SELECT i.quantity, \
                    EXISTS(SELECT 1 FROM game_item_timed_states t WHERE t.item_instance_id=i.item_instance_id) \
                    FROM game_item_instances i WHERE i.definition_production_key=$2 AND \
                    (i.item_instance_id=encode($1,'hex')::uuid OR i.item_instance_id IN \
                    (SELECT item_instance_id FROM game_item_nested_entries \
                    WHERE parent_item_instance_id=encode($1,'hex')::uuid))")
                    .bind(root.item_instance_id.as_slice()).bind(key).fetch_all(&h.pool).await?;
                assert_eq!(
                    charged,
                    [(1, false)],
                    "{}: full definition charges",
                    chest.source
                );
            }
            if let Some(original) = first_root {
                assert_eq!(root.item_instance_id, original);
            } else {
                first_root = Some(root.item_instance_id);
            }
            let children: Vec<(String, i64)> = sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY e.placement_ordinal")
                .bind(root.item_instance_id.as_slice()).fetch_all(&h.pool).await?;
            assert_eq!(children, chest.children);
            let counts: (i64,i64,i64,i64) = sqlx::query_as("SELECT \
                (SELECT count(*) FROM game_reward_claims WHERE claim_production_key=$1), \
                (SELECT count(*) FROM game_reward_claim_mint_receipts), \
                (SELECT count(*) FROM game_item_audit_outbox WHERE item_instance_id=encode($2,'hex')::uuid OR item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid)), \
                (SELECT count(DISTINCT created_xact_id::text) FROM game_item_audit_outbox WHERE item_instance_id=encode($2,'hex')::uuid OR item_instance_id IN (SELECT item_instance_id FROM game_item_nested_entries WHERE parent_item_instance_id=encode($2,'hex')::uuid))")
                .bind(&chest.claim).bind(root.item_instance_id.as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(counts, (1, 1, 1 + i64::try_from(chest.children.len())?, 1));
            let achievements: Vec<String> = sqlx::query_scalar(
                "SELECT achievement_key FROM game_account_achievements ORDER BY achievement_key LIMIT 2",
            )
            .fetch_all(&h.pool)
            .await?;
            assert_eq!(
                achievements,
                chest.achievement.iter().cloned().collect::<Vec<_>>(),
                "{}: exact achievement grant persists across replay",
                chest.source
            );
            if catalogue.is_some() {
                let quest_counts: (i64, i64, i64) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM game_character_quest_receipts), \
                    (SELECT count(*) FROM game_character_quest_obligations WHERE state='CONSUMED'), \
                    (SELECT count(*) FROM game_character_quest_obligations WHERE state <> 'CONSUMED')",
                )
                .fetch_one(&h.pool)
                .await?;
                if let Some(callback) = &world_claim_callback {
                    assert_eq!(quest_counts, (1, 0, 0), "{}", chest.claim);
                    let storage = callback.storage.first().ok_or("source World claim track")?;
                    let value: i64 = sqlx::query_scalar(
                        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1",
                    )
                    .bind(&storage.track)
                    .fetch_one(&h.pool)
                    .await?;
                    assert_eq!(value, storage.set);
                    let coupled: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM game_character_quest_receipts q \
                         JOIN game_reward_claim_mint_receipts r ON r.character_id=q.character_id \
                         AND r.game_session_id=q.cause_id AND r.command_id=q.cause_ordinal \
                         AND r.created_xact_id=q.created_xact_id \
                         WHERE q.transition_key=$1 AND r.claim_production_key=$2)",
                    )
                    .bind(&callback.key)
                    .bind(&chest.claim)
                    .fetch_one(&h.pool)
                    .await?;
                    assert!(
                        coupled,
                        "source World progress and original claim MINT are atomic"
                    );
                } else {
                    assert_eq!(quest_counts, (1, 1, 0), "{}", chest.claim);
                }
            }
        }
        drop(authority);
        drop(seal);
        h.cleanup().await
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
