use super::*;
#[test]
fn gem_presentation_all_canonical_vocations_fit_and_use_executed_corrections()
-> Result<(), &'static str> {
    let data = WheelGemData::embedded().map_err(|_| "catalogue")?;
    let mut mastery_ids = std::collections::BTreeSet::new();
    for (vocation, modifier_count) in [
        ("druid", 73),
        ("knight", 71),
        ("monk", 73),
        ("paladin", 73),
        ("sorcerer", 73),
    ] {
        let rows = project(&data, vocation).ok_or("project")?;
        assert_eq!(rows.len(), modifier_count, "{vocation}");
        let reply = wire::Reply {
            grades_current: true,
            qualified_gems: (1_u128..=225).map(u128::to_be_bytes).collect(),
            request: wire::Request {
                nonce: 1,
                revision: u64::MAX,
            },
            catalogue: format!("{}@{}", data.revision(), data.source_candidate_sha256()),
            vocation: vocation.into(),
            modifiers: rows.clone(),
        };
        let bytes = reply.encode().ok_or("all real entries must fit")?;
        assert!(bytes.len() <= wire::MAX_REPLY_BYTES);
        assert_eq!(wire::Reply::decode(&bytes), Some(reply));
        if vocation == "knight" {
            let cleansing = rows
                .iter()
                .find(|row| row.kind == 1 && row.id == 97)
                .ok_or("cleansing")?;
            let momentum = cleansing
                .effects
                .iter()
                .find(|effect| effect.name == "momentum_chance")
                .ok_or("Momentum")?;
            assert_eq!(momentum.values, [0, 330, 660, 1000]);
        }
        if vocation == "monk" {
            let capacity = rows
                .iter()
                .find(|r| r.kind == 0 && r.id == 48)
                .ok_or("capacity")?;
            assert_eq!(capacity.effects[0].values[3], 750000);
            for id in 33..=36 {
                let mana = rows
                    .iter()
                    .find(|r| r.kind == 0 && r.id == id)
                    .ok_or("mixed mana")?;
                assert_eq!(
                    mana.effects
                        .iter()
                        .find(|e| e.name == "Mana")
                        .ok_or("mana")?
                        .values,
                    [100_000, 110_000, 120_000, 150_000],
                    "Monk mixed Mana mod {id}"
                );
            }
        }
        let dodge = rows
            .iter()
            .find(|r| r.kind == 1 && r.id == 0)
            .ok_or("dodge")?;
        assert_eq!(dodge.effects[0].values, [280, 300, 330, 420]);
        let mastery = rows
            .iter()
            .filter(|r| {
                r.kind == 1
                    && r.effects
                        .iter()
                        .any(|e| e.name == "revelation_mastery_points")
            })
            .collect::<Vec<_>>();
        assert!(!mastery.is_empty(), "{vocation} has Mastery rows");
        for row in &mastery {
            for effect in row
                .effects
                .iter()
                .filter(|e| e.name == "revelation_mastery_points")
            {
                assert_eq!(
                    effect.values,
                    [150_000, 165_000, 180_000, 225_000],
                    "{vocation} Mastery mod {}",
                    row.id
                );
            }
            mastery_ids.insert(row.id);
        }
        if vocation == "monk" {
            assert!(mastery.iter().any(|r| r.id == 92), "Monk Mastery mod 92");
        }
    }
    assert_eq!(
        mastery_ids,
        [
            5, 21, 22, 23, 39, 40, 41, 56, 57, 58, 73, 74, 75, 91, 92, 93
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
    );
    assert!(project(&data, "none").is_none());
    Ok(())
}
