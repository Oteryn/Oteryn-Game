//! The Isle of Evil introduction and rewards through the live NPC and durable quest writers.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{isle_of_evil as quest, world_callbacks::Position};

#[derive(Clone, Copy)]
enum Case {
    Introduction,
    Exchange(Exchange),
    KingReward,
}

#[test]
fn direct_isle_keyword_opens_the_mission() -> TestResult {
    run_case("qcx86-direct-intro", Case::Introduction)
}

#[derive(Clone, Copy)]
struct Exchange {
    npc: &'static str,
    words: &'static [&'static str],
    changes: &'static [(&'static str, i64, i64)],
    give: Option<u16>,
    take: Option<u16>,
}

#[test]
fn rapanaio_grants_one_mechanical_fishing_rod() -> TestResult {
    run_case(
        "qcx86-fishing-rod",
        Case::Exchange(Exchange {
            npc: "oteryn:npc.rapanaio",
            words: &["mission", "yes"],
            changes: &[("questline", 1, 2), ("mission01", 0, 1)],
            give: Some(9306),
            take: None,
        }),
    )
}

#[test]
fn rapanaio_accepts_a_mechanical_fish_without_fishing_credit() -> TestResult {
    run_case(
        "qcx86-fish",
        Case::Exchange(Exchange {
            npc: "oteryn:npc.rapanaio",
            words: &["mission", "yes"],
            changes: &[("questline", 2, 3), ("mission01", 1, 2)],
            give: None,
            take: Some(9307),
        }),
    )
}

#[test]
fn scutty_lends_the_worm_punisher_and_opens_the_door() -> TestResult {
    run_case(
        "qcx86-hammer",
        Case::Exchange(Exchange {
            npc: "oteryn:npc.scutty",
            words: &["machine", "yes", "yes"],
            changes: &[("mission04", 1, 2), ("door_04", 0, 1)],
            give: Some(9234),
            take: None,
        }),
    )
}

#[test]
fn scutty_grants_the_machine_crate_after_the_worm_task() -> TestResult {
    run_case(
        "qcx86-crate",
        Case::Exchange(Exchange {
            npc: "oteryn:npc.scutty",
            words: &["mission"],
            changes: &[("mission04", 3, 4)],
            give: Some(9390),
            take: None,
        }),
    )
}

#[test]
fn rapanaio_accepts_the_machine_crate() -> TestResult {
    run_case(
        "qcx86-machine",
        Case::Exchange(Exchange {
            npc: "oteryn:npc.rapanaio",
            words: &["mission", "yes"],
            changes: &[("questline", 8, 9), ("mission04", 4, 5)],
            give: None,
            take: Some(9390),
        }),
    )
}

#[test]
fn rapanaio_accepts_the_nautical_map() -> TestResult {
    run_case(
        "qcx86-map",
        Case::Exchange(Exchange {
            npc: "oteryn:npc.rapanaio",
            words: &["mission", "yes"],
            changes: &[("questline", 10, 11), ("mission05", 2, 3)],
            give: None,
            take: Some(9308),
        }),
    )
}

#[test]
fn king_tibianus_grants_the_global_reward_once() -> TestResult {
    run_case("qcx86-king-reward", Case::KingReward)
}

fn run_case(name: &'static str, case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || scenario(case));
    eprintln!("{name} duration={:?}", started.elapsed());
    result
}

fn scenario(case: Case) -> TestResult {
    let trigger = Position {
        x: 33294,
        y: 31677,
        floor: -15,
    };
    let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key.ends_with("behemoth"))
        .ok_or("terrain fixture")?;
    let world = super::quest_world_callbacks::world(&callback, trigger)?;
    let local = super::quest_world_callbacks::local;
    run(async move |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx86", false, false).await?;
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
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(trigger.x) - 1,
                i32::from(trigger.y),
                -i16::from(trigger.floor),
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
            4,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = local(Position {
            x: trigger.x - 1,
            ..trigger
        });
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let native = generation.native_gameplay().ok_or("native")?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(debug)?;
        let questline = format!("{}questline", quest::PREFIX);
        let mission07 = format!("{}mission07", quest::PREFIX);
        let initial: &[(&str, i64, i64)] = match case {
            Case::Introduction => &[],
            Case::Exchange(exchange) => exchange.changes,
            Case::KingReward => &[("questline", 15, 16), ("mission07", 3, 4)],
        };
        for (suffix, value, _) in initial {
            let track = format!("{}{suffix}", quest::PREFIX);
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).bind(track).bind(quest::QUEST).bind(value)
                .execute(&h.pool).await?;
        }
        if !initial.is_empty() {
            // Recorded from origin/main's embedded catalogue at qcx86's base SHA.
            // These characters started the quest before the NPC reward callbacks.
            let old_hash = "573e29f8cffaaffbe4014ccef1b899159e09bfaa39fd2b6486f865c8072dff0d";
            sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) SELECT character_id,$2,'content-1',decode($3,'hex'),character_revision,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).bind(quest::QUEST).bind(old_hash)
                .execute(&h.pool).await?;
        }
        let input = match case {
            Case::Introduction => None,
            Case::Exchange(exchange) => exchange.take,
            Case::KingReward => Some(9391),
        };
        if let Some(item) = input {
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($2,'hex')::uuid,world_id,'Item',$3,'definition-r1',1,1,encode($2,'hex')::uuid FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).bind(id(210).as_slice()).bind(format!("oteryn:item.tibia.i{item}")).execute(&mut *tx).await?;
            let inserted = sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT encode($2,'hex')::uuid,world_id,character_id,item_instance_id,1,encode($2,'hex')::uuid FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).bind(id(210).as_slice()).execute(&mut *tx).await?;
            assert_eq!(inserted.rows_affected(), 1);
            tx.commit().await?;
        }
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;

        let (npc_key, greeting) = match case {
            Case::KingReward => ("oteryn:npc.king_tibianus", "hail king"),
            Case::Introduction => ("oteryn:npc.rapanaio", "hi"),
            Case::Exchange(exchange) => (exchange.npc, "hi"),
        };
        let [npc] = install(&owner, &admitted, [npc_key]).await?;
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Auditor", owner.owner_now());
        owner.states.chat.set_npc_service(admitted.game_session_id);
        talk(&mut owner, &admitted, npc, 3000, greeting).await?;
        // TibiaWiki /Spoiler curid 36717 oldid 855959, Method and Missions 1 and 7.
        match case {
            Case::Introduction => {
                talk(&mut owner, &admitted, npc, 3001, "yes").await?;
                assert_eq!(
                    progress(&owner, &admitted, &authority, &questline).await?,
                    0
                );
                let reply = talk(&mut owner, &admitted, npc, 3002, "isle of evil").await?;
                let offer = quest::load()?.reply("rapanaio:172").ok_or("offer")?;
                assert_eq!(reply.len(), offer.lines.len());
                assert_eq!(reply.first(), offer.lines.first());
                talk(&mut owner, &admitted, npc, 3003, "yes").await?;
                assert_eq!(
                    progress(&owner, &admitted, &authority, &questline).await?,
                    1
                );
            }
            Case::Exchange(exchange) => {
                for (n, word) in (3001..).zip(exchange.words) {
                    talk(&mut owner, &admitted, npc, n, word).await?;
                }
                for (suffix, _, after) in exchange.changes {
                    let track = format!("{}{suffix}", quest::PREFIX);
                    assert_eq!(
                        progress(&owner, &admitted, &authority, &track).await?,
                        *after,
                        "{track}"
                    );
                }
                if let Some(item) = exchange.take {
                    assert_eq!(
                        item_count(&h, &format!("oteryn:item.tibia.i{item}")).await?,
                        0
                    );
                }
                if let Some(item) = exchange.give {
                    talk(
                        &mut owner,
                        &admitted,
                        npc,
                        3010,
                        exchange.words[exchange.words.len() - 1],
                    )
                    .await?;
                    assert_eq!(
                        item_count(&h, &format!("oteryn:item.tibia.i{item}")).await?,
                        1
                    );
                }
            }
            Case::KingReward => {
                let before: i64 = sqlx::query_scalar(
                    "SELECT total_experience FROM game_character_progression_state",
                )
                .fetch_one(&h.pool)
                .await?;
                let card = "fan club membership card";
                talk(&mut owner, &admitted, npc, 3001, card).await?;
                assert_eq!(
                    progress(&owner, &admitted, &authority, &mission07).await?,
                    4
                );
                assert_eq!(
                    progress(&owner, &admitted, &authority, &questline).await?,
                    16
                );
                talk(&mut owner, &admitted, npc, 3002, card).await?;
                let after: i64 = sqlx::query_scalar(
                    "SELECT total_experience FROM game_character_progression_state",
                )
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(after - before, 6666);
                assert_eq!(item_count(&h, "oteryn:item.tibia.i9389").await?, 1);
                assert_eq!(item_count(&h, "oteryn:item.tibia.i9391").await?, 1);
                let earned: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/king_tibianus_fan'").fetch_one(&h.pool).await?;
                assert_eq!(earned, 1);
            }
        }
        Ok(())
    })
}

async fn item_count(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(SUM(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(item).fetch_one(&h.pool).await?)
}
