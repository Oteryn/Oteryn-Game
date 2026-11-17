use super::*;
fn current() -> WheelPanel {
    let a = atelier::AtelierReply {
        refused: false,
        revision: 4,
        gems: vec![atelier::GemView {
            id: [1; 16],
            quality: 2,
            domain: 0,
            locked: false,
            vessel: false,
            modifiers: vec![48, 0, 5],
        }],
        grades: vec![],
        quality_quotes: vec![atelier::QualityQuote {
            quality: 0,
            reveal_gold: 123,
            switch_gold: 148,
            fragment_kind: 0,
            fragment_min: 9,
            fragment_max: 11,
        }],
        grade_quotes: vec![],
    };
    WheelPanel {
        selected: true,
        synchronized: true,
        confirmed_wheel: Some(wheel::WheelAllocationReply {
            refused: false,
            revision: 3,
            points: [0; 36],
            vessels: Some([None; 4]),
        }),
        confirmed_atelier: Some(a),
        gem: Some([1; 16]),
        ..Default::default()
    }
}
#[test]
fn gem_review_requires_confirmation_and_refuses_changed_expired_or_scope_state()
-> Result<(), &'static str> {
    let mut p = current();
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    assert!(stage(&mut p, Intent::Reveal, id));
    assert!(p.pending.is_none());
    assert!(
        p.gems
            .review
            .as_ref()
            .ok_or("review")?
            .text
            .contains("123 gold")
    );
    assert!(
        p.gems
            .review
            .as_ref()
            .ok_or("review")?
            .text
            .contains("unavailable")
    );
    p.quality = 1;
    assert!(confirm(&mut p).is_none());
    assert!(p.pending.is_none());
    p.quality = 0;
    assert!(stage(&mut p, Intent::Reveal, id));
    p.gems.review.as_mut().ok_or("review")?.expires = std::time::Instant::now();
    assert!(confirm(&mut p).is_none());
    assert!(stage(&mut p, Intent::Reveal, id));
    p.gems.invalidate();
    assert!(confirm(&mut p).is_none());
    assert!(stage(&mut p, Intent::Reveal, id));
    let exact = p.gems.review.as_ref().ok_or("review")?.command.clone();
    assert_eq!(confirm(&mut p), Some(exact));
    assert!(confirm(&mut p).is_none());
    Ok(())
}
#[test]
fn gem_reply_is_bound_to_current_revision_and_nonce() -> Result<(), &'static str> {
    let mut p = current();
    p.gems.selected = true;
    let command = query(&mut p).ok_or("query")?;
    let request = wire::Request::decode(&command.payload).ok_or("request")?;
    let reply = wire::Reply {
        grades_current: true,
        qualified_gems: Vec::new(),
        request,
        catalogue: "catalogue".into(),
        vocation: "monk".into(),
        modifiers: vec![wire::Modifier {
            kind: 0,
            id: 48,
            name: "Capacity".into(),
            effects: vec![wire::Effect {
                name: "Capacity".into(),
                target: String::new(),
                unit: 1,
                values: [500000, 550000, 600000, 750000],
            }],
        }],
    };
    let r = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: reply.encode().ok_or("encode")?,
    };
    p.confirmed_atelier.as_mut().ok_or("atelier")?.revision += 1;
    apply(&mut p, &command, &r);
    assert!(p.gems.reply.is_none());
    p.gems.invalidate();
    apply(&mut p, &command, &r);
    assert!(p.gems.reply.is_none());
    Ok(())
}

#[test]
fn gem_current_catalogue_qualification_does_not_claim_old_values_are_active()
-> Result<(), &'static str> {
    let mut p = current();
    p.confirmed_atelier
        .as_mut()
        .ok_or("atelier")?
        .grades
        .push(atelier::GradeView {
            kind: 0,
            mod_id: 48,
            grade: 3,
        });
    p.modifier = Some((0, 48));
    p.gems.reply = Some(wire::Reply {
        request: wire::Request {
            nonce: 1,
            revision: 4,
        },
        catalogue: "current-source".into(),
        vocation: "monk".into(),
        grades_current: false,
        qualified_gems: Vec::new(),
        modifiers: vec![wire::Modifier {
            kind: 0,
            id: 48,
            name: "Capacity".into(),
            effects: vec![wire::Effect {
                name: "Capacity".into(),
                target: String::new(),
                unit: 1,
                values: [500000, 550000, 600000, 750000],
            }],
        }],
    });
    let text = view::details(&p);
    assert!(text.contains("earlier catalogue"));
    assert!(text.contains("current rules use Grade I"));
    assert!(text.contains("Grade 1: +500 ← current"));
    assert!(!text.contains("Grade 4: +750 ← current"));
    Ok(())
}

#[test]
fn gem_review_and_wheel_preview_requests_are_serialized() -> Result<(), &'static str> {
    let mut p = current();
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    p.diagram.selected = true;
    assert!(super::super::diagram::query(&mut p).is_some());
    assert!(!stage(&mut p, Intent::Reveal, id));
    p.diagram.pending = None;
    assert!(stage(&mut p, Intent::Reveal, id));
    assert!(super::super::diagram::query(&mut p).is_none());
    assert!(confirm(&mut p).is_some());
    Ok(())
}
