"""Embed the owned source guard leaves in the unmerged SRV7 migration."""
import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MIGRATION = ROOT / "apps/game-server/migrations/0993_seasonal_events.sql"
LEAVES = (
    ("npc-source-quest-routing.sql", "SRV7 SOURCE QUEST ROUTING", True),
    ("summer-instance-corpse-mint.sql", "SRV7 RAKESH SOURCE summer-instance-corpse-mint.sql", False),
)


def replace_leaf(before, name, marker, may_append):
    source = (ROOT / "tools/content-schema/native-gameplay" / name).read_text()
    if not source.endswith("\n"):
        raise ValueError("source Quest routing leaf must end with a newline")
    begin = "-- BEGIN " + marker + "\n"
    end_marker = "-- END " + marker + "\n"
    block = begin + source + end_marker
    if may_append and begin not in before and end_marker not in before:
        return before.rstrip("\n") + "\n\n" + block
    if before.count(begin) != 1 or before.count(end_marker) != 1:
        raise ValueError("source Quest routing migration marker drift")
    start = before.index(begin)
    end = before.index(end_marker, start) + len(end_marker)
    return before[:start] + block + before[end:]


def output():
    generated = MIGRATION.read_text()
    for name, marker, may_append in LEAVES:
        generated = replace_leaf(generated, name, marker, may_append)
    return generated


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = output()
    if args.check:
        if MIGRATION.read_text() != generated:
            raise ValueError("source Quest routing migration drift")
    else:
        MIGRATION.write_text(generated)
    print("PASS exact SRV7 source guard leaves; unrelated migration bytes preserved")


if __name__ == "__main__":
    main()
