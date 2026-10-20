#![allow(clippy::expect_used)]

use crate::durability::highscore_snapshot::ranking::*;
use crate::foundation::CharacterId;

/// One filter's list, as the cut takes it from [`ranked_lists`].
fn ranked_list<'a, Value: Ord>(
    rows: &'a [RankingRow<'a, Value>],
    vocation_filter: Option<&str>,
) -> Result<Vec<RankedRow<'a, Value>>, RankingError> {
    Ok(ranked_lists(rows, &[vocation_filter])?
        .into_iter()
        .next()
        .unwrap_or_default())
}

fn character(number: u32) -> CharacterId {
    let mut bytes = [0; 16];
    bytes[..4].copy_from_slice(&number.to_be_bytes());
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    CharacterId::decode(&bytes).expect("valid deterministic native CharacterId")
}

fn row<'a>(number: u32, name: &'a str, vocation: &'a str, value: u64) -> RankingRow<'a, u64> {
    RankingRow {
        character_id: character(number),
        name,
        vocation,
        value,
    }
}

#[test]
fn competition_ranks_skip_positions_without_using_names_as_values() {
    let rows = [
        row(4, "Fourth", "druid", 7),
        row(3, "Zed", "knight", 9),
        row(1, "First", "druid", 10),
        row(2, "Alpha", "knight", 9),
    ];
    let ranked = ranked_list(&rows, None).expect("eligible unique rows");
    assert_eq!(
        ranked.iter().map(|r| r.global_rank).collect::<Vec<_>>(),
        [1, 2, 2, 4]
    );
    assert_eq!(
        ranked
            .iter()
            .map(|r| r.row.character_id)
            .collect::<Vec<_>>(),
        [character(1), character(2), character(3), character(4)]
    );
}

#[test]
fn ties_use_raw_unicode_code_points_without_case_folding_or_normalization() {
    let rows = [
        row(1, "\u{10000}", "monk", 1),
        row(2, "Å", "monk", 1),
        row(3, "a", "monk", 1),
        row(4, "Z", "monk", 1),
        row(5, "A\u{30a}", "monk", 1),
        row(6, "\u{e000}", "monk", 1),
        row(7, "Ä", "monk", 1),
    ];
    let ranked = ranked_list(&rows, None).expect("eligible unique rows");
    assert_eq!(
        ranked.iter().map(|r| r.row.name).collect::<Vec<_>>(),
        ["A\u{30a}", "Z", "a", "Ä", "Å", "\u{e000}", "\u{10000}"]
    );
    assert!(ranked.iter().all(|r| r.global_rank == 1));
}

#[test]
fn equal_name_and_value_use_native_character_id_independent_of_input_order() {
    for order in [[3, 1, 2], [2, 3, 1], [1, 2, 3]] {
        let rows = order.map(|id| row(id, "Same", "paladin", 5));
        let ranked = ranked_list(&rows, None).expect("eligible unique rows");
        assert_eq!(
            ranked
                .iter()
                .map(|r| r.row.character_id)
                .collect::<Vec<_>>(),
            [character(1), character(2), character(3)]
        );
    }
}

#[test]
fn full_level_progress_tuple_defines_value_and_shared_rank() {
    let rows = [
        (1, "A", (10_u16, 9_u64)),
        (2, "B", (11, 0)),
        (3, "C", (10, 10)),
        (4, "D", (10, 10)),
    ]
    .map(|(id, name, value)| RankingRow {
        character_id: character(id),
        name,
        vocation: "druid",
        value,
    });
    let ranked = ranked_list(&rows, None).expect("eligible unique rows");
    assert_eq!(
        ranked
            .iter()
            .map(|r| (r.row.character_id, r.global_rank))
            .collect::<Vec<_>>(),
        [
            (character(2), 1),
            (character(3), 2),
            (character(4), 2),
            (character(1), 4)
        ]
    );
}

#[test]
fn vocation_filter_keeps_global_ranks_including_cross_vocation_ties() {
    let rows = [
        row(1, "First", "druid", 10),
        row(2, "Alpha", "knight", 9),
        row(3, "Zed", "elder_druid", 9),
        row(4, "Last", "elite_knight", 7),
    ];
    let ranked = ranked_list(&rows, Some("knight")).expect("base vocation filter");
    assert_eq!(
        ranked.iter().map(|r| r.global_rank).collect::<Vec<_>>(),
        [2, 4]
    );
}

#[test]
fn all_five_native_promotion_families_count_as_their_base() {
    let pairs = [
        ("knight", "elite_knight"),
        ("paladin", "royal_paladin"),
        ("sorcerer", "master_sorcerer"),
        ("druid", "elder_druid"),
        ("monk", "exalted_monk"),
    ];
    for (base, promoted) in pairs {
        let rows = [row(1, "Base", base, 1), row(2, "Promoted", promoted, 1)];
        let ranked = ranked_list(&rows, Some(base)).expect("base vocation filter");
        assert_eq!(ranked.len(), 2);
        assert!(ranked.iter().all(|r| r.global_rank == 1));
    }
}

#[test]
fn filtered_list_is_not_derived_from_the_capped_global_list() {
    let rows: Vec<_> = (0..2_200)
        .map(|n| {
            row(
                n,
                "Same",
                if n < 1_100 { "druid" } else { "exalted_monk" },
                u64::from(2_200 - n),
            )
        })
        .collect();
    let all = ranked_list(&rows, None).expect("eligible unique rows");
    assert_eq!(all.len(), 1_000);
    assert!(all.iter().all(|r| r.row.vocation == "druid"));
    let monks = ranked_list(&rows, Some("monk")).expect("base vocation filter");
    assert_eq!(monks.len(), 1_000);
    assert_eq!(monks.first().map(|r| r.global_rank), Some(1_101));
    assert_eq!(monks.last().map(|r| r.global_rank), Some(2_100));
}

#[test]
fn cap_is_one_thousand_even_when_every_value_ties_at_the_boundary() {
    let rows: Vec<_> = (0..1_002)
        .rev()
        .map(|n| row(n, "Same", "knight", 1))
        .collect();
    let ranked = ranked_list(&rows, Some("knight")).expect("base vocation filter");
    assert_eq!(ranked.len(), RANKING_ROWS_MAX);
    assert!(ranked.iter().all(|r| r.global_rank == 1));
    assert_eq!(
        ranked.first().map(|r| r.row.character_id),
        Some(character(0))
    );
    assert_eq!(
        ranked.last().map(|r| r.row.character_id),
        Some(character(999))
    );
}

#[test]
fn empty_lists_and_none_vocation_do_not_invent_category_eligibility() {
    assert!(ranked_list::<u64>(&[], None).expect("empty cut").is_empty());
    let rows = [row(1, "Representative", "none", 100)];
    // Account-category eligibility is the caller's job; none is not globally discarded here.
    assert_eq!(ranked_list(&rows, None).expect("eligible cut").len(), 1);
    assert!(
        ranked_list(&rows, Some("monk"))
            .expect("base filter")
            .is_empty()
    );
}

#[test]
fn unknown_or_promoted_filter_is_refused_and_duplicate_sources_are_not_ranked() {
    let rows = [row(1, "Name", "knight", 10)];
    for filter in ["none", "elite_knight", "KNIGHT", "", "future"] {
        assert!(matches!(
            ranked_list(&rows, Some(filter)),
            Err(RankingError::InvalidVocationFilter)
        ));
    }
    let duplicate = [row(1, "First", "knight", 10), row(1, "Other", "druid", 20)];
    assert!(matches!(
        ranked_list(&duplicate, None),
        Err(RankingError::DuplicateCharacter)
    ));
}

#[test]
fn scalar_extremes_sort_without_subtraction_or_score_arithmetic() {
    let rows = [
        row(1, "Zero", "monk", 0),
        row(2, "Maximum", "monk", u64::MAX),
    ];
    let ranked = ranked_list(&rows, None).expect("eligible unique rows");
    assert_eq!(ranked[0].row.value, u64::MAX);
    assert_eq!(ranked[1].row.value, 0);
}

#[test]
fn one_sort_yields_each_filters_exact_ranked_list() {
    const VOCATIONS: [&str; 7] = [
        "knight",
        "elite_knight",
        "paladin",
        "sorcerer",
        "master_sorcerer",
        "druid",
        "monk",
    ];
    let names: Vec<String> = (0..2_600_u32).map(|n| format!("Name {}", n % 97)).collect();
    // Values tie across vocations and names, and every list runs past the 1,000 cap.
    let rows: Vec<_> = names
        .iter()
        .zip(0_u32..)
        .map(|(name, n)| {
            row(
                n + 1,
                name,
                VOCATIONS[(n as usize * 5) % VOCATIONS.len()],
                u64::from(n % 41),
            )
        })
        .collect();
    let filters = [
        None,
        Some("knight"),
        Some("paladin"),
        Some("sorcerer"),
        Some("druid"),
        Some("monk"),
    ];
    let lists = ranked_lists(&rows, &filters).expect("eligible unique rows");
    assert_eq!(lists.len(), filters.len());
    for (filter, list) in filters.iter().zip(&lists) {
        let single = ranked_list(&rows, *filter).expect("eligible unique rows");
        assert_eq!(
            list.iter()
                .map(|r| (r.global_rank, r.row.character_id))
                .collect::<Vec<_>>(),
            single
                .iter()
                .map(|r| (r.global_rank, r.row.character_id))
                .collect::<Vec<_>>(),
            "{filter:?}"
        );
    }
    assert_eq!(lists[0].len(), RANKING_ROWS_MAX);
    assert!(lists.iter().all(|list| !list.is_empty()));
    assert!(matches!(
        ranked_lists(&rows, &[None, Some("elite_knight")]),
        Err(RankingError::InvalidVocationFilter)
    ));
    assert!(ranked_lists(&rows, &[]).expect("no filter").is_empty());
}
