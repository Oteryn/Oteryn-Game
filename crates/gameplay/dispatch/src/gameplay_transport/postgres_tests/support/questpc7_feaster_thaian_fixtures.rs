use super::fixture::{Harness,debug};

use super::{AdmittedSession,ComposedFreshAdmission,TestResult};

use oteryn_gameplay_quests::quest::world_callbacks::Position;

const KEY:&str="oteryn:creature.thaian";

pub(super) async fn exercise(owner:&mut ComposedFreshAdmission<'_, '_, '_>,a:&mut AdmittedSession,h:&Harness)->TestResult {
    let version:String=sqlx::query_scalar("SHOW server_version").fetch_one(&h.pool).await?;
    eprintln!("QUESTPC7_THAIAN POSTGRES_STARTED version={version}");
    super::questpc7_feaster_fixtures::begin(owner,a,h).await?;
    let native=owner.active_generation.and_then(|g|g.native_gameplay()).ok_or("native")?;
    let record=native.creature_profiles().records.iter().find(|r|r.profile.target.key==KEY).ok_or("missing first-party Thaian native profile")?;
    let profile=serde_json::to_value(&record.profile)?;
    assert_eq!(profile["data"]["profile"]["health"],50000);
    assert_eq!(profile["data"]["profile"]["experience"],0,"chosen zero XP, no invented source reward");
    assert_eq!(profile["data"]["profile"]["details"]["flags"]["illusionable"],false);
    let immunities=profile["data"]["profile"]["details"]["condition_immunities"].as_array().ok_or("condition immunities")?;
    assert!(immunities.iter().any(|v|v=="paralyze" || v=="paralysis"));
    let behavior=serde_json::to_value(&record.behavior)?;
    assert_eq!(behavior["data"]["profile"]["targeting"]["sense_invisible"],true);
    super::questpc7_native_deaths_fixtures::prepare_native_hunter(owner,a).await?;
    let mut runtime=owner.runtime.lock().await;
    let policy=runtime.companion_policy(KEY).map_err(debug)?;
    assert_eq!(policy.maximum_health,50000);
    assert!(!policy.flags.illusionable);
    assert_eq!(policy.condition_immunities.len(),2);
    assert_eq!(policy.outfit_look_type,48,"documented existing ghost visual approximation");
    let actor=runtime.admit_encounter_actor(KEY,50000,super::quest_world_callbacks_fixtures::local(Position{x:32627,y:32077,floor:-7})).map_err(debug)?;
    runtime.install_creature_policy(actor,KEY).map_err(debug)?;
    assert_eq!(runtime.companion_snapshot(actor).map_err(debug)?.health,50000);
    drop(runtime);
    assert_eq!(super::questpc7_ferumbras_fixtures::value(owner,a,"oteryn:quest-progress/questpc7/feaster/programme").await?,0);
    assert_eq!(super::questpc7_outfit_items_fixtures::quantity(h,32582).await?,1);
    Ok(())
}
