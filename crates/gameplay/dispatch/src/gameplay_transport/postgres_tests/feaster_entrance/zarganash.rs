//! Actual death publication and durable per-Character credit for a staged group.
use super::*;
use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
use crate::gameplay_transport::kill_reward::{KillSink, record_creature_kill};
use oteryn_gameplay_entities::carrier::encounter_access::EncounterDefenders;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use std::sync::Arc;

const BOSS: &str = "oteryn:creature.the_fear_feaster";
const SOURCE: &str = "oteryn:quest-callback/feaster-zarganash-fixture";
const TRANSITION: &str = "oteryn:quest-transition/native-kills/feaster-zarganash-fixture";
const QUEST: &str = "oteryn:quest.feaster_of_souls_quest";

#[test]
fn feaster_zarganash_death_credits_only_the_damaging_entrant() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::super::kill_reward_e2e::scenario_thread::run_async(exercise)
}

fn catalogue() -> TestResult<QuestStateCatalogue> {
    let source = serde_json::json!({
        "key":SOURCE,"quest":QUEST,"trigger":"use","selector":{"position":{"x":1,"y":0,"floor":-7},"ordinal":0},
        "lever":null,"reversible":false,"passages":[],"storage":[],"teleport":null,
        "effects":{"encounter":{
            "minimum_level":250,
            "players":[
                {"from":{"x":1,"y":0,"floor":-7},"to":{"x":3,"y":0,"floor":-7}},
                {"from":{"x":2,"y":0,"floor":-7},"to":{"x":4,"y":0,"floor":-7}}
            ],
            "monsters":[{"position":{"x":5,"y":0,"floor":-7},"creature":BOSS}],
            "room_min":{"x":3,"y":0,"floor":-7},"room_max":{"x":5,"y":0,"floor":-7}
        }},
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:actions_bosses_killed.lua:2-18"]
    });
    let tracks = [
        "oteryn:quest-progress/feaster-fixture/gate",
        "oteryn:quest-progress/feaster-fixture/count",
    ];
    let input = serde_json::json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(SOURCE)),
        "sources":["Crystal Feaster death qualification"],
        "tracks":[
            {"key":tracks[0],"initial":1,"minimum":0,"maximum":1},
            {"key":tracks[1],"initial":0,"minimum":0,"maximum":1}
        ],
        "stages":[
            {"key":"entry","completes":false,"trigger":{"kind":"map_use","callback":source}},
            {"key":"kill","completes":false,"trigger":{"kind":"kill","counter":{
                "key":TRANSITION,"creature":BOSS,"track":tracks[1],"gate":tracks[0],
                "gate_value":1,"maximum":1,"encounter_source":SOURCE,"require_damage":true
            }}}
        ]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&input)?)?;
    Ok(bindings.attach(&QuestStateCatalogue::empty("content-1").ok_or("catalogue")?)?)
}

async fn exercise(admin: String) -> TestResult {
    let catalogue = Arc::new(catalogue()?);
    let hash = catalogue.definition_hash(QUEST).ok_or("definition")?;
    let h = Harness::create_with_admission_seed(admin, "feaster_zarganash", true, false).await?;
    prepare_current_session(&h).await?;
    h.seed_appearance_observer().await?;
    super::super::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let initial = Dependencies::new()?;
    let (controller, activated) =
        super::super::death_ml_loss_composed::production_generation_with_pin(
            initial.admitted.world_id,
            "content-1",
        )?;
    let active = controller.active().ok_or("active generation")?;
    let room = activated.qualified_room().clone();
    let (pin, cells, _, _) = activated.into_channel_parts();
    let deps = Dependencies::with_content_and_capacity(pin, room, initial.spells.clone(), 3)?;
    let mut owner = deps.owner(&h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(catalogue);
    let native = active.native_gameplay().ok_or("native")?;
    let session = deps.admitted.game_session_id;
    let actor = deps.actor;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "session fence")?
        .ok_or("current session fence")?;
    let lease = CharacterLease::new(
        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
        fence.character_lease_generation,
    )
    .map_err(debug)?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, session, lease)
            .map_err(debug)?;
        runtime
            .install_companion_policies(native.policies().clone())
            .map_err(debug)?;
    }
    deps.initialize(&owner).await;
    let observer = super::super::appearance_change_postgres::observer_visibility_tests::initialize(
        &deps, &owner,
    )
    .await?;
    let observer_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    owner.refresh_quest_session(session, true).await;
    owner.refresh_quest_session(observer_session, true).await;
    let queue = owner.states.attack.lock().await.kills().clone();
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    {
        let mut runtime = deps.runtime.lock().await;
        let policy = runtime.companion_policy(BOSS).map_err(debug)?;
        let target = runtime
            .admit_encounter_actor(
                BOSS,
                policy.maximum_health,
                MovementLocalPosition {
                    x: 5,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, BOSS)
            .map_err(debug)?;
        {
            let mut capture = queue.1.lock().map_err(|_| "capture lock")?;
            let admitted = capture
                .prepare_encounter(
                    &runtime,
                    SOURCE,
                    hash,
                    &[(actor, session), (observer, observer_session)],
                )
                .map_err(|()| "admitted encounter")?;
            let roster = EncounterDefenders::new(&[actor, observer], &[target]).map_err(debug)?;
            capture.publish_encounter(admitted, roster);
        }
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                target,
                actor,
                CommandRef::new(session, CommandId::new(90).map_err(debug)?),
                0,
                OwnerDamageCommand {
                    target: BOSS.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"feaster-zarganash",
                    damage: policy.maximum_health,
                },
            )
            .map_err(debug)?;
        record_creature_kill(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: &queue,
                rewards: native.reward_table(),
                charms: None,
            },
            target,
            crate::combat::CapturedRewardPrincipal {
                character: lease.character_id(),
                lease_generation: lease.generation(),
                session,
                actor,
            },
            now,
            Some(observed),
        );
    }
    owner.drain_kill_rewards(session).await;
    let recipients: Vec<String> = sqlx::query_scalar(
        "SELECT replace(character_id::text, '-', '') FROM game_character_quest_receipts WHERE transition_key=$1"
    ).bind(TRANSITION).fetch_all(&h.pool).await?;
    let expected: String = fence
        .character_id
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(recipients, [expected]);
    drop(authority);
    drop(seal);
    h.cleanup().await
}
