use super::picker::*;
#[test]
fn picker_codec_bounds_and_canonical_selection() -> Result<(), &'static str> {
    let claim = Claim {
        epoch: 4,
        day: 1,
        picks: vec![
            Pick {
                item: 266,
                count: 3,
            },
            Pick {
                item: 268,
                count: 2,
            },
        ],
    };
    assert_eq!(Claim::decode(&claim.encode()?)?, claim);
    let mut bad = claim.clone();
    bad.picks[1].item = 266;
    assert!(bad.encode().is_err());
    bad = claim.clone();
    bad.picks[0].count = 0;
    assert!(bad.encode().is_err());
    let mut bytes = claim.encode()?;
    bytes.push(0);
    assert!(Claim::decode(&bytes).is_err());
    let offer = Offer {
        epoch: 4,
        day: 1,
        premium: false,
        maximum: 5,
        items: [0; 32],
    };
    assert_eq!(Offer::decode(&offer.encode()?)?, offer);
    Ok(())
}
