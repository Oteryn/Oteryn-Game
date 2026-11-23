"""Source-bound Nomads Land cabinet and its literal readable reward."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
REVISION = "9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d"
SOURCE_PATH = "data-global/startup/tables/container_misc.lua"
SOURCE_SHA256 = "da146858a7bfbef47f601e02fc2b621c5e639ec2d096a6831446800277129bca"
SOURCE_URL = f"https://raw.githubusercontent.com/zimbadev/crystalserver/{REVISION}/{SOURCE_PATH}#L803-L822"
SOURCE_REF = f"crystalserver:{REVISION}:{SOURCE_PATH}:803-822:sha256:{SOURCE_SHA256}"
ARITO_COMPLETION_REF = (
    "crystalserver:201189f871d23597b82caf88fd611454a2ddefb8:"
    "data-global/npc/arito.lua:97-105:sha256:"
    "72036db5c28bae5fabb54a43657cb83f57a550665def3fc01b33357a652d79f1"
)
OUTPUT = "content/quests/world-callbacks/questpc5-nomads-cabinet.json"
KEY = "oteryn:quest-callback/nomads-land-sacred-casket-cabinet"
CLAIMED = "oteryn:quest-progress/quest/u8_1/tibia_tales/nomads_land_cabinet"
ARITO = "oteryn:quest-progress/quest/u8_1/tibia_tales/aritos_task"


def retained_source(path: str = SOURCE_PATH, digest: str = SOURCE_SHA256) -> bytes:
    archive = (
        ROOT / "tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz"
    )
    with tarfile.open(archive) as corpus:
        manifest = json.load(corpus.extractfile("corpus-manifest.json"))
        entry = next(
            row
            for row in manifest["files"]
            if row["source"] == "crystalserver"
            and row["revision"] == REVISION
            and row["path"] == path
        )
        raw = corpus.extractfile(entry.get("blob_path", entry.get("cache_path"))).read()
    if hashlib.sha256(raw).hexdigest() != digest:
        raise ValueError(f"Nomads source digest mismatch: {path}")
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
    if actual != TEXT or any(
        f"itemId = {item}" not in block for item in (2434, 2853, 7527, 2833)
    ):
        raise ValueError("Nomads cabinet identity or literal text changed")


def document() -> dict:
    return {
        "schema": 1,
        "callbacks": [
            {
                "key": KEY,
                "quest": "oteryn:quest.tibia_tales",
                "trigger": "use",
                "selector": {
                    "position": {"x": 33220, "y": 32858, "floor": -9},
                    "appearance": 2434,
                    "unique": None,
                    "action": None,
                },
                "admission": {"premium": True},
                "requires_world_admission": True,
                "lever": None,
                "reversible": False,
                "passages": [],
                "teleport": None,
                "storage": [
                    {"track": CLAIMED, "minimum": 0, "maximum": 0, "set": 1},
                    {
                        "track": ARITO,
                        "minimum": 2,
                        "maximum": 3,
                        "set": 2,
                        "preserve": True,
                    },
                ],
                "offerings": [],
                "item_on_target": None,
                "return_on_failure": False,
                "failure_message": None,
                "effects": {
                    "item_reward": {
                        "premium": True,
                        "item": "oteryn:item.tibia.i2853",
                        "count": 1,
                        "contents": [
                            {"item": "oteryn:item.tibia.i7527", "count": 1},
                            {"item": "oteryn:item.tibia.i2833", "count": 1},
                        ],
                        "text": {"contents": [{"index": 1, "text": TEXT}]},
                    }
                },
                "sources": [
                    SOURCE_REF,
                    SOURCE_URL,
                    ARITO_COMPLETION_REF,
                    "content/quests/definitions/quests-00300-00351.json#oteryn:quest.authored.nomads_land_quest/s5",
                ],
            }
        ],
    }


def referral_documents() -> dict[str, dict]:
    path = "data-global/npc/arito.lua"
    digest = ARITO_COMPLETION_REF.rsplit(":", 1)[1]
    source = retained_source(path, digest).decode()
    offer, accept = re.findall(
        r"npcHandler:say\(\{(.*?)\}, npc, creature\)", source, re.DOTALL
    )
    item = re.search(r"player:getItemCount\((\d+)\) > 0", source)[1]
    begin = {"track": ARITO, "minimum": 0, "maximum": 0, "set": 1}
    npc = "oteryn:npc.arito"
    rows = {
        "content/quests/npc-offers/nomads-land.json": (
            "offers",
            {
                "npc": npc,
                "keywords": ["nomads"],
                "topic": 0,
                "next_topic": 1,
                "storage": [{**begin, "set": 0, "preserve": True}],
                "reply": re.findall(r'"([^"\n]+)"', offer),
            },
        ),
        "content/quests/npc-callbacks/nomads-land.json": (
            "callbacks",
            {
                "key": "oteryn:quest-reward/nomads-land-arito-referral",
                "quest": "oteryn:quest.tibia_tales",
                "npc": npc,
                "keywords": ["yes"],
                "topic": 1,
                "next_topic": 0,
                "storage": [
                    begin,
                    {
                        "track": ARITO.rsplit("/", 1)[0] + "/default_start",
                        "minimum": 0,
                        "maximum": 1,
                        "set": 1,
                    },
                ],
                "take": [],
                "present": [{"item": f"oteryn:item.tibia.i{item}", "count": 1}],
                "give": None,
                "outfit": None,
                "achievement": None,
                "reply": re.findall(r'"([^"\n]+)"', accept),
                "sources": [f"crystalserver:{REVISION}:{path}:71-96:sha256:{digest}"],
            },
        ),
    }
    documents = {}
    for relative, (section, row) in rows.items():
        data = json.loads((ROOT / relative).read_bytes())
        other_rows = [
            entry
            for entry in data[section]
            if (
                entry["key"] != row["key"]
                if section == "callbacks"
                else (entry["npc"], entry["topic"], entry["keywords"])
                != (npc, row["topic"], row["keywords"])
            )
        ]
        data[section] = [row, *other_rows]
        documents[relative] = data
    return documents


def write(check: bool = False) -> None:
    for relative, data in {OUTPUT: document(), **referral_documents()}.items():
        path = ROOT / relative
        expected = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        if path.exists() and path.read_text() == expected:
            continue
        if check:
            raise ValueError(f"stale Nomads content: {relative}")
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
