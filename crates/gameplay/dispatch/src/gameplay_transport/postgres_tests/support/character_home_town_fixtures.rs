
pub(super) fn towns() -> oteryn_world_bundle::subarea::Table {
    use oteryn_world_bundle::subarea::{Area, Mask, Run, Table, Temple};
    let hash = "a".repeat(64);
    let area = |id, town, x| Area {
        id,
        key: format!("oteryn:content.area.fixture{id}"),
        name: format!("Town {id}"),
        parent: (id != 1).then_some(1),
        source_record_sha256: hash.clone(),
        town_id: town,
        hometown: town.is_some(),
        temple: town.map(|_| Temple {
            x,
            y: 11,
            floor: -7,
        }),
    };
    Table {
        default_town: Some(8),
        client_version: "15.30".into(),
        candidate_sha256: hash.clone(),
        // Town 11 (Edron) is the P20 premium-area home town; 8 and 6 are free-account towns.
        areas: vec![
            area(1, None, 0),
            area(2, Some(8), 2),
            area(3, Some(6), 8),
            area(4, Some(11), 14),
        ],
        masks: vec![Mask {
            subarea: 2,
            asset: format!("content/assets/files/subarea-0002-{hash}.bmp.lzma"),
            compressed_sha256: hash.clone(),
            bmp_sha256: hash.clone(),
            source_record_sha256: hash,
            source_tiles: 1,
            admitted_tiles: 1,
        }],
        runs: vec![Run {
            floor: -7,
            y: 11,
            begin: 2,
            end: 3,
            subarea: 2,
        }],
    }
}
