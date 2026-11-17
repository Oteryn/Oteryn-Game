use super::*;
use oteryn_client_runtime::{CommandStatus, raw_feature::GatedCommandResult};
fn entry() -> wire::Entry {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    wire::Entry {
        hireling: id,
        name: "Alice".into(),
        sex: 2,
        look_type: 1107,
        colors: [1, 2, 3, 4],
        skills: 15,
        outfits: 1,
        position: Some((7, 9, -1)),
    }
}
#[test]
fn hireling_management_selects_server_identity_and_sends_outfit()
-> Result<(), Box<dyn std::error::Error>> {
    let mut p = Panel {
        selected: true,
        ..Default::default()
    };
    let list = request(&mut p, Intent::List).ok_or("list missing")?;
    assert_eq!(
        (list.capability, list.kind, list.payload),
        (250, 470, vec![0])
    );
    let e = entry();
    assert!(apply_result(
        &mut p,
        &GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: wire::Reply::State(vec![e.clone()])
                .encode()
                .ok_or("encode")?
        }
    ));
    assert_eq!(p.entries.as_slice(), std::slice::from_ref(&e));
    assert!(select(&mut p, 0));
    let outfit = request(
        &mut p,
        Intent::Outfit {
            look_type: 1109,
            colors: [5, 6, 7, 8],
        },
    )
    .ok_or("outfit missing")?;
    assert_eq!(
        wire::Request::decode(&outfit.payload),
        Some(wire::Request::Outfit {
            hireling: e.hireling,
            look_type: 1109,
            colors: [5, 6, 7, 8]
        })
    );
    assert!(apply_result(
        &mut p,
        &GatedCommandResult {
            command_id: 2,
            status: CommandStatus::DuplicateReplay,
            server_sequence: 2,
            payload: vec![1]
        }
    ));
    assert_eq!(p.entries[0].look_type, 1109);
    assert_eq!(p.entries[0].colors, [5, 6, 7, 8]);
    Ok(())
}
#[test]
fn hireling_management_gates_requests_and_handles_refused_and_malformed_results()
-> Result<(), Box<dyn std::error::Error>> {
    let e = entry();
    let mut p = Panel {
        selected: true,
        entries: vec![e.clone()],
        identity: Some(e.hireling),
        ..Default::default()
    };
    assert!(
        request(
            &mut p,
            Intent::Outfit {
                look_type: 1131,
                colors: [0; 4]
            }
        )
        .is_none()
    );
    for bytes in [vec![2, 4], vec![0, 1], vec![1, 0]] {
        assert!(request(&mut p, Intent::Return).is_some());
        assert!(request(&mut p, Intent::List).is_none());
        assert!(!apply_result(
            &mut p,
            &GatedCommandResult {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: bytes
            }
        ));
        assert_eq!(p.entries[0], e);
        assert!(p.feedback.is_some());
    }
    p.selected = false;
    assert!(request(&mut p, Intent::List).is_none());
    Ok(())
}

#[test]
fn hireling_configuration_displays_owned_jobs_and_pending_outfit_preview() {
    let e = entry();
    let p = Panel {
        selected: true,
        identity: Some(e.hireling),
        entries: vec![e.clone()],
        pending: Some(wire::Request::Outfit {
            hireling: e.hireling,
            look_type: 1109,
            colors: [5, 6, 7, 8],
        }),
        ..Panel::default()
    };
    let text = panel_text(&p);
    assert!(text.contains("Jobs: Banker, Cook, Steward, Trader"));
    assert!(text.contains("Preview: Alice (female), outfit 1109, colors [5, 6, 7, 8]"));
}
