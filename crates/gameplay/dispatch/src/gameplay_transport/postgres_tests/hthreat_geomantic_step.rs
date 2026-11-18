//! Crystal 201189f8 movements_geomantic_charges.lua:13–38 through paced movement.
use super::TestResult;
use super::full_world_support::world_scenario;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings};
use serde_json::json;

const QUEST: &str = "oteryn:quest.spike_task";
const NEST: &str = "oteryn:quest-progress/spike-task/middle-nest";
const CHARGE: &str = "oteryn:quest-progress/spike-task/middle-charge";

fn bindings() -> TestResult<Bindings> {
    let mut stages = Vec::new();
    for (key, nest, charge, disguise, advances) in [
        ("charge-only", (8, 8), (1, 1), false, true),
        ("charge-unassigned-nest", (-1, -1), (1, 1), false, true),
        ("combined", (0, 7), (1, 1), true, true),
        ("nest-ready", (0, 7), (2, 2), true, false),
        ("nest-uncharged", (0, 7), (-1, 0), true, false),
    ] {
        let position = json!({"x":9,"y":0,"floor":-7});
        let mut effects = json!({});
        if disguise {
            effects["timed_disguise"] =
                json!({"look_type":307,"duration_ms":120000,"magic_effect":11});
        }
        let success_message = advances
            .then_some("You have charged your body with geomantic energy and can report about it.");
        if advances {
            effects["magic"] = json!({"position":position,"effect":12});
        }
        stages.push(json!({"key":key,"completes":false,"trigger":{"kind":"step_in","callback":{
            "key":format!("oteryn:quest-callback/hthreat-geomantic-{key}"),
            "quest":QUEST,"trigger":"step_in","selector":{"position":position,"ordinal":0},
            "reversible":false,"passages":[],"return_on_failure":true,"offerings":[],
            "requires_world_admission":advances,
            "effects":effects,"success_message":success_message,
            "storage":[
                {"track":NEST,"minimum":nest.0,"maximum":nest.1,"set":nest.0,"preserve":advances,"read_only":!advances},
                {"track":CHARGE,"minimum":charge.0,"maximum":charge.1,"set":if advances {2} else {charge.0},"read_only":!advances}
            ],
            "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_spike_tasks/movements_geomantic_charges.lua:13"]
        }}}));
    }
    // These are test-only placements. The admitted World supplies the AID 56421 placement.
    // A digest of the scoped source reference binds this fixture's recipe identity.
    use sha2::{Digest, Sha256};
    let digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&stages)?));
    Ok(Bindings::parse(&serde_json::to_vec(
        &json!({"schema":1,"quests":[{
            "quest":QUEST, "recipe_sha256":digest,
            "sources":["Crystal 201189f8 geomantic StepIn fixture"],
            "tracks":[
                {"key":CHARGE,"initial":1,"minimum":-1,"maximum":2},
                {"key":NEST,"initial":0,"minimum":-1,"maximum":8}
            ],
            "stages":stages
        }]}),
    )?)?)
}

#[test]
fn hthreat_geomantic_step_commits_charge_and_two_minute_disguise() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run("hthreat-geomantic", scenario)
}

fn scenario() -> TestResult {
    world_scenario!(
        "hthreat_geomantic",
        |h,
         _request,
         _seal,
         authority,
         world_id,
         native,
         world,
         cells,
         room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            // Replace the fixture activation's original frame with this bundle's frame.
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 2, 1, $3, $4, $5)")
                .bind(world_id.as_bytes().as_slice())
                .bind(deps.admitted.channel_id.as_bytes().as_slice())
                .bind(pin.server_artifact_digest().as_slice())
                .bind(pin.client_artifact_digest().as_slice())
                .bind(pin.frame_binding_digest().as_slice())
                .execute(&h.pool).await?;
            let catalogue = QuestStateCatalogue::empty("content-1").ok_or("empty catalogue")?;
            owner.quest_catalogue = Some(Arc::new(bindings()?.attach(&catalogue)?));
            owner.refresh_quest_session(session, false).await;
            assert_eq!(step_west(&mut owner, actor, session).await?.x, 9);

            let durable = owner
                .root
                .read_character_quest_state(owner.character, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(durable.tracks().get(CHARGE), Some(&2));
            let effects = owner
                .bundle_map()
                .ok_or("bundle")?
                .facts
                .movement_effects_since(0);
            assert_eq!(
                effects
                    .iter()
                    .map(|(_, _, effect)| *effect)
                    .collect::<Vec<_>>(),
                [11, 12]
            );

            let disguise = async |time: u64| {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                Ok::<_, Box<dyn std::error::Error>>(
                    states
                        .get(&runtime, actor, session)
                        .ok_or("spell state")?
                        .owned_conditions()
                        .outfit_at(time),
                )
            };
            let now = owner.owner_now().get();
            assert_eq!(disguise(now + 119_000_000).await?, Some(307));
            assert_eq!(disguise(now + 120_000_000).await?, None);
            owner.clock_origin -= Duration::from_secs(2);
            let (outcome, _) = owner.paced_step(actor, session, StepDirection::East).await;
            assert_eq!(outcome.disposition, StepDisposition::Moved);
            owner.clock_origin -= Duration::from_secs(2);
            let _ = owner.paced_step(actor, session, StepDirection::West).await;
            assert_eq!(
                position(&owner, actor).await?.x,
                10,
                "active outfit returns to origin"
            );
            assert_eq!(
                owner
                    .bundle_map()
                    .ok_or("bundle")?
                    .facts
                    .movement_effects_since(0),
                effects
            );
            let receipts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1",
            )
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(receipts, 1);
            h.cleanup().await
        }
    )
}
