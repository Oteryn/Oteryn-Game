#![allow(clippy::expect_used)]
use super::*;

#[test]
fn henricus_quotes_the_sum_for_only_missing_regular_blessings() {
    let catalogue = BlessingCatalogue::load().expect("catalogue");
    let held = vec!["oteryn:blessing.the_spiritual_shielding".to_owned()];
    let (price, missing) = catalogue
        .henricus_quote(30, &held)
        .expect("missing blessings");
    assert_eq!(missing.len(), 4);
    assert_eq!(price, 8_000);
    assert!(
        missing
            .iter()
            .all(|blessing| blessing.price_family == PriceFamily::Regular)
    );

    let all = catalogue
        .blessings
        .iter()
        .filter(|blessing| blessing.price_family == PriceFamily::Regular)
        .map(|blessing| blessing.storage_key.clone())
        .collect::<Vec<_>>();
    assert_eq!(catalogue.henricus_quote(30, &all), None);
}

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
