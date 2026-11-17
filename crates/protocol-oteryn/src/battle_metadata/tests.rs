use super::*;
#[test]
fn battle_metadata_codec_is_bounded_and_preserves_generation_and_unknowns()
-> Result<(), &'static str> {
    let entity = |id| EntityRef {
        identity: [id; 16],
        generation: 7,
    };
    let query = vec![entity(1), entity(2)];
    assert_eq!(
        decode_query(&encode_query(&query).ok_or("encode")?),
        Some(query.clone())
    );
    let rows = vec![
        PlayerMetadata {
            entity: entity(1),
            vocation: Some(4),
            same_guild: Some(true),
        },
        PlayerMetadata {
            entity: entity(2),
            vocation: None,
            same_guild: None,
        },
    ];
    let payload = encode_reply(&rows).ok_or("reply")?;
    assert_eq!(decode_reply(&payload), Some(rows));
    assert!(encode_query(&[query[0], query[0]]).is_none());
    assert!(encode_query(&vec![entity(1); 17]).is_none());
    assert!(
        encode_query(&[EntityRef {
            generation: 0,
            ..entity(1)
        }])
        .is_none()
    );
    let mut invalid = payload.clone();
    invalid[25] = 6;
    assert!(decode_reply(&invalid).is_none());
    assert!(decode_reply(&payload[..payload.len() - 1]).is_none());
    assert!(decode_query(&[17]).is_none());
    Ok(())
}
