use super::*;

fn state(values: &[(&str, i64)]) -> BTreeMap<String, i64> {
    values
        .iter()
        .map(|(key, value)| (format!("{TRACK_PREFIX}{key}"), *value))
        .collect()
}

#[test]
fn donor_keyword_matching_rejects_prefixed_occurrences_but_allows_suffixes() {
    assert!(message_contains("MISSION", "mission"));
    assert!(message_contains("a mission please", "mission"));
    assert!(message_contains("missions", "mission"));
    assert!(!message_contains("admission", "mission"));
    assert!(!message_contains("admission mission", "mission"));
    assert!(!message_contains("yes", "mission"));
}

#[test]
fn first_investigation_requires_two_contextual_replies_before_start() -> Result<(), String> {
    let book = load()?;
    let tracks = state(&[]);
    assert!(
        book.select("oteryn:npc.ezebeth", "strange", 0, &tracks)
            .is_none()
    );
    let invite = book
        .select("oteryn:npc.ezebeth", "mission", 0, &tracks)
        .ok_or("invite")?;
    assert_eq!(invite.next_topic, Some(1));
    assert!(invite.sets.is_empty());
    let help = book
        .select("oteryn:npc.ezebeth", "assistance", 1, &tracks)
        .ok_or("help")?;
    assert_eq!(help.next_topic, Some(2));
    let start = book
        .select("oteryn:npc.ezebeth", "strange", 2, &tracks)
        .ok_or("start")?;
    assert_eq!(start.sets[0].track, format!("{TRACK_PREFIX}mission01"));
    assert_eq!(start.sets[0].value, 1);
    assert_eq!(start.next_topic, Some(0));
    assert!(book.reply(start)?.starts_with("They usually know better"));
    Ok(())
}

#[test]
fn repeat_mission_does_not_reopen_the_first_topic() -> Result<(), String> {
    let book = load()?;
    let reply = book
        .select(
            "oteryn:npc.ezebeth",
            "mission",
            2,
            &state(&[("mission01", 1)]),
        )
        .ok_or("already")?;
    assert_eq!(reply.key, "ezebeth-already");
    assert_eq!(reply.next_topic, Some(0));
    assert!(reply.sets.is_empty());
    Ok(())
}

#[test]
fn beggar_acceptance_updates_both_missions_in_one_transition() -> Result<(), String> {
    let book = load()?;
    let tracks = state(&[("mission01", 1)]);
    assert!(
        book.select("oteryn:npc.a_beggar", "yes", 0, &tracks)
            .is_none()
    );
    let ask = book
        .select("oteryn:npc.a_beggar", "want", 0, &tracks)
        .ok_or("want")?;
    assert_eq!(ask.next_topic, Some(1));
    let accept = book
        .select("oteryn:npc.a_beggar", "yes", 1, &tracks)
        .ok_or("yes")?;
    let catalogue =
        extend(super::super::QuestStateCatalogue::empty("dark-trails-test").ok_or("empty")?)?;
    let change = catalogue
        .evaluate(
            catalogue
                .transition(&transition_key(&accept.key))
                .ok_or("transition")?,
            &tracks,
            0,
        )
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(change.len(), 2);
    assert!(
        change
            .iter()
            .any(|v| v.track.ends_with("mission01") && v.after == 2)
    );
    assert!(
        change
            .iter()
            .any(|v| v.track.ends_with("mission02") && v.after == 1)
    );
    Ok(())
}

#[test]
fn brothers_only_report_a_discovered_ritual_in_the_report_topic() -> Result<(), String> {
    let book = load()?;
    for npc in ["oteryn:npc.jacob", "oteryn:npc.elliott"] {
        assert!(book.select(npc, "report", 7, &state(&[])).is_none());
        let tracks = state(&[("mission05", 1)]);
        let repeat = book.select(npc, "report", 0, &tracks).ok_or("repeat")?;
        assert!(repeat.sets.is_empty());
        let report = book.select(npc, "report", 7, &tracks).ok_or("report")?;
        assert_eq!(report.sets[0].track, format!("{TRACK_PREFIX}mission06"));
    }
    Ok(())
}

#[test]
fn read_guards_are_revalidated_by_the_durable_transition_without_rewriting_them()
-> Result<(), String> {
    let book = load()?;
    let before = state(&[("mission06", 1)]);
    let rule = book
        .select("oteryn:npc.barazbaz", "ritual", 0, &before)
        .ok_or("ritual")?;
    let catalogue =
        extend(super::super::QuestStateCatalogue::empty("dark-trails-test").ok_or("empty")?)?;
    let transition = catalogue
        .transition(&transition_key(&rule.key))
        .ok_or("transition")?;
    let applied = catalogue
        .evaluate(transition, &before, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert!(
        applied
            .iter()
            .any(|change| change.track.ends_with("mission06")
                && change.before == 1
                && change.after == 1)
    );
    assert!(
        catalogue
            .evaluate(transition, &state(&[("mission06", 0)]), 0)
            .is_err()
    );
    assert!(
        catalogue
            .evaluate(transition, &state(&[("mission06", 1), ("mission07", 1)]), 0)
            .is_err()
    );
    Ok(())
}

#[test]
fn roswitha_grants_the_door_and_mission_together() -> Result<(), String> {
    let book = load()?;
    let rule = book
        .select(
            "oteryn:npc.roswitha",
            "Harsin",
            0,
            &state(&[("mission13", 1)]),
        )
        .ok_or("harsin")?;
    assert_eq!(rule.sets.len(), 2);
    assert!(
        rule.sets
            .iter()
            .any(|set| set.track.ends_with("door_quandon") && set.value == 1)
    );
    assert!(
        book.select(
            "oteryn:npc.roswitha",
            "Harsin",
            0,
            &state(&[("mission13", 1), ("mission14", 1)])
        )
        .is_none()
    );
    Ok(())
}

#[test]
fn murder_report_checks_the_corpse_stage() -> Result<(), String> {
    let book = load()?;
    let missing = book
        .select(
            "oteryn:npc.sholley",
            "quandon",
            0,
            &state(&[("mission14", 1)]),
        )
        .ok_or("missing")?;
    assert!(missing.sets.is_empty());
    let found = book
        .select(
            "oteryn:npc.sholley",
            "quandon",
            0,
            &state(&[("mission14", 2)]),
        )
        .ok_or("found")?;
    assert_eq!(found.sets[0].value, 1);
    assert!(found.sets[0].track.ends_with("mission15"));
    Ok(())
}

#[test]
fn terrence_uses_the_donor_effort_topic_without_inventing_a_kill_quota() -> Result<(), String> {
    let book = load()?;
    let before = state(&[("mission16", 1)]);
    assert!(
        book.select("oteryn:npc.terrence", "effort", 0, &before)
            .is_none()
    );
    let manway = book
        .select("oteryn:npc.terrence", "manway", 0, &before)
        .ok_or("manway")?;
    assert_eq!(manway.next_topic, Some(1));
    let effort = book
        .select("oteryn:npc.terrence", "effort", 1, &before)
        .ok_or("effort")?;
    assert!(effort.guards.is_empty());
    assert_eq!(effort.sets.len(), 2);
    assert!(
        effort
            .sets
            .iter()
            .any(|set| set.track.ends_with("door_hideout") && set.value == 1)
    );
    Ok(())
}

#[test]
fn ghost_greeting_requires_the_completed_seance() -> Result<(), String> {
    let book = load()?;
    assert!(
        book.select(
            "oteryn:npc.quandons_ghost",
            "hi",
            0,
            &state(&[("mission15", 2)])
        )
        .ok_or("default greeting")?
        .sets
        .is_empty()
    );
    let rule = book
        .select(
            "oteryn:npc.quandons_ghost",
            "hi",
            0,
            &state(&[("mission15", 3)]),
        )
        .ok_or("greeting")?;
    assert_eq!(book.replies[&rule.reply].lines.len(), 3);
    assert!(
        book.select(
            "oteryn:npc.quandons_ghost",
            "hi",
            0,
            &state(&[("mission15", 3), ("mission16", 1)])
        )
        .ok_or("repeat greeting")?
        .sets
        .is_empty()
    );
    assert!(
        !book
            .select(
                "oteryn:npc.quandons_ghost",
                "hello",
                0,
                &state(&[("mission15", 3)])
            )
            .ok_or("hello")?
            .sets
            .is_empty()
    );
    Ok(())
}

#[test]
fn unsupported_source_branches_still_shadow_later_supported_keywords() -> Result<(), String> {
    let book = load()?;
    let tracks = state(&[("mission05", 1)]);
    assert!(
        book.select("oteryn:npc.jacob", "mission report", 7, &tracks)
            .is_none()
    );
    assert!(
        book.select(
            "oteryn:npc.sholley",
            "friend quandon",
            0,
            &state(&[("mission14", 2)])
        )
        .is_none()
    );
    Ok(())
}

#[test]
fn every_runtime_rule_has_full_source_text_and_a_pinned_source() -> Result<(), String> {
    let book = load()?;
    for rule in &book.rules {
        assert!(!book.reply(rule)?.is_empty(), "{}", rule.key);
        assert!(rule.sources.iter().all(|s| {
            s.starts_with("zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8:")
        }));
    }
    Ok(())
}

#[test]
fn all_25_rules_select_and_mutations_use_the_registered_catalogue() -> Result<(), String> {
    let book = load()?;
    let expected = [
        "ezebeth-invite",
        "ezebeth-already",
        "ezebeth-assistance",
        "ezebeth-start",
        "beggar-want",
        "beggar-want-other",
        "beggar-report",
        "jacob-investigate",
        "jacob-report",
        "jacob-already",
        "elliott-investigate",
        "elliott-report",
        "elliott-already",
        "barazbaz-ritual",
        "barazbaz-archives",
        "roswitha-harsin",
        "sholley-quandon",
        "sholley-body-missing",
        "terrence-manway",
        "terrence-manway-denied",
        "terrence-effort",
        "ghost-witness",
        "ghost-witness-hello",
        "ghost-greet-other",
        "ghost-hello-other",
    ];
    assert_eq!(
        book.rules
            .iter()
            .map(|r| r.key.as_str())
            .collect::<Vec<_>>(),
        expected
    );
    let lowered = super::super::loader::load_embedded_quest_state("dark-trails-all-rules")
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = lowered.catalogue();
    for rule in &book.rules {
        let tracks: BTreeMap<_, _> = rule
            .guards
            .iter()
            .map(|guard| (guard.track.clone(), guard.minimum))
            .collect();
        let selected = book
            .select(&rule.npc, &rule.keyword, rule.topic.unwrap_or(0), &tracks)
            .ok_or_else(|| format!("unreachable rule {}", rule.key))?;
        assert_eq!(selected.key, rule.key);
        assert!(!book.replies[&selected.reply].lines.is_empty());
        let transition = catalogue.transition(&transition_key(&rule.key));
        if rule.sets.is_empty() {
            assert!(transition.is_none(), "{} has no durable effect", rule.key);
            continue;
        }
        let transition = transition.ok_or("registered transition")?;
        let changes = catalogue
            .evaluate(transition, &tracks, 0)
            .map_err(|e| format!("{e:?}"))?;
        for set in &rule.sets {
            assert!(
                changes
                    .iter()
                    .any(|change| change.track == set.track && change.after == set.value),
                "{}",
                rule.key
            );
        }
        for guard in &rule.guards {
            let mut stale = tracks.clone();
            stale.insert(guard.track.clone(), guard.maximum + 1);
            assert!(
                catalogue.evaluate(transition, &stale, 0).is_err(),
                "{}",
                rule.key
            );
        }
    }
    Ok(())
}
