use super::*;
use crate::quest::{QuestRefusal, loader};

fn stored(values: &[(&str, i64)]) -> BTreeMap<String, i64> {
    values
        .iter()
        .map(|(key, value)| (format!("{PREFIX}{key}"), *value))
        .collect()
}
fn catalogue() -> Result<super::super::QuestStateCatalogue, String> {
    loader::load_embedded_quest_state("isle-test")
        .map(|loaded| loaded.catalogue().clone())
        .map_err(|error| format!("{error:?}"))
}

#[test]
fn isle_public_loader_installs_nine_compound_npc_transitions() -> Result<(), String> {
    let catalogue = catalogue()?;
    let book = load()?;
    for step in &book.data.transitions {
        let transition = catalogue
            .transition(&transition_key(&step.key))
            .ok_or("missing native Isle transition")?;
        let before = step
            .effects
            .iter()
            .map(|effect| (effect.track.clone(), effect.from.unwrap_or(0)))
            .collect();
        let changes = catalogue
            .evaluate(transition, &before, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(changes.len(), step.effects.len());
        for effect in &step.effects {
            assert_eq!(
                changes
                    .iter()
                    .find(|change| change.track == effect.track)
                    .ok_or("change")?
                    .after,
                effect.set
            );
        }
        assert!(transition.experience.is_none());
        assert!(!transition.completes);
    }
    Ok(())
}

#[test]
fn isle_final_mission_sets_the_exact_scoped_travel_flag_atomically() -> Result<(), String> {
    let catalogue = catalogue()?;
    let flag = format!("{KV_PREFIX}rapanaio-travel");
    let track = catalogue.track(&flag).ok_or("missing scoped source KV")?;
    assert_eq!((track.initial, track.min, track.max), (0, 0, 1));
    let transition = catalogue
        .transition(&transition_key("begin-final-mission"))
        .ok_or("transition")?;
    assert_eq!(
        catalogue.evaluate(transition, &stored(&[("questline", 12)]), 0),
        Err(QuestRefusal::StageMismatch)
    );
    let changes = catalogue
        .evaluate(transition, &stored(&[("questline", 13)]), 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(changes.len(), 3);
    assert_eq!(
        changes
            .iter()
            .find(|change| change.track == flag)
            .ok_or("KV change")?
            .after,
        1
    );
    assert!(
        catalogue
            .track(&format!("{PREFIX}mission04_must_pay"))
            .is_none()
    );
    assert!(
        catalogue
            .track(&format!("{PREFIX}mission04_worm_count"))
            .is_none()
    );
    Ok(())
}

#[test]
fn isle_mortimer_revalidates_its_own_mission_and_preserves_existing_bounds() -> Result<(), String> {
    let catalogue = catalogue()?;
    let transition = catalogue
        .transition(&transition_key("mortimer-map"))
        .ok_or("map transition")?;
    let after = catalogue
        .evaluate(transition, &stored(&[("mission05", 1)]), 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(after.len(), 2);
    assert_eq!(
        catalogue.evaluate(transition, &stored(&[("mission05", 2)]), 0),
        Err(QuestRefusal::StageMismatch)
    );
    let track = catalogue
        .track(&format!("{PREFIX}questline"))
        .ok_or("existing questline")?;
    assert_eq!((track.initial, track.min, track.max), (0, 0, 16));
    Ok(())
}

#[test]
fn isle_rapanaio_requires_the_complete_topic_sequence_and_keeps_source_paragraphs()
-> Result<(), String> {
    let book = load()?;
    let state = stored(&[]);
    assert!(
        book.respond("oteryn:npc.rapanaio", "yes", 0, &state)
            .is_none()
    );
    let mut topic = 0;
    for (word, next) in [
        ("mission", 1),
        ("fight", 2),
        ("evil", 3),
        ("hunt", 4),
        ("ridicule", 5),
        ("truth", 6),
        ("isle of evil", 7),
    ] {
        let plan = book
            .respond("oteryn:npc.rapanaio", word, topic, &state)
            .ok_or("source step")?;
        assert!(!plan.held);
        assert!(plan.transition.is_none());
        assert_eq!(plan.next_topic, Some(next));
        if word == "truth" {
            assert_eq!(plan.lines.len(), 3);
        }
        topic = next;
    }
    let accept = book
        .respond("oteryn:npc.rapanaio", "yes", topic, &state)
        .ok_or("source accept")?;
    assert_eq!(accept.transition, Some(transition_key("accept-fight")));
    assert_eq!(accept.lines.len(), 2);
    assert_eq!(accept.next_topic, Some(0));
    Ok(())
}

#[test]
fn isle_source_independent_mission_block_runs_before_the_following_topic_block()
-> Result<(), String> {
    let book = load()?;
    let plan = book
        .respond("oteryn:npc.rapanaio", "mission fight", 0, &stored(&[]))
        .ok_or("two source blocks")?;
    assert_eq!(plan.next_topic, Some(2));
    assert_eq!(plan.lines.len(), 4);
    let plan = book
        .respond(
            "oteryn:npc.rapanaio",
            "mission yes",
            7,
            &stored(&[("questline", 3)]),
        )
        .ok_or("mission resets old topic")?;
    assert_eq!(plan.transition, Some(transition_key("mission-3")));
    assert_eq!(plan.next_topic, Some(0));
    Ok(())
}

#[test]
fn isle_item_and_travel_holds_never_return_success_text_or_storage_changes() -> Result<(), String> {
    let book = load()?;
    for topic in 8..=13 {
        let plan = book
            .respond("oteryn:npc.rapanaio", "yes", topic, &stored(&[]))
            .ok_or("held item branch")?;
        assert!(plan.held);
        assert!(plan.transition.is_none() && plan.lines.is_empty() && plan.next_topic.is_none());
    }
    for (npc, topic) in [
        ("rapanaio_boat", 8),
        ("rapanaio_isle_of_evil", 2),
        ("scutty", 2),
        ("scutty", 3),
    ] {
        let plan = book
            .respond(&format!("oteryn:npc.{npc}"), "yes", topic, &stored(&[]))
            .ok_or("held compound branch")?;
        assert!(plan.held);
        assert!(plan.transition.is_none() && plan.lines.is_empty());
    }
    assert!(
        book.respond("oteryn:npc.rapanaio_boat", "yes", 9, &stored(&[]))
            .is_none(),
        "source topic9 remains unreachable"
    );
    Ok(())
}

#[test]
fn isle_scutty_reads_the_existing_boolean_kv_instead_of_nil_legacy_constants() -> Result<(), String>
{
    let book = load()?;
    let mut state = stored(&[("mission04", 2)]);
    let before = book
        .respond("oteryn:npc.scutty", "mission", 0, &state)
        .ok_or("source task")?;
    assert_eq!(before.next_topic, Some(0));
    state.insert(format!("{KV_PREFIX}mission04-mustpay"), 1);
    let owed = book
        .respond("oteryn:npc.scutty", "mission", 0, &state)
        .ok_or("source penalty")?;
    assert_eq!(owed.next_topic, Some(3));
    assert!(
        owed.lines
            .first()
            .is_some_and(|line| line.contains("500 gold"))
    );
    assert!(owed.transition.is_none());
    let payment = book
        .respond("oteryn:npc.scutty", "yes", 3, &state)
        .ok_or("held payment")?;
    assert!(payment.held && payment.lines.is_empty());
    assert_eq!(state[&format!("{KV_PREFIX}mission04-mustpay")], 1);
    Ok(())
}

#[test]
fn isle_mortimer_preserves_earlier_foreign_keyword_precedence() -> Result<(), String> {
    let book = load()?;
    let state = stored(&[("mission05", 1)]);
    assert_eq!(
        book.respond("oteryn:npc.mortimer", "nautical map", 0, &state)
            .ok_or("Isle map")?
            .transition,
        Some(transition_key("mortimer-map"))
    );
    for mixed in [
        "mission nautical map",
        "join nautical map",
        "job nautical map",
        "nautical map yes",
    ] {
        let plan = book
            .respond("oteryn:npc.mortimer", mixed, 0, &state)
            .ok_or("foreign precedence hold")?;
        assert!(plan.held && plan.transition.is_none());
    }
    assert!(
        book.respond("oteryn:npc.mortimer", "nautical map", 33, &state)
            .ok_or("foreign held topic")?
            .held
    );
    assert!(
        book.respond(
            "oteryn:npc.king_tibianus",
            "fan club membership card",
            0,
            &stored(&[("questline", 15)])
        )
        .is_none()
    );
    Ok(())
}

#[test]
fn isle_literal_msgcontains_preserves_the_donor_prefix_rule() -> Result<(), String> {
    let book = load()?;
    let state = stored(&[]);
    assert!(
        book.respond("oteryn:npc.rapanaio", "amission", 0, &state)
            .is_none()
    );
    assert!(
        book.respond("oteryn:npc.rapanaio", "mission amission", 0, &state)
            .is_none()
    );
    assert_eq!(
        book.respond("oteryn:npc.rapanaio", "_MISSION", 0, &state)
            .ok_or("underscore is not Lua word")?
            .next_topic,
        Some(1)
    );
    assert_eq!(
        book.respond("oteryn:npc.rapanaio", "missionary", 0, &state)
            .ok_or("no trailing boundary")?
            .next_topic,
        Some(1)
    );
    Ok(())
}
