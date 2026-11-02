//! Child of quest::world_callbacks, using its real loader and evaluator.
//! Run RED before adding the per-quest callback records, GREEN afterwards.
//! These tests do not claim missing production UID/AID bindings are present.
use super::*;

fn named(suffix: &str) -> Result<Callback, String> {
    load()?
        .into_iter()
        .find(|c| c.key == format!("oteryn:quest-callback/{suffix}"))
        .ok_or_else(|| format!("missing source callback {suffix}"))
}

#[test]
fn quests6_shadows_inkwell_needs_eight_successes_and_resets_the_same_counter() -> Result<(), String>
{
    // Crystal's eight-success guard is retained per Q6-D17 and the coordinator's
    // ANSWER: https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589450226
    // Decision: https://github.com/Oteryn/Oteryn-Game/pull/2847#issuecomment-6589532280
    let c = named("shadows-inkwell")?;
    let loaded = super::super::loader::load_embedded_quest_state("quests6-test")
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue.transition(&c.key).ok_or("transition")?;
    let prefix = "oteryn:quest-progress/crystalserver/quest/u8_5/shadows_of_yalahar/";
    let mut before = std::collections::BTreeMap::from([
        (format!("{prefix}mission09"), 1),
        (format!("{prefix}mission09_books_success"), 7),
    ]);
    assert!(catalogue.evaluate(transition, &before, 0).is_err());
    before.insert(format!("{prefix}mission09_books_success"), 8);
    let changes = catalogue
        .evaluate(transition, &before, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert!(
        changes
            .iter()
            .any(|c| c.track == format!("{prefix}mission09") && c.before == 1 && c.after == 2)
    );
    assert!(
        changes
            .iter()
            .any(|c| c.track == format!("{prefix}mission09_books_success")
                && c.before == 8
                && c.after == -1)
    );
    before.insert(format!("{prefix}mission09"), 2);
    assert!(catalogue.evaluate(transition, &before, 0).is_err());
    Ok(())
}

#[test]
fn quests6_galthen_literal_exits_do_not_create_a_progress_track() -> Result<(), String> {
    for (key, from, to) in [
        (
            "galthen-iksupan-exit-1",
            (34015, 31892, -8),
            (32728, 32880, -7),
        ),
        (
            "galthen-iksupan-exit-2",
            (34041, 31726, -10),
            (32728, 32880, -7),
        ),
    ] {
        let c = named(key)?;
        assert_eq!(
            c.selector.position,
            Some(Position {
                x: from.0,
                y: from.1,
                floor: from.2
            })
        );
        assert_eq!(
            c.teleport,
            Some(Position {
                x: to.0,
                y: to.1,
                floor: to.2
            })
        );
        assert!(c.storage.is_empty());
        assert!(c.minimum_level.is_none());
    }
    Ok(())
}

