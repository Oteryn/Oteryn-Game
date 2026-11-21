"""Source-bound Nomads Land cabinet and its literal readable reward."""
from __future__ import annotations

import argparse
import hashlib
import json
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REVISION = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
SOURCE_PATH = "data-global/startup/tables/container_misc.lua"
SOURCE_SHA256 = "da146858a7bfbef47f601e02fc2b621c5e639ec2d096a6831446800277129bca"
SOURCE_URL = f"https://raw.githubusercontent.com/zimbadev/crystalserver/{REVISION}/{SOURCE_PATH}#L803-L822"
SOURCE_REF = f"crystalserver:{REVISION}:{SOURCE_PATH}:803-822:sha256:{SOURCE_SHA256}"
OUTPUT = "content/quests/world-callbacks/questpc5-nomads-cabinet.json"
KEY = "oteryn:quest-callback/nomads-land-sacred-casket-cabinet"
CLAIMED = "oteryn:quest-progress/quest/u8_1/tibia_tales/nomads_land_cabinet"
ARITO = "oteryn:quest-progress/quest/u8_1/tibia_tales/aritos_task"
def retained_source() -> bytes:
    archive = ROOT / "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
    with tarfile.open(archive) as corpus:
        manifest = json.load(corpus.extractfile("corpus-manifest.json"))
        entry = next(row for row in manifest["files"]
                     if row["source"] == "crystalserver" and row["revision"] == REVISION
                     and row["path"] == SOURCE_PATH)
        raw = corpus.extractfile(entry.get("blob_path", entry.get("cache_path"))).read()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("Nomads cabinet source digest mismatch")
    return raw


def literal_body(raw: bytes) -> str:
    source = raw.decode()
    block = source.split("-- drawers at 33220,32858,9", 1)[1].split("-- bookcase", 1)[0]
    if any(f"itemId = {item}" not in block for item in (2434, 2853, 7527, 2833)):
        raise ValueError("Nomads cabinet identities changed")
    return block.split("text = [[", 1)[1].split("]]", 1)[0].removeprefix("\n")


TEXT = literal_body(retained_source())


def verify_source(path: Path) -> None:
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("Nomads cabinet source digest mismatch")
    source = raw.decode()
    block = source.split("-- drawers at 33220,32858,9", 1)[1].split("-- bookcase", 1)[0]
    actual = block.split("text = [[", 1)[1].split("]]", 1)[0].removeprefix("\n")
    if actual != TEXT or any(f"itemId = {item}" not in block for item in (2434, 2853, 7527, 2833)):
        raise ValueError("Nomads cabinet identity or literal text changed")


def document() -> dict:
    return {
        "schema": 1,
        "callbacks": [{
            "key": KEY,
            "quest": "oteryn:quest.tibia_tales",
            "trigger": "use",
            "selector": {
                "position": {"x": 33220, "y": 32858, "floor": -9},
                "appearance": 2434, "unique": None, "action": None,
            },
            "admission": {"premium": True},
            "requires_world_admission": True,
            "lever": None, "reversible": False, "passages": [], "teleport": None,
            "storage": [
                {"track": CLAIMED, "minimum": 0, "maximum": 0, "set": 1},
                {"track": ARITO, "minimum": 2, "maximum": 2, "set": 2, "preserve": True},
            ],
            "offerings": [], "item_on_target": None,
            "return_on_failure": False, "failure_message": None,
            "effects": {"item_reward": {
                "premium": True, "item": "oteryn:item.tibia.i2853", "count": 1,
                "contents": [
                    {"item": "oteryn:item.tibia.i7527", "count": 1},
                    {"item": "oteryn:item.tibia.i2833", "count": 1},
                ],
                "text": {"contents": [{"index": 1, "text": TEXT}]},
            }},
            "sources": [SOURCE_REF, SOURCE_URL,
                "content/quests/definitions/quests-00300-00351.json#oteryn:quest.authored.nomads_land_quest/s5"],
        }],
    }


def write(check: bool = False) -> None:
    path = ROOT / OUTPUT
    expected = json.dumps(document(), ensure_ascii=False, indent=2) + "\n"
    if check:
        if not path.exists() or path.read_text() != expected:
            raise ValueError(f"stale Nomads cabinet: {OUTPUT}")
    elif not path.exists() or path.read_text() != expected:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(expected)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--source", type=Path)
    arguments = parser.parse_args()
    if arguments.source:
        verify_source(arguments.source)
    write(arguments.check)
