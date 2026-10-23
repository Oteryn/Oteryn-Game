#![allow(clippy::expect_used)]
use super::*;

#[test]
fn blessing_owner_prices_cover_every_boundary_and_promotion_has_no_discount() {
    let catalogue = BlessingCatalogue::load().expect("admitted data");
    for blessing in catalogue.blessings {
        let cases = match blessing.price_family {
            PriceFamily::Regular => vec![
                (30, 2_000),
                (31, 2_200),
                (119, 19_800),
                (120, 20_000),
                (121, 20_075),
            ],
            PriceFamily::Enhanced => vec![
                (30, 2_600),
                (31, 2_860),
                (119, 25_740),
                (120, 26_000),
                (121, 26_100),
            ],
            PriceFamily::Twist => vec![
                (30, 2_000),
                (31, 2_200),
                (269, 49_800),
                (270, 50_000),
                (271, 50_000),
            ],
        };
        for (level, price) in cases {
            assert_eq!(blessing.price(level), price, "{} {level}", blessing.name);
        }
        assert!(blessing.price(u32::MAX) <= 1_000_000_000_000);
    }
}

#[test]
fn blessing_unknown_conflict_and_undated_owner_evidence_refuse() {
    let source = include_str!("../../../../../rulesets/character/blessings/catalogue.json");
    for invalid in [
        source.replace("PROVEN_BY_OWNER", "UNKNOWN"),
        source.replace("PROVEN_BY_OWNER", "CONFLICT"),
        source.replace("2026-10-18", ""),
        source.replace("\"none\"", "\"discount\""),
    ] {
        assert!(BlessingCatalogue::parse(&invalid).is_err());
    }
    for admitted in ["PROVEN", "DERIVED"] {
        assert!(BlessingCatalogue::parse(&source.replace("PROVEN_BY_OWNER", admitted)).is_ok());
    }
}
