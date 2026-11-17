use super::TestResult;
use oteryn_gameplay_quests::quest::{
    QuestRefusal, loader::load_embedded_quest_state, reward_callbacks,
};
use std::collections::BTreeMap;

const QUEST: &str = "oteryn:quest.sea_of_light_quest";
const PREFIX: &str = "oteryn:quest-reward/qcx-sea-of-light/";

#[test]
fn qcx_sea_of_light_registers_paid_plans_dialogue_without_closing_the_quest() -> TestResult {
    let state =
        load_embedded_quest_state("qcx-sea-of-light-test").map_err(super::fixture::debug)?;
    let catalogue = state.catalogue();
    let callbacks = reward_callbacks::offered_for(catalogue)?;
    let lookup = |stage: &str| {
        callbacks
            .iter()
            .find(|callback| callback.quest == QUEST && callback.key == format!("{PREFIX}{stage}"))
            .ok_or_else(|| format!("missing Sea of Light stage {stage}"))
    };

    let accept = lookup("accept")?;
    assert_eq!(accept.npc, "oteryn:npc.spectulus");
    assert_eq!(accept.keywords, ["yes"]);
    assert!(accept.requires_premium);

    for stage in ["first-coin", "second-coin"] {
        let payment = lookup(stage)?;
        assert_eq!(payment.npc, "oteryn:npc.shoddy_beggar");
        let fee = payment.fee.as_ref().ok_or("missing individual donation")?;
        assert_eq!(fee.gold, 1);
        assert!(!fee.include_bank);
        assert!(payment.give.is_none());
    }

    let plans = lookup("receive-plans")?;
    let reward = plans.give.as_ref().ok_or("missing Plans reward")?;
    assert_eq!(reward.item, "oteryn:item.tibia.i9696");
    assert_eq!(reward.count, 1);
    let handoff = lookup("hand-in-plans")?;
    let [input] = handoff.take.as_slice() else {
        return Err("expected exactly one Plans input".into());
    };
    assert_eq!(input.item, reward.item);
    assert_eq!(input.count, 1);
    assert_eq!(handoff.experience, Some(400));

    let handoff_transition = catalogue.transition(&handoff.key).ok_or("handoff")?;
    let phase = handoff.storage[0].track.clone();
    let handoff_state = BTreeMap::from([(phase, handoff.storage[0].minimum)]);
    let stored = catalogue
        .evaluate(handoff_transition, &handoff_state, 10_000)
        .map_err(|error| format!("handoff evaluation: {error:?}"))?
        .into_iter()
        .map(|change| (change.track, change.after))
        .collect();
    let studied = lookup("plans-studied")?;
    let transition = catalogue
        .transition(&studied.key)
        .ok_or("study transition")?;
    assert!(matches!(
        catalogue.evaluate(transition, &stored, 11_799),
        Err(QuestRefusal::StageMismatch)
    ));
    assert!(catalogue.evaluate(transition, &stored, 11_800).is_ok());

    for callback in callbacks.iter().filter(|callback| callback.quest == QUEST) {
        assert!(!callback.completes);
    }
    Ok(())
}
