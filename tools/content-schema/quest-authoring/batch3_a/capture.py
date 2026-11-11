"""Pinned literal capture helpers for reviewed quest authoring recipes.

This is a build-time reader, never a Lua interpreter. It uses the repository's
literal decoder and records each source file's SHA-256 beside the emitted rules.
"""

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
from build_dialogues import PIN, QUESTS, validate_document
from lua_tables import Parser, _string, tokenize


def snake(name):
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", name).lower()


def item(item_id):
    return f"oteryn:item.tibia.i{item_id}"


def topic(value):
    return {"kind": "set_topic", "value": value}


def inventory(kind, item_id, count=1, subtype=None):
    result = {"kind": kind, "item": item(item_id), "count": count}
    if subtype is not None:
        result["subtype"] = subtype
    return result


class Capture:
    def __init__(self, donor, quest, prefix):
        self.donor = Path(donor)
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=self.donor, text=True).strip()
        if head != PIN.split(":")[1]:
            raise ValueError(f"Unpinned donor HEAD: {head}")
        self.quest = quest
        self.key = QUESTS[quest]
        self.prefix = "oteryn:quest-progress/" + prefix
        self.rules = []
        self.tracks = {}
        self.files = {}

    def read(self, npc):
        path = f"data-global/npc/{npc}.lua"
        data = (self.donor / path).read_bytes()
        self.files[path] = hashlib.sha256(data).hexdigest()
        return data.decode()

    def ref(self, npc, line):
        return f"{PIN}data-global/npc/{npc}.lua:{line}"

    def track(self, name, maximum=2147483647, line=None, npc=None):
        key = self.prefix + "/" + "/".join(snake(part) for part in name.split("."))
        if key not in self.tracks:
            if line is None:
                line = {"bigfoot_burden": 1906, "fathers_burden": 1703,
                        "hot_cuisine": 1317, "in_service_of_yalahar": 1230,
                        "the_hidden_city_of_beregar": 1289,
                        "the_cursed_crystal": 2249, "the_way_of_the_monk": 3089}[self.quest]
            sources = [self.ref(npc, line)] if npc else [f"{PIN}data-global/lib/core/storages.lua:{line}"]
            self.tracks[key] = {"key": key, "quest": self.key, "initial": -1, "min": -1, "max": maximum, "sources": sources}
        return key

    def guard(self, name, op, value, maximum=None):
        guard = {"kind": "storage", "track": self.track(name), "op": op, "value": value}
        if maximum is not None:
            guard["maximum"] = maximum
        return guard

    def effect(self, name, value, kind="set"):
        return {"kind": kind, "track": self.track(name), "value": value}

    def say(self, npc, line):
        lines = self.read(npc).splitlines(keepends=True)
        code = "".join(lines[line - 1:])
        match = re.search(r"npcHandler:say\s*\(", code)
        if not match or "\n" in code[:match.start()]:
            raise ValueError(f"Expected say at {npc}:{line}")
        value = Parser(code[match.end():]).value()
        if isinstance(value, str):
            return [value]
        if isinstance(value, dict) and "fields" in value and all(isinstance(f["value"], str) for f in value["fields"]):
            return [f["value"] for f in value["fields"]]
        raise ValueError(f"Nonliteral say at {npc}:{line}")

    def strings(self, npc, first, last):
        code = "\n".join(self.read(npc).splitlines()[first - 1:last])
        return [_string(token) for token in tokenize(code) if token[0] in {"string", "lstring"}]

    def rule(self, npc, label, line, keywords=(), guards=(), effects=(), replies=None, say=None, topics=None, exclude=(), **extra):
        if replies is None:
            replies = []
            for anchor in ([] if say is None else ([say] if isinstance(say, int) else say)):
                replies.extend(self.say(npc, anchor))
        rule = {
            "key": f"oteryn:quest-npc/{self.key.split('quest.')[1]}/{npc}/{label}",
            "quest": self.key, "npc": "oteryn:npc." + npc.replace(".", ""),
            "trigger": "say", "keywords": list(keywords), "guards": list(guards),
            "reply": replies, "effects": list(effects), "sources": [self.ref(npc, line)],
        }
        if topics is not None:
            rule["topics"] = list(topics)
        if exclude:
            rule["exclude_keywords"] = list(exclude)
        rule.update(extra)
        self.rules.append(rule)
        return rule

    def save(self):
        document = {"schema": 1, "quest": self.key, "tracks": list(self.tracks.values()), "rules": self.rules,
                    "source_files": [{"path": path, "sha256": sha, "revision": PIN.split(":")[1]} for path, sha in sorted(self.files.items())]}
        from questpc2_live_eligibility import prerequisites
        document = prerequisites(document)
        validate_document(document)
        path = HERE / "npc" / f"{self.quest}.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n")
        print(f"Captured {self.quest}: {len(self.rules)} rules, {len(self.tracks)} tracks")
