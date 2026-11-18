//! Crystal 201189f8, chayenne_realm/actions_lever.lua:3-22 supplies the
//! retained key and timed wall control. The fifth floor is a test fixture.
use super::fixture::debug;
use super::m07_realm_trials::{BEGIN, CONTROL, phase, use_source};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::Bindings,
    world_callbacks::{Callback, Offering, load},
};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::sync::Arc;

pub(super) const NAME: &str = "m07/realm-trial-five-floor";

#[test]
fn realm_control_waits_for_guardian_on_fifth_floor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| quest_world_callbacks::scenario(NAME, CONTROL, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "five-floor clearance worker panicked")??;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    fence: CurrentCharacterGameplayFence,
    control: &Callback,
) -> TestResult {
    let begin = load()?
        .into_iter()
        .find(|callback| {
            callback.selector.position == Some(BEGIN) && callback.quest == control.quest
        })
        .ok_or("Realm start control")?;
    let mut control = control.clone();
    let area = control.effects.clear_area.as_mut().ok_or("Realm area")?;
    area.additional_floors.push(-7);
    let bindings = Bindings::parse(&serde_json::to_vec(&serde_json::json!({
        "schema":1,
        "quests":[{
            "quest":control.quest,
            "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
            "sources":["Crystal 201189f8 chayenne_realm/actions_lever.lua:3-22; synthetic extra floor"],
            "tracks":[{"key":control.storage[0].track,"initial":-1,"minimum":-1,"maximum":2}],
            "stages":[
                {"key":"begin","completes":false,"trigger":{"kind":"map_use","callback":begin}},
                {"key":"control","completes":false,"trigger":{"kind":"map_use","callback":control}}
            ]
        }]
    }))?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("test catalogue")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));

    quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: BEGIN,
            item: "oteryn:item.tibia.i14682".into(),
            count: 1,
        },
        81,
        true,
    )
    .await?;
    assert_eq!(
        use_source(owner, admitted, 1, BEGIN, 3, 0).await?,
        UseDisposition::Committed
    );
    let guardian = {
        let mut runtime = owner.runtime.lock().await;
        owner
            .active_generation
            .and_then(|generation| generation.native_gameplay())
            .ok_or("native policies")?
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let species = "oteryn:creature.dragon";
        let health = runtime
            .companion_policy(species)
            .map_err(debug)?
            .maximum_health;
        let guardian = runtime
            .admit_encounter_actor(
                species,
                health,
                MovementLocalPosition {
                    x: 33088,
                    y: 32583,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(guardian, species)
            .map_err(debug)?;
        guardian
    };
    assert_eq!(
        use_source(owner, admitted, 2, CONTROL, 1, 0).await?,
        UseDisposition::Rejected
    );
    assert_eq!(phase(owner, fence, &control).await?, Some(1));
    assert!(owner.runtime.lock().await.kill_creature_for_test(guardian));
    assert_eq!(
        use_source(owner, admitted, 3, CONTROL, 1, 0).await?,
        UseDisposition::Committed
    );
    assert_eq!(phase(owner, fence, &control).await?, Some(2));
    Ok(())
}
