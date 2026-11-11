"""Register the reviewed Corruptor combat-only fixture in the room census."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WRITER = 'gameplay_transport/postgres_tests/newhaven_corruptor.rs'
SOURCE = ROOT / 'crates/gameplay/dispatch/src' / WRITER
TARGET = ROOT / 'crates/gameplay/dispatch/src/gameplay_transport/bundle_gates_tests.rs'


def regenerate():
    source = SOURCE.read_text()
    if (source.count('SeamRooms') != 1 or not re.search(
            r'SeamRooms::new\(\s*None,\s*Some\(&room\)\s*\)', source)):
        raise ValueError('CORRUPTOR_FIXTURE_MUST_HAVE_NO_QUALIFIED_ROOM')
    text = TARGET.read_text()
    start = text.index('const CONSTRUCTIONS:')
    stop = text.index('#[test]', start)
    block = text[start:stop]
    entry = '    ("' + WRITER + '", 1),\n'
    if entry not in block:
        anchor = '    ("gameplay_transport/postgres_tests/magic2_conjure.rs", 2),\n'
        if block.count(anchor) != 1:
            raise ValueError('ROOM_CENSUS_ANCHOR')
        block = block.replace(anchor, anchor + entry)
        TARGET.write_text(text[:start] + block + text[stop:])
    print('PASS reviewed combat-only Corruptor room construction')


if __name__ == '__main__':
    regenerate()
