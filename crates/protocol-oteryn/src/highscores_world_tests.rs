use super::*;
#[test]
fn cross_world_query_and_position_metadata_are_opt_in_bounded_and_strict() -> WireResult<()> {
    let world = WorldId::decode(&[90, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 90])
        .map_err(|_| HighscoresWireError::Malformed)?;
    let query = HighscoresQuery {
        world: Some(world),
        details: true,
        category: 1,
        vocation: 1,
        page: 2,
        snapshot_id: 4,
    };
    let bytes = encode_highscores_query(query)?;
    assert_eq!(decode_highscores_query(&bytes)?, query);
    assert!(decode_highscores_query(&[bytes.as_slice(), &[0x30, 1]].concat()).is_err());
    assert!(decode_highscores_query(&[0x08, 1, 0x18, 1, 0x30, 2]).is_err());
    assert!(decode_highscores_query(&[0x08, 1, 0x18, 1, 0x2a, 0]).is_err());
    let legacy = HighscoresQuery {
        world: None,
        details: false,
        page: 1,
        snapshot_id: 0,
        ..query
    };
    assert_eq!(
        encode_highscores_query(legacy)?,
        [0x08, 1, 0x10, 1, 0x18, 1]
    );
    let mut result = HighscoresResult {
        world: Some(world),
        own_page: Some(2),
        category: 1,
        snapshot_id: 4,
        computed_at: 0,
        page: 1,
        has_more: false,
        rows: vec![],
        own_row: Some(HighscoreRow {
            rank: 1500,
            name: "Own Knight".into(),
            vocation: "knight".into(),
            level: 100,
            value: 500,
        }),
    };
    let bytes = encode_highscores_result(&result)?;
    assert_eq!(decode_highscores_result(&bytes)?, result);
    assert!(decode_highscores_result(&[bytes.as_slice(), &[0x40, 2]].concat()).is_err());
    for bad in [0, 21, u32::MAX] {
        result.own_page = Some(bad);
        assert!(encode_highscores_result(&result).is_err());
    }
    result.own_page = Some(2);
    result.own_row = None;
    assert!(encode_highscores_result(&result).is_err());
    Ok(())
}
