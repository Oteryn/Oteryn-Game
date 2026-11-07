//! Layer baseline only: isolated source facts qualify plan construction; they
//! do not claim a real chest reward, earned quest predecessor, or final grant.
use super::*;
#[test]
fn srv7_avar_tar_source_final_plan_reaches_existing_quest_owner() -> Result<(), String> {
    let document =
        source::embedded().map_err(|error| format!("actual source document: {error:?}"))?;
    let npc = document
        .npc("oteryn:npc.avar_tar")
        .ok_or("actual AvarTar programme")?;
    let transition = npc
        .transitions
        .iter()
        .find(|row| row.source_key == "oteryn:npc-cosmetic/avar_tar/demon-base")
        .ok_or("actual source final branch")?;
    let binding = document
        .binding("Storage.Quest.U7_24.TheAnnihilator.Reward")
        .ok_or("actual source reward binding")?;
    let state = Snapshot {
        name: "Policy qualification".into(),
        achievement_progress: Default::default(),
        tracks: Default::default(),
        source_values: std::collections::BTreeMap::from([(binding.key.clone(), 1)]),
        claim_projections: Default::default(),
        inventory: Default::default(),
        now: 1_800_000_000,
        level: 100,
        sex: source::Sex::Male,
        premium: true,
        looks: Default::default(),
        mounts: Default::default(),
        selected: None,
        hazard: None,
        primal: None,
        actor_drunk: None,
    };
    let selected = document
        .choose_for_trigger(
            &npc.npc,
            "base",
            2,
            &state,
            source_contains,
            source::Trigger::Say,
        )
        .map_err(|error| format!("actual source chooser: {error:?}"))?
        .ok_or("qualified source choice")?;
    assert_eq!(selected.source_key, transition.source_key);
    let world = oteryn_protocol_oteryn::WorldId::decode(&[
        1, 144, 0, 0, 0, 8, 112, 0, 128, 0, 0, 0, 0, 0, 0, 8,
    ])
    .map_err(|error| format!("actual fixture World identity: {error:?}"))?;
    let native =
        oteryn_gameplay_content::content::native_gameplay::retained_callback_native_fixture(world);
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("actual loaded Quest catalogue: {error:?}"))?;
    let effects = NpcSourceEffects {
        quest_catalogue: Some(std::sync::Arc::new(loaded.catalogue().clone())),
        ..Default::default()
    };
    let result = plan::prepare(npc, transition, &state, &native, &effects);
    assert!(
        result.is_ok(),
        "exact source final branch requires a composed Quest owner: {:?}",
        result.err()
    );
    Ok(())
}
