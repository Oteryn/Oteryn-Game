fn current(revision: u64) -> (wheel::WheelAllocationReply, atelier::AtelierReply) {
    (
        wheel::WheelAllocationReply {
            refused: false,
            revision,
            points: [0; 36],
            vessels: None,
        },
        atelier::AtelierReply {
            refused: false,
            revision,
            gems: vec![atelier::GemView {
                id: [17; 16],
                quality: 2,
                domain: 0,
                locked: false,
                vessel: false,
                modifiers: vec![31, 0, 5],
            }],
            grades: vec![atelier::GradeView {
                kind: 1,
                mod_id: 5,
                grade: 1,
            }],
            quality_quotes: (0..3)
                .map(|quality| atelier::QualityQuote {
                    quality,
                    reveal_gold: 123_456 + u64::from(quality),
                    switch_gold: 654_321,
                    fragment_kind: u8::from(quality == 2),
                    fragment_min: 9,
                    fragment_max: 11,
                })
                .collect(),
            grade_quotes: (0..2)
                .flat_map(|kind| {
                    (1..4).map(move |grade| atelier::GradeQuote {
                        kind,
                        grade,
                        gold: 222_222,
                        fragments: 777,
                    })
                })
                .collect(),
        },
    )
}
