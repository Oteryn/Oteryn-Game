use super::*;
fn model() -> WheelPanel {
    let request = wire::Request {
        nonce: 1,
        wheel_revision: 2,
        atelier_revision: 3,
        points: [0; 36],
        vessels: [None; 4],
    };
    let mut p = WheelPanel {
        selected: true,
        synchronized: true,
        confirmed_wheel: Some(wheel::WheelAllocationReply {
            refused: false,
            revision: 2,
            points: [0; 36],
            vessels: Some([None; 4]),
        }),
        confirmed_atelier: Some(atelier::AtelierReply {
            refused: false,
            revision: 3,
            gems: Vec::new(),
            grades: Vec::new(),
            quality_quotes: Vec::new(),
            grade_quotes: Vec::new(),
        }),
        ..Default::default()
    };
    p.diagram.selected = true;
    p.diagram.reply = Some(wire::Reply {
        request,
        catalogue: "qualified".into(),
        vocation: "knight".into(),
        available: 300,
        unlock_points: 300,
        removal_permitted: true,
        eligible: true,
        draft_valid: true,
        reason: String::new(),
        nodes: (0..36)
            .map(|_| wire::Node {
                domain: 0,
                capacity: 50,
                minimum_points: 0,
                unlock_from: Vec::new(),
                name: "Server perk".into(),
                per_point: Vec::new(),
                full_slot: Vec::new(),
            })
            .collect(),
        revelations: (0..4)
            .map(|domain| wire::Revelation {
                domain,
                name: "Server revelation".into(),
                stages: [250, 500, 1000]
                    .into_iter()
                    .map(|minimum_points| wire::Stage {
                        minimum_points,
                        effects: Vec::new(),
                    })
                    .collect(),
            })
            .collect(),
        current: Vec::new(),
        draft: Vec::new(),
    });
    p
}
#[test]
fn wheel_graph_edges_and_cpu_pixels_use_only_declared_links_and_selected_slot()
-> Result<(), &'static str> {
    let mut p = model();
    let r = p.diagram.reply.as_mut().ok_or("reply")?;
    r.nodes[0].unlock_from = vec![1];
    r.nodes[5].unlock_from = vec![11];
    r.nodes[11].unlock_from = vec![5];
    assert_eq!(graph::edges(&r.nodes), Some(vec![(0, 1), (5, 11)]));
    let first = graph::rgba(&p).ok_or("pixels")?;
    assert_eq!(first.len(), 384 * 384 * 4);
    assert!(slot(&mut p, 1, 0));
    let second = graph::rgba(&p).ok_or("pixels")?;
    assert!(
        first != second,
        "selecting another node must move the visible highlight"
    );
    assert!(!slot(&mut p, 0, 2));
    Ok(())
}
#[test]
fn wheel_preview_cannot_submit_a_changed_unreviewed_or_stale_scope_draft()
-> Result<(), &'static str> {
    let mut p = model();
    assert!(slot(&mut p, 1, 0));
    assert!(points(&mut p, 50));
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    assert!(submit(&mut p, id).is_none());
    let c = query(&mut p).ok_or("query")?;
    let q = wire::Request::decode(&c.payload).ok_or("request")?;
    let mut reply = p.diagram.reply.clone().ok_or("metadata")?;
    reply.request = q;
    let r = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: reply.encode().ok_or("reply")?,
    };
    apply(&mut p, &c, &r);
    let command = submit(&mut p, id).ok_or("save")?;
    assert_eq!(
        wheel::decode_wheel_request(&command.payload)
            .map_err(|_| "old request")?
            .action,
        1
    );
    assert!(submit(&mut p, id).is_none());
    p.diagram.invalidate();
    assert!(metadata(&p).is_none());
    Ok(())
}
#[test]
fn wheel_reset_draft_requires_a_new_valid_preview_and_uses_original_reset()
-> Result<(), &'static str> {
    let mut p = model();
    p.points[14] = 50;
    p.confirmed_wheel.as_mut().ok_or("wheel")?.points[14] = 50;
    p.diagram.reply.as_mut().ok_or("reply")?.request.points[14] = 50;
    assert!(reset_draft(&mut p));
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    assert!(submit(&mut p, id).is_none());
    let c = query(&mut p).ok_or("preview")?;
    let q = wire::Request::decode(&c.payload).ok_or("request")?;
    let mut reply = p.diagram.reply.clone().ok_or("metadata")?;
    reply.request = q;
    let r = GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: reply.encode().ok_or("reply")?,
    };
    apply(&mut p, &c, &r);
    let command = submit(&mut p, id).ok_or("reset")?;
    assert_eq!(
        wheel::decode_wheel_request(&command.payload)
            .map_err(|_| "old reset")?
            .action,
        2
    );
    Ok(())
}
#[test]
fn vessel_node_shows_the_gem_vessel_perk_text_and_no_domain_name() -> Result<(), &'static str> {
    let mut p = model();
    p.diagram.reply.as_mut().ok_or("reply")?.nodes[14].name = "Gem Vessel".into();
    assert!(slot(&mut p, 1, 14));
    let text = view::text(&p);
    assert!(
        text.contains(
            "Each node grants +1 Damage / Healing and unlocks a vessel in the Gem Atelier."
        )
    );
    assert!(!text.contains("Green"));
    assert!(slot(&mut p, 1, 13));
    assert!(!view::text(&p).contains("unlocks a vessel"));
    Ok(())
}
