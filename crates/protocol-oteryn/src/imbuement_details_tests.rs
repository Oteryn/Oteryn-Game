use super::*;
#[test]
fn imbuement_details_strict_roundtrip_and_material_bounds() -> Result<(), Error> {
    let item = ItemHandle::new(7).ok_or(Error::Malformed)?;
    let reply = Reply {
        item,
        quotes: vec![Quote {
            family: 1,
            tier: 1,
            gold: 7500,
            requires_premium: false,
            recipe_available: true,
            success_percent: 100,
            materials: vec![Material {
                definition_key: "oteryn:item.tibia.i11444".into(),
                quantity: 20,
            }],
        }],
    };
    let bytes = encode_reply(&reply)?;
    assert_eq!(decode_query(&encode_query(item))?, item);
    assert_eq!(decode_reply(&bytes)?, reply);
    assert!(decode_query(&[8, 0]).is_err());
    let mut duplicate = bytes.clone();
    duplicate.extend(encode_query(item));
    assert!(decode_reply(&duplicate).is_err());
    let mut invalid = reply.clone();
    invalid.quotes[0].materials[0].quantity = 0;
    assert!(encode_reply(&invalid).is_err());
    invalid = reply.clone();
    invalid.quotes[0].materials = vec![invalid.quotes[0].materials[0].clone(); 4];
    assert!(encode_reply(&invalid).is_err());
    invalid = reply.clone();
    invalid.quotes[0].success_percent = 99;
    assert!(encode_reply(&invalid).is_err());
    for end in 0..bytes.len() {
        if let Ok(partial) = decode_reply(&bytes[..end]) {
            assert!(partial.quotes.is_empty());
        }
    }
    let mut unavailable = reply.clone();
    unavailable.quotes[0].family = 2;
    unavailable.quotes[0].recipe_available = false;
    unavailable.quotes[0].materials.clear();
    assert_eq!(decode_reply(&encode_reply(&unavailable)?)?, unavailable);
    assert!(decode_reply(&[0; MAX_REPLY_BYTES + 1]).is_err());
    Ok(())
}
