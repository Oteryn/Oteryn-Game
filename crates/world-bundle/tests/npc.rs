//! The NPC family's limits at their maximum and one more (`NPCPLACE1-RL-01`, `-02`,
//! `NPCBEH0-RL-01`), its round trip and its key and order checks (format document §14).

use oteryn_world_bundle::Error;
use oteryn_world_bundle::bundle::Extent;
use oteryn_world_bundle::npc::{self, Direction, Npc, Placement, Table};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn world() -> Extent {
    Extent {
        min_x: 0,
        min_y: 0,
        max_x: 4_096,
        max_y: 4_096,
        floors: vec![6, 7],
    }
}

fn placement(i: usize) -> Placement {
    Placement {
        floor: 7,
        x: (i % 1_024) as u16,
        y: (i / 1_024) as u16,
        direction: Direction::South,
    }
}

/// `placements` placements spread over NPCs of at most `per_npc` each, keys ascending.
fn table(placements: usize, per_npc: usize) -> Table {
    let mut npcs = Vec::new();
    let mut next = 0;
    while next < placements {
        let n = per_npc.min(placements - next);
        npcs.push(Npc {
            key: format!("oteryn:npc.n{:05}", npcs.len()),
            placements: (next..next + n).map(placement).collect(),
        });
        next += n;
    }
    Table { npcs }
}

fn round_trip(table: &Table) -> Result<Table, Error> {
    npc::validate(table, &world())?;
    npc::decode(&npc::encode(table), &world())
}

#[test]
fn placements_per_bundle_accept_the_maximum_and_refuse_one_more() -> TestResult {
    let full = table(npc::MAX_PLACEMENTS, npc::MAX_PLACEMENTS_PER_NPC);
    assert_eq!(full.placement_count(), 2_048);
    assert_eq!(round_trip(&full)?, full);
    let over = table(npc::MAX_PLACEMENTS + 1, npc::MAX_PLACEMENTS_PER_NPC);
    assert!(matches!(
        npc::validate(&over, &world()),
        Err(Error::Limit(_))
    ));
    assert!(matches!(
        npc::decode(&npc::encode(&over), &world()),
        Err(Error::Limit(_))
    ));
    // 2,048 NPCs of one placement each hit the same row (the NPC count it bounds).
    let single = table(npc::MAX_PLACEMENTS, 1);
    assert_eq!(single.npcs.len(), 2_048);
    assert_eq!(round_trip(&single)?, single);
    let over = table(npc::MAX_PLACEMENTS + 1, 1);
    assert!(matches!(
        npc::decode(&npc::encode(&over), &world()),
        Err(Error::Limit(_))
    ));
    Ok(())
}

#[test]
fn placements_per_npc_accept_the_maximum_and_refuse_one_more() -> TestResult {
    let full = table(npc::MAX_PLACEMENTS_PER_NPC, npc::MAX_PLACEMENTS_PER_NPC);
    assert_eq!(full.npcs.len(), 1);
    assert_eq!(round_trip(&full)?, full);
    let over = table(
        npc::MAX_PLACEMENTS_PER_NPC + 1,
        npc::MAX_PLACEMENTS_PER_NPC + 1,
    );
    assert_eq!(over.npcs.len(), 1);
    assert!(matches!(
        npc::validate(&over, &world()),
        Err(Error::Limit(_))
    ));
    assert!(matches!(
        npc::decode(&npc::encode(&over), &world()),
        Err(Error::Limit(_))
    ));
    Ok(())
}

#[test]
fn keys_order_cells_and_bounds_are_checked() {
    let format = |table: &Table| matches!(npc::validate(table, &world()), Err(Error::Format(_)));
    let mut bad = table(2, 1);
    bad.npcs.swap(0, 1);
    assert!(format(&bad), "keys not ascending");
    let mut bad = table(1, 1);
    bad.npcs[0].key = "oteryn:creature.rat".into();
    assert!(format(&bad), "not an NPC key");
    let mut bad = table(1, 1);
    bad.npcs[0].key = format!("oteryn:npc.{}", "a".repeat(npc::MAX_KEY_BYTES));
    assert!(format(&bad), "key too long");
    let mut bad = table(2, 2);
    bad.npcs[0].placements.swap(0, 1);
    assert!(format(&bad), "placements not ascending");
    let mut bad = table(2, 1);
    bad.npcs[1].placements[0] = bad.npcs[0].placements[0];
    assert!(format(&bad), "shared cell");
    let mut bad = table(1, 1);
    bad.npcs[0].placements[0].floor = 8;
    assert!(format(&bad), "floor outside the World");
    let mut bad = table(1, 1);
    bad.npcs[0].placements.clear();
    assert!(format(&bad), "NPC without placements");
}

#[test]
fn decode_refuses_trailing_bytes_and_unknown_directions() {
    let one = table(1, 1);
    let mut bytes = npc::encode(&one);
    bytes.push(0);
    assert!(matches!(
        npc::decode(&bytes, &world()),
        Err(Error::Format(_))
    ));
    let mut bytes = npc::encode(&one);
    let last = bytes.len() - 1;
    bytes[last] = 4;
    assert!(matches!(
        npc::decode(&bytes, &world()),
        Err(Error::Format(_))
    ));
    assert!(npc::decode(&[], &world()).is_err());
    assert_eq!(npc::decode(&[0], &world()), Ok(Table::default()));
}

#[test]
fn the_catalogue_digest_is_order_independent_and_length_framed() {
    let a = npc::catalogue_digest(&[
        ("npcs/definitions/a.json", b"x".as_slice()),
        ("services/b", b""),
    ]);
    let b = npc::catalogue_digest(&[
        ("services/b", b"".as_slice()),
        ("npcs/definitions/a.json", b"x"),
    ]);
    assert_eq!(a, b);
    assert!(npc::is_sha256_hex(&a));
    // Moving a byte from the content into the path changes the digest.
    let c = npc::catalogue_digest(&[
        ("npcs/definitions/a.jsonx", b"".as_slice()),
        ("services/b", b""),
    ]);
    assert_ne!(a, c);
    assert_ne!(npc::catalogue_digest::<&str, &[u8]>(&[]), a);
}
