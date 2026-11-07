//! WSP1: legacy bytes stay unchanged; weighted references fail closed and round-trip.
use oteryn_world_bundle::{
    bundle::Extent,
    spawn::{
        self, Creature, Direction, Period, Point, Source, Table, WeightedChoice, WeightedPoint,
    },
};
fn fixture() -> (Table, Extent) {
    let table = Table {
        creatures: vec![
            Creature {
                key: "oteryn:creature.silverfrost_sentinel".into(),
                period: Period::All,
            },
            Creature {
                key: "oteryn:creature.true_frost_flower_asura".into(),
                period: Period::All,
            },
        ],
        sources: vec![Source {
            key: "oteryn:spawn.x33866_y32698_z9".into(),
            floor: -9,
            x: 33866,
            y: 32698,
            points: vec![Point {
                creature: 1,
                x: 33866,
                y: 32698,
                direction: Direction::North,
                respawn_ms: 47_104_000,
                boss_policy: None,
            }],
        }],
        weighted: Vec::new(),
    };
    (
        table,
        Extent {
            min_x: 33860,
            min_y: 32690,
            max_x: 33870,
            max_y: 32700,
            floors: vec![-9],
        },
    )
}
#[test]
fn srv7_weighted_spawn_trailer_roundtrip_and_old_ordinary_bytes()
-> Result<(), oteryn_world_bundle::Error> {
    let (mut table, world) = fixture();
    let ordinary = spawn::encode(&table);
    assert_eq!(spawn::decode(&ordinary, &world)?, table);
    table.weighted = vec![WeightedPoint {
        source: 0,
        point: 0,
        choices: vec![
            WeightedChoice {
                creature: 0,
                weight: 25,
            },
            WeightedChoice {
                creature: 1,
                weight: 25,
            },
        ],
    }];
    spawn::validate(&table, &world)?;
    let encoded = spawn::encode(&table);
    assert_eq!(
        &encoded[..ordinary.len()],
        ordinary.as_slice(),
        "all existing scalar point bytes unchanged"
    );
    assert_eq!(
        &encoded[ordinary.len()..],
        b"WSP1\x01\x00\x00\x02\x00\x19\x01\x19"
    );
    assert_eq!(spawn::decode(&encoded, &world)?, table);
    for end in ordinary.len() + 1..encoded.len() {
        assert!(
            spawn::decode(&encoded[..end], &world).is_err(),
            "truncated trailer at{end}"
        );
    }
    let mut junk = encoded.clone();
    junk.push(0);
    assert!(spawn::decode(&junk, &world).is_err());
    Ok(())
}
#[test]
fn srv7_weighted_spawn_rejects_zero_overflow_duplicate_and_invalid_references() {
    let (mut table, world) = fixture();
    table.weighted = vec![WeightedPoint {
        source: 0,
        point: 0,
        choices: vec![
            WeightedChoice {
                creature: 0,
                weight: 25,
            },
            WeightedChoice {
                creature: 1,
                weight: 25,
            },
        ],
    }];
    for case in 0..7 {
        let mut bad = table.clone();
        match case {
            0 => bad.weighted[0].choices[0].weight = 0,
            1 => bad.weighted[0].choices[0].weight = u32::MAX,
            2 => bad.weighted[0].choices[1].creature = 0,
            3 => bad.weighted[0].source = 1,
            4 => bad.weighted[0].point = 1,
            5 => bad.creatures[0].period = Period::Night,
            _ => bad.weighted.push(bad.weighted[0].clone()),
        }
        assert!(spawn::validate(&bad, &world).is_err(), "case{case}");
        assert!(
            spawn::decode(&spawn::encode(&bad), &world).is_err(),
            "decode case{case}"
        );
    }
}

#[test]
fn srv7_weighted_spawn_choice_limit_is_checked_before_decode_allocation()
-> Result<(), oteryn_world_bundle::Error> {
    let (mut table, world) = fixture();
    table.creatures = (0..64)
        .map(|n| Creature {
            key: format!("oteryn:creature.fixture{n:02}"),
            period: Period::All,
        })
        .collect();
    table.sources[0].points[0].creature = 63;
    table.weighted = vec![WeightedPoint {
        source: 0,
        point: 0,
        choices: (0..64)
            .map(|creature| WeightedChoice {
                creature,
                weight: 1,
            })
            .collect(),
    }];
    spawn::validate(&table, &world)?;
    assert_eq!(spawn::decode(&spawn::encode(&table), &world)?, table);
    table.creatures.push(Creature {
        key: "oteryn:creature.fixture64".into(),
        period: Period::All,
    });
    table.weighted[0].choices.push(WeightedChoice {
        creature: 64,
        weight: 1,
    });
    assert!(spawn::validate(&table, &world).is_err());
    assert!(spawn::decode(&spawn::encode(&table), &world).is_err());
    Ok(())
}
