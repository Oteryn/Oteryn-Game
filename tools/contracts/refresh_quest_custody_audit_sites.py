"""Register only the two reviewed QBASE writers; retain all tuple checks."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'crates/gameplay/durability/src/durability/item_mint_audit.rs'
REVIEWED = (
    ('quest_state/npc_program/fathers_burden.rs', 'reward_claim_mint/boss_reward/delivery.rs', False),
    ('tutorial_viola.rs', 'reward_claim_mint.rs', True),
)
PREFIX = 'crates/gameplay/durability/src/durability/'


def regenerate():
    text = SOURCE.read_text()
    start = text.index('fn every_type2_insert_takes_its_tuple_from_a_type2_transaction()')
    stop = text.index('#[test]', start)
    block = text[start:stop]
    for relative, anchor, after in REVIEWED:
        writer = (ROOT / (PREFIX + relative)).read_text()
        if ('INSERT INTO game_item_audit_outbox(' not in writer
                or 'tuple.schema_revision()' not in writer
                or 'tuple.retention_profile_id()' not in writer):
            raise ValueError('REVIEWED_TYPE2_WRITER_PROOF:' + relative)
        new = '                "' + PREFIX + relative + '",\n'
        if new in block:
            continue
        old = '                "' + PREFIX + anchor + '",\n'
        if block.count(old) != 1:
            raise ValueError('TYPE2_INVENTORY_ANCHOR:' + anchor)
        block = block.replace(old, old + new if after else new + old)
    updated = text[:start] + block + text[stop:]
    if updated != text:
        SOURCE.write_text(updated)
    print('PASS reviewed QBASE Type2 writer inventory')


if __name__ == '__main__':
    regenerate()
