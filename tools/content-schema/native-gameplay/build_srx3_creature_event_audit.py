#!/usr/bin/env python3
"""Read-only, pinned donor event closure for W01's real native callback corpus.

This is an evidence census, not a runtime-implementation or completion claim.
Inputs are the published callback witnesses and actual Git-object Lua cache.
The optional tree paths permit reproducing it from API trees of the pinned refs.
"""
import argparse
import hashlib
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[3]
REVISIONS = {
    "canary": ("opentibiabr/canary", "47dfd51f45280a59a1d3e50ba7edd573d7234446"),
    "crystal": ("zimbadev/crystalserver", "201189f871d23597b82caf88fd611454a2ddefb8"),
}
EVENT = re.compile(r"CreatureEvent\(\s*(['\"])(.*?)\1\s*\)")
STRINGS = re.compile(r"(['\"])([^\n]*?)\1")
TYPE = re.compile(r"Game\.createMonsterType\(\s*(['\"])(.*?)\1\s*\)")
EVENTS = re.compile(r"monster\.events\s*=\s*\{(.*?)\}", re.S)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def build(cache, trees):
    witness = json.loads((ROOT / "crates/gameplay/content/src/content/native_monster_callback_witness.json").read_text())
    seed_bindings = [b for b in witness["source_identity_bindings"] if b["target"]["family"] == "Creature"]
    result = {"schema": "OTERYN_SRX3_CREATURE_EVENT_AUDIT/v1", "completion_claim": False,
              "source_definitions_sha256": witness["source_definitions_sha256"],
              "seed_bindings": seed_bindings, "donors": []}
    for donor, (repository, revision) in REVISIONS.items():
        tree = json.loads(trees[donor].read_text())["tree"]
        files, handlers, types = {}, {}, {}
        monster_prefix = "data-otservbr-global/monster/" if donor == "canary" else "data-global/monster/"
        for entry in tree:
            path = entry["path"]
            if not path.endswith(".lua"):
                continue
            source = cache / (entry["sha"] + ".lua")
            if not source.exists():
                continue
            data = source.read_bytes()
            blob = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
            if blob != entry["sha"]:
                raise ValueError(f"non-Git-object source cache: {path}")
            text = data.decode()
            files[path] = (entry["sha"], sha(data), text)
            if path.startswith(monster_prefix):
                match = TYPE.search(text)
                if match:
                    types.setdefault(match[2].casefold(), []).append(path)
            if "/scripts/" in path:
                for match in EVENT.finditer(text):
                    handlers.setdefault(match[2], []).append(path)
        script_prefix = "data-otservbr-global/scripts/" if donor == "canary" else "data-global/scripts/"
        required = [entry["path"] for entry in tree if entry["path"].endswith(".lua")
                    and entry["path"].startswith((monster_prefix, script_prefix))]
        missing_cache = [path for path in required if path not in files]
        if missing_cache:
            raise ValueError("incomplete pinned source cache: " + ", ".join(missing_cache))
        pending, missing = [], []
        for binding in seed_bindings:
            external = binding["external_id"]
            suffix = external if external.endswith(".lua") else external + ".lua"
            if suffix.startswith("data-global/monster/"):
                suffix = suffix.removeprefix("data-global/monster/")
            candidates = [path for path in files if path.startswith(monster_prefix) and path.endswith("/" + suffix)]
            if candidates:
                pending.extend(candidates)
            else:
                missing.append({"creature": binding["target"]["key"], "external_id": external,
                                "fact": "no exact source path in this pinned cached donor; no alias inferred"})
        for name in ["Omrafir", "Hellfire Fighter", "Hirintror", "White Pale"]:
            pending.extend(types.get(name.casefold(), []))
        creatures, event_uses, visited = {}, {}, set()
        while pending:
            path = pending.pop()
            if path in visited:
                continue
            visited.add(path)
            blob, digest, text = files[path]
            match = TYPE.search(text)
            if match:
                registered = EVENTS.search(text)
                names = [m[2] for m in STRINGS.finditer(registered[1])] if registered else []
                creatures[path] = {"name": match[2], "path": path, "git_blob": blob,
                                   "sha256": digest, "events": names}
                for name in names:
                    event_uses.setdefault(name, set()).add(path)
                    pending.extend(handlers.get(name, []))
            # Literal birth calls do not inherit unrelated names from a
            # generic boss-death progression table. Only a genuinely dynamic
            # first argument conservatively pulls named alternatives from the file.
            calls = list(re.finditer(r"Game\.createMonster\(\s*", text))
            literal = list(re.finditer(r"Game\.createMonster\(\s*(['\"])(.*?)\1", text))
            child_names = [call[2] for call in literal]
            child_names.extend(call[2] for call in re.finditer(r":setType\(\s*(['\"])(.*?)\1", text))
            child_names.extend(call[2] for call in re.finditer(r"addEvent\(Game\.createMonster,\s*[^,]+,\s*(['\"])(.*?)\1", text))
            summons = re.search(r"monster\.summons\s*=\s*\{(.*?)\n\}", text, re.S)
            if summons:
                child_names.extend(call[2] for call in re.finditer(r"name\s*=\s*(['\"])(.*?)\1", summons[1]))
            if len(calls) > len(literal):
                child_names.extend(quoted[2] for quoted in STRINGS.finditer(text))
            for name in child_names:
                pending.extend(types.get(name.casefold(), []))
        # These already-restored W01 intents have source CreatureEvent
        # declarations but no monster.events attachment in their literal Lua.
        # Keep that distinction visible; declaration alone is not a caller.
        for name in ["OmrafirExplode", "HirintrorHeal", "WhitePaleHeal"]:
            event_uses.setdefault(name, set())
        events = []
        for name, users in sorted(event_uses.items()):
            sources = []
            for path in sorted(handlers.get(name, [])):
                blob, digest, text = files[path]
                sources.append({"path": path, "git_blob": blob, "sha256": digest,
                                "lines": [{"line": i, "text": line} for i, line in enumerate(text.splitlines(), 1)],
                                "callbacks": [{"line": i, "signature": line.strip()} for i, line in enumerate(text.splitlines(), 1)
                                              if re.search(r"function\s+\w+\.on\w+\(", line)],
                                "birth_expressions": [{"line": i, "expression": line.strip()} for i, line in enumerate(text.splitlines(), 1)
                                                      if "Game.createMonster(" in line]})
            events.append({"name": name, "registered_by": sorted(users), "sources": sources,
                           "attachment_status": "literal-monster-events" if users else "declaration-without-literal-monster-attachment",
                           "implementation_status": "requires-current-production-caller-audit"})
        result["donors"].append({"repository": repository, "revision": revision,
                                  "source_cache_complete": True,
                                  "scanned_global_monster_files": sum(path.startswith(monster_prefix) for path in required),
                                  "scanned_global_script_files": sum(path.startswith(script_prefix) for path in required),
                                  "closure_rule": "actual MonsterType named alternatives in reached birth/transform files; generic progression-table names do not imply births; source-defined aliases are not fabricated",
                                  "creatures": sorted(creatures.values(), key=lambda v: v["path"]),
                                  "events": events, "missing_exact_seed_paths": missing,
                                  "counts": {"creatures": len(creatures), "event_names": len(events),
                                             "literal_registered_event_names": sum(bool(event["registered_by"]) for event in events),
                                             "missing_event_sources": sum(not event["sources"] for event in events)}})
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", type=pathlib.Path, default=pathlib.Path("/tmp/srx3-lua-audit/blobs"))
    parser.add_argument("--canary-tree", type=pathlib.Path, default=pathlib.Path("/tmp/srx-canary-tree.json"))
    parser.add_argument("--crystal-tree", type=pathlib.Path, default=pathlib.Path("/tmp/srx-crystal-tree.json"))
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / "docs/agents/evidence/srx3-creature-event-audit.json")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    value = build(args.cache, {"canary": args.canary_tree, "crystal": args.crystal_tree})
    data = (json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True) + "\n").encode()
    if args.check:
        if args.output.read_bytes() != data:
            raise SystemExit("stale W01 source event audit")
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_bytes(data)
    print(json.dumps({"bytes": len(data), "sha256": sha(data), "donors": [d["counts"] for d in value["donors"]]}))


if __name__ == "__main__":
    main()
