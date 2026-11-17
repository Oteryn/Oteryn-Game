use super::*;

fn request() -> SetRequest {
    SetRequest {
        item: [3; 16],
        expected_revision: 4,
        display_kind: 1,
        direction: 2,
        key: "citizen".into(),
    }
}

fn panel(flagged: bool) -> Panel {
    Panel {
        selected: true,
        flagged,
        ..Default::default()
    }
}

#[test]
fn c0489_selected_set_request_appends_the_three_flags() -> Result<(), &'static str> {
    let mut p = panel(true);
    assert!(toggle(&mut p, Part::Outfit));
    let payload = set_payload(&p, &request()).ok_or("encode")?;
    let plain = request().encode().ok_or("plain")?;
    assert_eq!(payload[..plain.len()], plain[..]);
    assert_eq!(payload[plain.len()..], [1, 0, 1]);
    Ok(())
}

#[test]
fn c0489_unselected_session_keeps_the_v1_request_and_ignores_switches() -> Result<(), &'static str>
{
    let mut p = panel(false);
    assert!(!toggle(&mut p, Part::Mount));
    assert_eq!(set_payload(&p, &request()), request().encode());
    assert_eq!(flags_line(&p), None);
    let plain = Projection::encode(Some(&Projection {
        podium_kind: 1,
        display_kind: 1,
        direction: 2,
        revision: 9,
        key: "citizen".into(),
    }))
    .ok_or("projection")?;
    let (shown, flags) = decode_projection(&p, &plain).flatten().ok_or("decodes")?;
    assert_eq!((shown.revision, flags), (9, None));
    assert!(decode_projection(&panel(true), &plain).is_none());
    Ok(())
}

#[test]
fn c0489_selected_projection_reports_the_committed_flags() -> Result<(), &'static str> {
    let shown = Projection {
        podium_kind: 1,
        display_kind: 3,
        direction: 0,
        revision: 12,
        key: "wolf".into(),
    };
    let flags = DisplayFlags {
        show_platform: false,
        show_outfit: true,
        show_mount: false,
    };
    let payload = Projection::encode_flagged(Some((&shown, flags))).ok_or("encode")?;
    let p = panel(true);
    assert_eq!(
        decode_projection(&p, &payload),
        Some(Some((shown, Some(flags))))
    );
    assert_eq!(decode_projection(&p, &[0]), Some(None));
    assert!(decode_projection(&panel(false), &payload).is_none());
    let mut p = p;
    p.flags = flags;
    assert_eq!(
        flags_line(&p).as_deref(),
        Some("Platform hidden | outfit shown | mount hidden")
    );
    Ok(())
}

#[test]
fn c0489_switches_are_locked_while_a_request_is_pending() {
    let mut p = panel(true);
    p.pending = true;
    assert!(!toggle(&mut p, Part::Platform));
    assert!(p.flags.show_platform);
}
