use super::*;
#[test]
fn profile_wire_roundtrip_and_rejects_noncanonical_data() -> Result<(), Error> {
    for target in [None, Some([2; 16])] {
        assert_eq!(decode_request(&request(target))?, target);
    }
    for bytes in [vec![], vec![SELECTOR, 1], vec![0], vec![SELECTOR; 18]] {
        assert!(decode_request(&bytes).is_err());
    }
    let p = Profile {
        character: [2; 16],
        name: "Other Hero".into(),
        badges: vec![Badge::FledgelingHero],
        equipment: vec![EquippedItem {
            slot: 1,
            definition: "oteryn:item/hat".into(),
            quantity: 1,
        }],
    };
    let bytes = p.encode()?;
    assert_eq!(Profile::decode(&bytes)?, p);
    for n in 0..bytes.len() {
        assert!(Profile::decode(&bytes[..n]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(Profile::decode(&trailing).is_err());
    let mut invalid = p.clone();
    invalid.badges.push(Badge::FledgelingHero);
    assert!(invalid.encode().is_err());
    let mut invalid = p.clone();
    invalid.equipment.push(p.equipment[0].clone());
    assert!(invalid.encode().is_err());
    let mut invalid = p;
    invalid.equipment[0].quantity = 0;
    assert!(invalid.encode().is_err());
    Ok(())
}
