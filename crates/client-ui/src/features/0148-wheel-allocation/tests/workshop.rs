use super::*;

fn grade(kind: u8, mod_id: u16, grade: u8) -> atelier::GradeView {
    atelier::GradeView {
        kind,
        mod_id,
        grade,
    }
}

#[test]
fn workshop_query_modifiers_can_be_upgraded_without_any_owned_gem() -> Result<(), Error> {
    let mut current = panel();
    let atelier = current.confirmed_atelier.as_mut().ok_or("atelier")?;
    atelier.gems.clear();
    atelier.grades = vec![grade(0, 37, 0), grade(1, 0, 0)];
    current.select_gem(None);
    assert_eq!(current.modifiers(), vec![(0, 37), (1, 0)]);
    assert!(current.selected_gem().is_none());
    assert!(current.ready());
    for (index, expected) in current.modifiers().into_iter().enumerate() {
        let mut panel = current.clone();
        // The existing native list maps its selected row through this bounded list.
        panel.modifier = panel.modifiers().get(index).copied();
        let before = panel.confirmed_atelier.clone();
        let command = request(&mut panel, Intent::GradeUp, occurrence(11))
            .ok_or("unseen modifier upgrade")?;
        assert_eq!(command.kind, atelier::COMMAND_TYPE_ATELIER_MUTATE);
        assert!(command.mutation);
        assert_eq!(
            wire(atelier::decode_mutate(&command.payload))?,
            atelier::MutateRequest {
                occurrence: occurrence(11),
                expected_revision: 4,
                mutation: atelier::Mutation::GradeUp {
                    kind: expected.0,
                    mod_id: expected.1
                },
            }
        );
        assert_eq!(panel.confirmed_atelier, before);
        assert_eq!(panel.grade(expected.0, expected.1), 0);
    }
    current.modifier = Some((1, 0));
    assert!(panel_text(&current).contains("Upgrade Dodge · Grade I:"));
    Ok(())
}

#[test]
fn workshop_legacy_fallback_deduplicates_pairs_and_keeps_gem_details() -> Result<(), Error> {
    let mut panel = panel();
    // A sparse legacy reply has the upgraded supreme grade and two ungraded basic
    // modifiers known only from the selected gem.
    assert_eq!(panel.modifiers(), vec![(0, 0), (0, 31), (1, 5)]);
    panel
        .confirmed_atelier
        .as_mut()
        .ok_or("atelier")?
        .grades
        .clear();
    assert_eq!(panel.modifiers(), vec![(0, 0), (0, 31), (1, 5)]);
    panel.modifier = Some((0, 31));
    let command =
        request(&mut panel, Intent::GradeUp, occurrence(12)).ok_or("legacy gem modifier")?;
    assert_eq!(
        wire(atelier::decode_mutate(&command.payload))?.mutation,
        atelier::Mutation::GradeUp {
            kind: 0,
            mod_id: 31
        }
    );
    panel.pending = None;
    panel.confirmed_atelier.as_mut().ok_or("atelier")?.grades = vec![
        grade(1, 0, 0),
        grade(0, 0, 0),
        grade(1, 0, 0),
        grade(1, 5, 1),
    ];
    assert_eq!(panel.modifiers(), vec![(0, 0), (0, 31), (1, 0), (1, 5)]);
    let text = panel_text(&panel);
    assert!(text.contains("Selected gem modifiers: Hit Points · Grade I; Physical Resistance · Grade I; Revelation Mastery Gift of Life · Grade II."));
    assert!(!text.contains("Selected gem modifiers: Dodge"));
    Ok(())
}

#[test]
fn workshop_selection_survives_gem_changes_and_uses_the_query_after_replay() -> Result<(), Error> {
    let mut panel = panel();
    let atelier = panel.confirmed_atelier.as_mut().ok_or("atelier")?;
    atelier.grades = vec![grade(0, 3, 0), grade(1, 0, 0)];
    atelier.gems.push(atelier::GemView {
        id: [18; 16],
        quality: 0,
        domain: 1,
        locked: false,
        vessel: false,
        modifiers: vec![3],
    });
    panel.modifier = Some((1, 0));
    panel.select_gem(Some([18; 16]));
    assert_eq!(panel.modifier, Some((1, 0)));
    panel.select_gem(None);
    assert_eq!(panel.modifier, Some((1, 0)));
    request(&mut panel, Intent::GradeUp, occurrence(13)).ok_or("grade request")?;

    let (_, mut historical) = current(5);
    historical.grades = vec![grade(1, 0, 1)];
    historical.quality_quotes.clear();
    historical.grade_quotes.clear();
    let (wheel, mut atelier) = current(6);
    atelier.gems.clear();
    atelier.grades = vec![grade(0, 3, 0), grade(1, 0, 2)];
    let reply = response(
        atelier::COMMAND_TYPE_ATELIER_MUTATE,
        CommandStatus::DuplicateReplay,
        wire(atelier::encode_reply(&historical))?,
        &wheel,
        &atelier,
    )?;
    assert!(apply_result(&mut panel, &reply));
    assert_eq!(panel.confirmed_atelier, Some(atelier));
    assert_eq!(panel.modifier, Some((1, 0)));
    assert_eq!(panel.grade(1, 0), 2);
    assert!(panel.selected_gem().is_none());
    assert!(panel.ready());
    assert!(panel.retry.is_none());

    // A later current catalogue may remove a choice; the selection must then move
    // to a present row instead of leaving a stale kind/id eligible for submission.
    request(&mut panel, Intent::Refresh, [0; 16]).ok_or("refresh")?;
    let (wheel, mut atelier) = current(7);
    atelier.gems.clear();
    atelier.grades = vec![grade(0, 3, 0)];
    let reply = response(
        wheel::COMMAND_TYPE_WHEEL_ALLOCATION,
        CommandStatus::Accepted,
        wire(wheel::encode_wheel_reply(&wheel))?,
        &wheel,
        &atelier,
    )?;
    assert!(apply_result(&mut panel, &reply));
    assert_eq!(panel.modifier, Some((0, 3)));
    assert!(!panel.modifiers().contains(&(1, 0)));
    Ok(())
}

#[test]
fn workshop_invalid_or_out_of_range_selection_sends_no_upgrade() -> Result<(), Error> {
    let mut panel = panel();
    let atelier = panel.confirmed_atelier.as_mut().ok_or("atelier")?;
    atelier.gems.clear();
    atelier.grades = vec![grade(1, 0, 0)];
    panel.select_gem(None);
    let before = panel.confirmed_atelier.clone();
    let choices = panel.modifiers();
    panel.modifier = choices.get(choices.len()).copied();
    assert!(request(&mut panel, Intent::GradeUp, occurrence(14)).is_none());
    assert_eq!(panel.feedback, Some("Select a Workshop modifier first"));
    for invalid in [None, Some((0, 0)), Some((1, 1)), Some((2, 0))] {
        panel.modifier = invalid;
        assert!(request(&mut panel, Intent::GradeUp, occurrence(14)).is_none());
        assert!(panel.pending.is_none());
        assert_eq!(panel.confirmed_atelier, before);
    }
    Ok(())
}
