//! QC-DT: actual NPC dispatch and the existing fenced QuestState writer.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_quests::quest::{dark_trails as quest, world_callbacks::Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

#[test]
fn dark_trails_npc_flow_commits_once_and_fences_conversation_topics() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("dark-trails-npc", || scenario(None, false))
}

const VOTES: &str = "oteryn:quest-progress/quest/u10_50/oramond_quest/voting_points";

#[test]
fn dark_trails_marvin_spends_one_oramond_vote_and_credits_archives() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty(), "routed PostgreSQL needs an admin URL");
    super::kill_reward_e2e::scenario_thread::run("dark-trails-marvin", || {
        scenario(
            Some(VoteScenario {
                npc: "oteryn:npc.marvin",
                prerequisite: "mission07",
                votes: 2,
                target: "mission08",
                words: &["funding", "archives", "1", "yes", "yes"],
                expected_votes: 1,
                expected_credit: 1,
            }),
            false,
        )
    })
}

#[test]
fn dark_trails_sholley_requires_fifty_available_votes_without_spending_them() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty(), "routed PostgreSQL needs an admin URL");
    super::kill_reward_e2e::scenario_thread::run("dark-trails-sholley", || {
        scenario(
            Some(VoteScenario {
                npc: "oteryn:npc.sholley",
                prerequisite: "mission12",
                votes: 49,
                target: "mission13",
                words: &["friend"],
                expected_votes: 49,
                expected_credit: 0,
            }),
            false,
        )?;
        scenario(
            Some(VoteScenario {
                npc: "oteryn:npc.sholley",
                prerequisite: "mission12",
                votes: 50,
                target: "mission13",
                words: &["friend"],
                expected_votes: 50,
                expected_credit: 1,
            }),
            false,
        )
    })
}

#[test]
fn dark_trails_barnabas_refuses_unavailable_self_harvest_credit() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty(), "routed PostgreSQL needs an admin URL");
    super::kill_reward_e2e::scenario_thread::run("dark-trails-barnabas", || scenario(None, true))
}

struct VoteScenario {
    npc: &'static str,
    prerequisite: &'static str,
    votes: i64,
    target: &'static str,
    words: &'static [&'static str],
    expected_votes: i64,
    expected_credit: i64,
}

fn scenario(vote_case: Option<VoteScenario>, check_self_harvest: bool) -> TestResult {
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
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false)
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
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        if let Some(VoteScenario {
            npc, prerequisite, votes, target, words, expected_votes, expected_credit,
        }) = vote_case {
            let mut seed = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *seed)
                .await?;
            for (track, quest, value) in [
                (
                    format!("{}{prerequisite}", quest::TRACK_PREFIX),
                    quest::QUEST,
                    1,
                ),
                (VOTES.into(), "oteryn:quest.oramond_quest", votes),
            ] {
                sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,GREATEST(2, character_revision) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).bind(track).bind(quest).bind(value).execute(&mut *seed).await?;
            }
            seed.commit().await?;
            owner.admit_quest_session(&admitted).await;
            let [npc] = install(&owner, &admitted, [npc]).await?;
            talk(&mut owner, &admitted, npc, 3000, "hi").await?;
            for (ordinal, word) in (3001..).zip(words) {
                talk(&mut owner, &admitted, npc, ordinal, word).await?;
            }
            let credit = progress(
                &owner,
                &admitted,
                &authority,
                &format!("{}{target}", quest::TRACK_PREFIX),
            )
            .await?;
            assert_eq!(credit, expected_credit, "{target} credit");
            assert_eq!(
                progress(&owner, &admitted, &authority, VOTES).await?,
                expected_votes
            );
            if target == "mission08" {
                let receipts: Vec<(Vec<i64>, Vec<i64>)> = sqlx::query_as(
                    "SELECT p.values_before, p.values_after
                     FROM game_quest_reward_participants p
                     JOIN game_character_quest_receipts r USING
                       (character_id, cause_id, cause_ordinal, transition_key, committed_character_revision)
                     WHERE p.transition_key = $1 AND p.quest_key = 'oteryn:quest.oramond_quest'",
                )
                .bind("oteryn:quest-reward/dark-trails/archive-vote")
                .fetch_all(&h.pool)
                .await?;
                assert_eq!(receipts, [(vec![votes], vec![expected_votes])]);
            }
            return Ok(());
        }
        owner.admit_quest_session(&admitted).await;

        if check_self_harvest {
            let [barnabas] = install(&owner, &admitted, ["oteryn:npc.barnabas_dee"]).await?;
            talk(&mut owner, &admitted, barnabas, 3000, "hi").await?;
            talk(&mut owner, &admitted, barnabas, 3001, "mission").await?;
            assert_eq!(
                talk(&mut owner, &admitted, barnabas, 3002, "seance").await?,
                ["Sorry, not possible."]
            );
            assert_eq!(
                progress(
                    &owner,
                    &admitted,
                    &authority,
                    &format!("{}mission16", quest::TRACK_PREFIX)
                )
                .await?,
                0,
            );
            return Ok(());
        }

        let [ezebeth, beggar, rapanaio] = install(
            &owner,
            &admitted,
            [
                "oteryn:npc.ezebeth",
                "oteryn:npc.a_beggar",
                "oteryn:npc.rapanaio",
            ],
        )
        .await?;
        owner.states.chat.admit(
            admitted.game_session_id,
            "[|PLAYERNAME|] ${name} %s",
            owner.owner_now(),
        );
        owner.states.chat.set_npc_service(admitted.game_session_id);
        talk(&mut owner, &admitted, ezebeth, 3000, "hi").await?;
        talk(&mut owner, &admitted, ezebeth, 3001, "strange").await?;
        talk(&mut owner, &admitted, ezebeth, 3002, "mission").await?;
        talk(&mut owner, &admitted, ezebeth, 3003, "assistance").await?;
        // Moving to another NPC invalidates Ezebeth's held topic.
        talk(&mut owner, &admitted, beggar, 3004, "hi").await?;
        talk(&mut owner, &admitted, ezebeth, 3005, "hi").await?;
        talk(&mut owner, &admitted, ezebeth, 3006, "strange").await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-transition/dark-trails/%'")
            .fetch_one(&h.pool).await?;
        assert_eq!(count, 0, "no mutation without the current source topics");
        talk(&mut owner, &admitted, ezebeth, 3007, "mission").await?;
        talk(&mut owner, &admitted, ezebeth, 3008, "assistance").await?;
        let mut stale = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?
            .item_fence();
        stale.character_lease_generation += 1;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let refused = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: 3009,
                    item_fence: Some(stale),
                },
                NpcTalkIntent {
                    npc_actor: ezebeth,
                    text: "strange".into(),
                },
            )
            .await;
        assert_eq!(refused.disposition, NpcIntentDisposition::Rejected);
        owner
            .states
            .npc_conversations
            .fixture_interrupt_quest_dialogue_commit(admitted.game_session_id, 3010)?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 3010,
            item_fence: Some(fence.item_fence()),
        };
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let actor = admitted.runtime_actor.ok_or("actor")?;
        let lost = owner
            .npc_talk(
                actor,
                command,
                NpcTalkIntent {
                    npc_actor: ezebeth,
                    text: "strange".into(),
                },
            )
            .await;
        assert!(lost.unknown && lost.conversation.is_none());
        assert!(
            owner
                .resume_npc_bank(
                    actor,
                    UseCommand {
                        command_id: 3099,
                        ..command
                    }
                )
                .await
                .is_none()
        );
        let replay = owner
            .resume_npc_bank(actor, command)
            .await
            .ok_or("retained source reply")?;
        assert_eq!(replay.disposition, NpcIntentDisposition::Ok);
        assert!(!replay.unknown);
        let book = quest::load()?;
        assert_eq!(
            replay.conversation.ok_or("replay speech")?.lines,
            book.replies["ezebeth-start"].lines
        );
        assert!(owner.resume_npc_bank(actor, command).await.is_none());
        let mission01 = format!("{}mission01", quest::TRACK_PREFIX);
        assert_eq!(
            progress(&owner, &admitted, &authority, &mission01).await?,
            1
        );
        assert_eq!(
            talk(&mut owner, &admitted, ezebeth, 3011, "bye").await?,
            ["Good bye, [|PLAYERNAME|] ${name} %s."]
        );
        talk(&mut owner, &admitted, beggar, 3012, "hi").await?;
        talk(&mut owner, &admitted, beggar, 3013, "yes").await?;
        talk(&mut owner, &admitted, beggar, 3014, "want").await?;
        let report = talk(&mut owner, &admitted, beggar, 3015, "yes").await?;
        assert_eq!(report, book.replies["beggar-report"].lines);
        assert_eq!(
            progress(&owner, &admitted, &authority, &mission01).await?,
            2
        );
        assert_eq!(
            progress(
                &owner,
                &admitted,
                &authority,
                &format!("{}mission02", quest::TRACK_PREFIX)
            )
            .await?,
            1
        );
        talk(&mut owner, &admitted, beggar, 3016, "yes").await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-transition/dark-trails/%'")
            .fetch_one(&h.pool).await?;
        assert_eq!(count, 2, "exactly the two source mutations");
        // The second adapter uses the same caller, with a separate context.
        talk(&mut owner, &admitted, rapanaio, 3020, "hi").await?;
        talk(&mut owner, &admitted, rapanaio, 3021, "yes").await?;
        talk(&mut owner, &admitted, rapanaio, 3022, "mission").await?;
        for (id, word) in [
            (3023, "fight"),
            (3024, "evil"),
            (3025, "hunt"),
            (3026, "ridicule"),
            (3027, "truth"),
            (3028, "isle of evil"),
            (3029, "yes"),
        ] {
            talk(&mut owner, &admitted, rapanaio, id, word).await?;
        }
        assert_eq!(
            progress(
                &owner,
                &admitted,
                &authority,
                &format!(
                    "{}questline",
                    oteryn_gameplay_quests::quest::isle_of_evil::PREFIX
                )
            )
            .await?,
            1
        );
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key LIKE 'oteryn:quest-transition/isle-of-evil/%'")
            .fetch_one(&h.pool).await?;
        assert_eq!(count, 1);
        Ok(())
    })
}
