use super::*;
fn reply() -> Reply {
    Reply {
        request: Request {
            nonce: 1,
            wheel_revision: 2,
            atelier_revision: 3,
            points: [0; 36],
            vessels: [None; 4],
        },
        catalogue: "qualified-catalogue".into(),
        vocation: "knight".into(),
        available: 300,
        unlock_points: 295,
        removal_permitted: true,
        eligible: true,
        draft_valid: true,
        reason: String::new(),
        nodes: (0..36)
            .map(|_| Node {
                domain: 0,
                capacity: 50,
                minimum_points: 0,
                unlock_from: Vec::new(),
                name: "Test perk".into(),
                per_point: vec!["Maximum health: +1".into()],
                full_slot: vec!["Maximum health: +50".into()],
            })
            .collect(),
        revelations: (0..4)
            .map(|domain| Revelation {
                domain,
                name: "Test revelation".into(),
                stages: [250, 500, 1000]
                    .into_iter()
                    .map(|minimum_points| Stage {
                        minimum_points,
                        effects: vec!["Server-configured effect".into()],
                    })
                    .collect(),
            })
            .collect(),
        current: Vec::new(),
        draft: Vec::new(),
    }
}
#[test]
fn wheel_presentation_strict_codec_binds_full_draft_and_real_grid_shape() -> Result<(), &'static str>
{
    let r = reply();
    let bytes = r.encode().ok_or("reply")?;
    assert_eq!(Reply::decode(&bytes), Some(r.clone()));
    for n in 0..bytes.len() {
        assert!(Reply::decode(&bytes[..n]).is_none());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(Reply::decode(&extra).is_none());
    let mut invalid = r.clone();
    invalid.nodes[0].unlock_from = vec![36];
    assert!(invalid.encode().is_none());
    invalid = r.clone();
    invalid.nodes.pop();
    assert!(invalid.encode().is_none());
    invalid = r;
    invalid.request.nonce = 0;
    assert!(invalid.encode().is_none());
    Ok(())
}
#[test]
fn wheel_presentation_request_preserves_maximum_vessel_shape() -> Result<(), &'static str> {
    let request = Request {
        nonce: 1,
        wheel_revision: u64::MAX,
        atelier_revision: u64::MAX,
        points: [200; 36],
        vessels: [Some([1; 16]), Some([2; 16]), Some([3; 16]), Some([4; 16])],
    };
    let bytes = request.encode().ok_or("request")?;
    assert_eq!(bytes.len(), 164);
    assert_eq!(Request::decode(&bytes), Some(request));
    Ok(())
}
