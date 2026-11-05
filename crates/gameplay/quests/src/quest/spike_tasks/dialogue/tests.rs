use super::*;

#[test]
fn full_officer_source_text_and_other_ten_npcs_are_preserved() -> Result<(), String> {
    let book = load()?;
    assert_eq!(book.replies.len(), 166);
    assert_eq!(book.contacts.len(), 9);
    assert_eq!(book.parcel_replies.len(), 4);
    let tracks = BTreeMap::new();
    let plan = book
        .respond(
            "oteryn:npc.gnomilly",
            None,
            "area",
            &Facts {
                tracks: &tracks,
                level: 25,
                now: 1,
            },
        )
        .ok_or("area")?;
    assert_eq!(plan.reply.len(), 6);
    assert!(plan.reply[0].contains("monumental battle"));
    Ok(())
}

#[test]
fn exact_deadline_is_still_closed_and_next_second_reopens() -> Result<(), String> {
    let book = load()?;
    let task = book.task("upper", "kill").ok_or("task")?;
    let tracks = BTreeMap::from([(report_track(task), 100)]);
    assert_eq!(source_daily(task, &tracks), Some(72100));
    let closed = book
        .respond(
            "oteryn:npc.gnomilly",
            None,
            "kill",
            &Facts {
                tracks: &tracks,
                level: 25,
                now: 72100,
            },
        )
        .ok_or("closed")?;
    assert_eq!(closed.next_topic, None);
    assert_eq!(
        closed.reply,
        vec!["Sorry, you have to wait  before this task gets available again."]
    );
    let open = book
        .respond(
            "oteryn:npc.gnomilly",
            None,
            "kill",
            &Facts {
                tracks: &tracks,
                level: 25,
                now: 72101,
            },
        )
        .ok_or("open")?;
    assert_eq!(open.next_topic.as_deref(), Some("kill"));
    Ok(())
}

#[test]
fn source_menu_and_actual_offer_have_distinct_level_checks() -> Result<(), String> {
    let book = load()?;
    let tracks = BTreeMap::new();
    let facts = Facts {
        tracks: &tracks,
        level: 1,
        now: 1,
    };
    let menu = book
        .respond("oteryn:npc.gnomilly", None, "mission", &facts)
        .ok_or("menu")?;
    assert!(menu.reply[0].contains("I can offer you"));
    let task = book
        .respond("oteryn:npc.gnomilly", None, "kill", &facts)
        .ok_or("task")?;
    assert_eq!(
        task.reply,
        vec!["Sorry, you are not on the required range of levels [25-49]."]
    );
    assert_eq!(task.next_topic, None);
    Ok(())
}

#[test]
fn string_diff_retains_the_source_units_and_separators() {
    assert_eq!(time_text(0), Some(String::new()));
    assert_eq!(time_text(1), Some("1 second".into()));
    assert_eq!(time_text(61), Some("1 minute and 1 second".into()));
    assert_eq!(
        time_text(90061),
        Some("1 day, 1 hour, 1 minute and 1 second".into())
    );
    assert_eq!(time_text(-1), None);
}

#[test]
fn report_mode_has_priority_over_new_offers_and_always_clears() -> Result<(), String> {
    let book = load()?;
    let tracks = BTreeMap::new();
    let facts = Facts {
        tracks: &tracks,
        level: 25,
        now: 1,
    };
    let plan = book
        .respond("oteryn:npc.gnomilly", Some("report"), "killing yes", &facts)
        .ok_or("report")?;
    assert_eq!(plan.reply, vec!["You have not started that mission."]);
    assert_eq!(plan.next_topic, None);
    assert!(plan.starts.is_empty());
    let invalid = book
        .respond("oteryn:npc.gnomilly", Some("report"), "yes", &facts)
        .ok_or("invalid")?;
    assert_eq!(invalid.reply, vec!["That's not a valid mission name."]);
    assert_eq!(invalid.next_topic, None);
    Ok(())
}
