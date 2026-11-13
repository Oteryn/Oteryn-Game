use super::*;

fn callback() -> Result<Callback, String> {
    load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/l6/waterskin-of-mead")
        .map(|mut callback| {
            callback.effects.quest_completion =
                Some("oteryn:quest.authored.waterskin_of_mead_quest".into());
            callback
        })
        .ok_or_else(|| "Mead callback".into())
}
fn document(callback: &Callback) -> String {
    serde_json::json!({"schema":1,"callbacks":[callback]}).to_string()
}

#[test]
fn child_completion_keeps_the_parent_incomplete_and_requires_native_reward_admission()
-> Result<(), String> {
    let original = callback()?;
    assert!(!primary_transition(&original)?.completes);
    let child = completion_transition(&original).ok_or("child transition")?;
    assert!(child.completes);
    assert!(child.effects.is_empty());
    assert_eq!(child.quest, "oteryn:quest.authored.waterskin_of_mead_quest");
    assert_ne!(child.quest, original.quest);
    for invalid in 0..9 {
        let mut c = original.clone();
        match invalid {
            0 => c.effects.quest_completion = Some(c.quest.clone().into()),
            1 => c.effects.quest_completion = Some("not a quest".into()),
            2 => c.trigger = Trigger::StepIn,
            3 => c.effects.item_reward = None,
            4 => c.reversible = true,
            5 => c.storage.iter_mut().for_each(|s| s.preserve = true),
            6 => c.actor_kind = ActorKind::Monster,
            7 => c.effects.quest_completion = Some("oteryn:item.not_a_quest".into()),
            _ => c.effects.item_reward_cycle = Some(c.storage[0].track.clone()),
        }
        assert!(
            parse(&document(&c)).is_err(),
            "invalid completion shape {invalid}"
        );
    }
    Ok(())
}

#[test]
fn child_completion_target_is_in_the_native_binding_and_its_own_definition() -> Result<(), String> {
    let original = callback()?;
    let mut changed = original.clone();
    changed.effects.quest_completion = Some("oteryn:quest.authored.other_child".into());
    assert_ne!(
        completion_transition(&original),
        completion_transition(&changed)
    );
    let base = QuestStateCatalogue::empty("completion-test").ok_or("empty catalogue")?;
    let lowered = extend_callbacks(base, std::slice::from_ref(&original))?;
    let child = completion_transition(&original).ok_or("child transition")?;
    assert_eq!(lowered.transition(&child.key), Some(&child));
    assert!(lowered.definition_hash(&child.quest).is_some());
    assert_ne!(
        lowered.definition_hash(&child.quest),
        lowered.definition_hash(&original.quest)
    );
    Ok(())
}
