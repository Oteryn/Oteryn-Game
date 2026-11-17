use super::*;
#[test]
fn gem_presentation_strict_bounded_codec_preserves_signed_executed_grades()
-> Result<(), &'static str> {
    let request = Request {
        nonce: 1,
        revision: 0,
    };
    let mut bytes = 1_u64.to_le_bytes().to_vec();
    bytes.extend(0_u64.to_le_bytes());
    assert_eq!(request.encode(), Some(bytes.clone()));
    assert_eq!(Request::decode(&bytes), Some(request));
    let reply = Reply {
        grades_current: true,
        qualified_gems: Vec::new(),
        request,
        catalogue: "pinned-catalogue".into(),
        vocation: "monk".into(),
        modifiers: vec![Modifier {
            kind: 0,
            id: 48,
            name: "Capacity".into(),
            effects: vec![
                Effect {
                    name: "Capacity".into(),
                    target: String::new(),
                    unit: 1,
                    values: [500000, 550000, 600000, 750000],
                },
                Effect {
                    name: "Weakness example".into(),
                    target: String::new(),
                    unit: 0,
                    values: [-100, -110, -120, -150],
                },
            ],
        }],
    };
    let bytes = reply.encode().ok_or("encode")?;
    assert_eq!(Reply::decode(&bytes), Some(reply.clone()));
    for n in 0..bytes.len() {
        assert!(Reply::decode(&bytes[..n]).is_none());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(Reply::decode(&extra).is_none());
    let mut invalid = reply.clone();
    invalid.modifiers.push(invalid.modifiers[0].clone());
    assert!(invalid.encode().is_none());
    invalid = reply.clone();
    invalid.qualified_gems = vec![[1; 16], [1; 16]];
    assert!(invalid.encode().is_none());
    invalid = reply;
    invalid.modifiers[0].effects[0].unit = 4;
    assert!(invalid.encode().is_none());
    assert!(
        Request {
            nonce: 0,
            revision: 1
        }
        .encode()
        .is_none()
    );
    Ok(())
}
