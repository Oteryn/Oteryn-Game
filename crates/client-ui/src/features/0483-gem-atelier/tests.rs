use super::*;
use atelier::v2;

type Error = Box<dyn std::error::Error>;

fn wire<T>(result: Result<T, impl std::fmt::Debug>) -> Result<T, Error> {
    result.map_err(|error| format!("{error:?}").into())
}

fn occurrence() -> [u8; 16] {
    let mut id = [9; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

fn reply() -> v2::AtelierReply {
    let slots = (0..12u8).map(|slot| v2::VesselSlotView {
        quality: slot / 4,
        open: slot != 5,
        gem: (slot == 1).then_some([1; 16]),
    });
    v2::AtelierReply {
        refused: false,
        revision: 4,
        wheel_revision: 8,
        in_temple: true,
        premium: true,
        gems: vec![
            v2::GemView {
                id: [1; 16],
                quality: 0,
                locked: true,
                slot: Some(1),
                modifiers: vec![3],
            },
            v2::GemView {
                id: [2; 16],
                quality: 2,
                locked: false,
                slot: None,
                modifiers: vec![],
            },
        ],
        grades: vec![],
        quality_quotes: vec![v2::QualityQuote {
            quality: 1,
            reveal_gold: 500,
            fragment_kind: 0,
            fragment_min: 2,
            fragment_max: 4,
        }],
        grade_quotes: vec![],
        vessels: slots.collect(),
        vessel_nodes: vec![3, 9],
        vessel_node_bonus: 1,
        resistance_capped: true,
    }
}

#[test]
fn a_v2_reply_decodes_into_gems_without_domains_and_the_vessel_board() -> Result<(), Error> {
    let bytes = wire(v2::encode_reply(&reply()))?;
    let projection = decode_reply(&bytes).ok_or("decode")?;
    assert_eq!(projection.atelier.revision, 4);
    assert_eq!(projection.atelier.gems.len(), 2);
    let socketed = &projection.atelier.gems[0];
    assert!(socketed.vessel && socketed.domain == 1 && socketed.locked);
    assert!(!projection.atelier.gems[1].vessel);
    assert_eq!(projection.atelier.quality_quotes[0].reveal_gold, 500);
    let board = &projection.board;
    assert_eq!(board.slots.len(), 12);
    assert_eq!(board.slots[1].gem, Some([1; 16]));
    assert_eq!(
        (board.wheel_revision, board.in_temple, board.premium),
        (8, true, true)
    );
    assert_eq!((board.node_bonus, board.nodes.clone()), (1, vec![3, 9]));
    assert!(board.resistance_capped);
    Ok(())
}

#[test]
fn replace_vessel_only_allows_an_open_slot_of_the_gems_own_quality() -> Result<(), Error> {
    let projection = project(&reply());
    let greater = &projection.atelier.gems[1];
    let board = &projection.board;
    // Slots 0-3 are Lesser, 4-7 Regular, 8-11 Greater; slot 5 is locked.
    assert_eq!(replace_block(board, greater, 8), None);
    assert_eq!(
        replace_block(board, greater, 0),
        Some(ReplaceBlock::QualityMismatch)
    );
    assert_eq!(
        replace_block(board, greater, 12),
        Some(ReplaceBlock::NoVessels)
    );
    let mut regular = greater.clone();
    regular.quality = 1;
    assert_eq!(
        replace_block(board, &regular, 5),
        Some(ReplaceBlock::SlotLocked)
    );
    let lesser = &projection.atelier.gems[0];
    assert_eq!(
        replace_block(board, lesser, 1),
        Some(ReplaceBlock::AlreadyInSlot)
    );
    Ok(())
}

#[test]
fn commands_use_the_v2_numbers_with_the_documented_bytes() -> Result<(), Error> {
    let query = query().ok_or("query")?;
    assert_eq!(
        (query.kind, query.payload.is_empty(), query.mutation),
        (6210, true, false)
    );

    let lock = set_lock(occurrence(), 4, [2; 16], true)?;
    assert_eq!((lock.kind, lock.mutation), (6211, true));
    let decoded = wire(v2::decode_set_lock(&lock.payload))?;
    assert_eq!(
        (decoded.expected_revision, decoded.gem, decoded.locked),
        (4, [2; 16], true)
    );

    let reveal = mutate(occurrence(), 4, &atelier::Mutation::Reveal { quality: 1 })?;
    assert_eq!((reveal.kind, reveal.mutation), (6212, true));
    assert_eq!(
        wire(v2::decode_mutate(&reveal.payload))?.mutation,
        v2::Mutation::Reveal { quality: 1 }
    );
    assert!(
        mutate(
            occurrence(),
            4,
            &atelier::Mutation::SwitchDomain { gem: [2; 16] }
        )
        .is_err()
    );

    let place = replace_vessel(occurrence(), 8, 9, Some([2; 16]))?;
    assert_eq!((place.kind, place.mutation), (6213, true));
    let decoded = wire(v2::decode_replace_vessel(&place.payload))?;
    assert_eq!(
        (decoded.expected_wheel_revision, decoded.slot, decoded.gem),
        (8, 9, Some([2; 16]))
    );
    let empty = replace_vessel(occurrence(), 8, 9, None)?;
    assert_eq!(wire(v2::decode_replace_vessel(&empty.payload))?.gem, None);
    assert!(replace_vessel(occurrence(), 8, 12, None).is_err());
    Ok(())
}

#[test]
fn vessel_lines_list_slots_the_node_bonus_and_the_resistance_cap() {
    let board = project(&reply()).board;
    let lines = vessel_lines(&board, |_| "gem".to_owned());
    assert_eq!(lines.len(), 12 + 2);
    assert_eq!(lines[0], "Gem Vessel 1 (Lesser): Empty");
    assert_eq!(lines[1], "Gem Vessel 2 (Lesser): gem");
    assert_eq!(lines[5], "Gem Vessel 6 (Regular): locked");
    assert!(lines[12].contains("Wheel slots 3, 9") && lines[12].contains("+1 Damage / Healing"));
    assert!(lines[13].contains("capped at 30%"));
    let mut plain = board;
    plain.resistance_capped = false;
    assert_eq!(vessel_lines(&plain, |_| String::new()).len(), 13);
}
