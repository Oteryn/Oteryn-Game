use super::*;
#[test]
fn atlas_query_uses_existing_map_command_without_changing_legacy_requests()
-> Result<(), MapWireError> {
    let query = [128, 127, 255, 255, 255, 255, 0, 0, 0];
    assert_eq!(decode_request(72, &query)?.operation, 128);
    assert_eq!(decode_request(72, &[])?.operation, 0);
    assert!(decode_request(73, &query).is_err());
    for n in 0..query.len() {
        assert!(n == 0 || decode_request(72, &query[..n]).is_err());
    }
    Ok(())
}

#[test]
fn atlas_codec_bounds_geometry_labels_pages_and_truncated_frames() -> Result<(), MapWireError> {
    use atlas::*;
    let mut a = Atlas {
        query: Query {
            floor: -7,
            x: 10,
            y: 20,
            zoom: 0,
            page: 0,
        },
        digest: [3; 32],
        floors: 128,
        progress: Some(MapResult {
            code: 0,
            revision: 1,
            discovered: false,
            found: 0,
            area_percent: 0,
            pool: 0,
            epoch: 0,
            improved: false,
            visible: vec![],
        }),
        cells: vec![2; WIDTH * HEIGHT],
        areas: vec![
            Area {
                id: 1,
                parent: 0,
                name: "Region".into(),
            },
            Area {
                id: 2,
                parent: 1,
                name: "Harbour".into(),
            },
        ],
        markers: vec![Marker {
            kind: 1,
            x: 11,
            y: 21,
            label: "oteryn:npc.guide".into(),
        }],
        marker_total: 1,
    };
    let bytes = a.encode()?;
    assert_eq!(Atlas::decode(&bytes)?, a);
    for n in 0..bytes.len() {
        assert!(Atlas::decode(&bytes[..n]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(Atlas::decode(&extra).is_err());
    let saved = a.progress.take();
    assert!(Atlas::decode(&a.encode()?)?.progress.is_none());
    a.progress = saved;
    a.query.floor = -128;
    assert!(a.encode().is_err());
    a.query.floor = -7;
    a.markers[0].kind = 0;
    assert!(a.encode().is_err());
    a.markers[0].kind = 1;
    a.query.page = 1;
    assert!(a.encode().is_err());
    a.query.page = 0;
    a.areas[0].name = "a".repeat(129);
    assert!(a.encode().is_err());
    Ok(())
}
