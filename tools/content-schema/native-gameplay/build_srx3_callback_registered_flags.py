#!/usr/bin/env python3
"""Audit registered callback flags from the already fetched, pinned donor blobs.

This emits evidence only. It never changes native Ability metadata or AI rules.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "docs/agents/evidence/srx3-native-callback-registered-flags.json"
REVISIONS = {"canary": "47dfd51f45280a59a1d3e50ba7edd573d7234446",
             "crystal": "201189f871d23597b82caf88fd611454a2ddefb8"}
REPOSITORIES = {"canary": "opentibiabr/canary", "crystal": "zimbadev/crystalserver"}
FILES = {"field-fill-heal_brain_head": "heal_brain_head",
         "field-fill-lisa_heal": "lisa_heal", "field-fill-tyrn_heal": "tyrn_heal",
         "field-fill-foamsplash": "foam_splash", "berserk_bridge": "smelly_cheese_berserk",
         "maxxenteleport": "maxxen_teleport", "rotthingshaper": "rotthing_shaper",
         "rotthingwave": "rotthing_wave", "glooth_generator_summon": "glooth-generator_summon",
         "gaz_haragoth_death": "gaz'haragoth_death",
         "gaz_haragoth_summon": "gaz'haragoth_summon"}


def registered_flags(text):
    result = {"needs_target": False, "needs_direction": False}
    setters = []
    for line, source in enumerate(text.splitlines(), 1):
        source = source.split("--", 1)[0]
        for match in re.finditer(r":(needTarget|needDirection)\(([^)]*)\)", source):
            value = match[2].strip()
            if value not in ("true", "false"):
                raise ValueError(f"unresolved registered flag at line {line}: {source}")
            key = {"needTarget": "needs_target", "needDirection": "needs_direction"}[match[1]]
            result[key] = value == "true"
            setters.append({"line": line, "flag": key, "value": result[key]})
    return result, setters


def verified_blob(blobs, entry):
    raw = (blobs / (entry["sha"] + ".lua")).read_bytes()
    digest = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
    if digest != entry["sha"]:
        raise ValueError("donor Git blob mismatch: " + entry["path"])
    return raw


def build(canary_tree, crystal_tree, blobs):
    trees = {repo: json.loads(path.read_bytes())["tree"]
             for repo, path in (("canary", canary_tree), ("crystal", crystal_tree))}
    witness = json.loads((ROOT / "crates/gameplay/content/src/content/native_monster_callback_witness.json").read_bytes())
    profiles = {row["target"]["key"]: row for row in witness["profiles"]}
    rows = []
    for entry in witness["entries"]:
        ability = entry["ability"]["key"]
        name = ability.rsplit(".", 1)[-1]
        filename = FILES.get(name, name) + ".lua"
        chosen = None
        for repo in ("canary", "crystal"):
            candidates = [row for row in trees[repo]
                          if row["path"].endswith("/" + filename) and "/spells/" in row["path"]]
            candidates.sort(key=lambda row: ("data-global/" not in row["path"], row["path"]))
            if candidates:
                chosen = repo, candidates[0]
                break
        if chosen is None:
            raise ValueError("missing pinned registered spell: " + ability)
        repo, donor = chosen
        raw = verified_blob(blobs, donor)
        flags, setters = registered_flags(raw.decode())
        details = profiles[ability]["data"]["profile"]["details"]
        native = {key: details[key] for key in ("needs_target", "needs_direction")}
        rows.append({"creature": entry["creature"], "ability": entry["ability"],
                     "list": entry["phase"], "index": entry["index"],
                     "native_metadata": native, "registered_flags": flags,
                     "metadata_differs": native != flags,
                     "source": {"repository": REPOSITORIES[repo], "revision": REVISIONS[repo],
                                "path": donor["path"], "git_blob": donor["sha"],
                                "sha256": hashlib.sha256(raw).hexdigest()},
                     "setters": setters})
    return {"schema": "OTERYN_SRX3_CALLBACK_REGISTERED_FLAGS/v1",
            "source_definitions_sha256": witness["source_definitions_sha256"],
            "note": "Attack proposals carry the independently selected combat target; source callback stage/anchor owns VAR semantics. This evidence does not authorize metadata projection.",
            "rows": rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--canary-tree", type=Path, required=True)
    parser.add_argument("--crystal-tree", type=Path, required=True)
    parser.add_argument("--blobs", type=Path, required=True)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    value = build(args.canary_tree, args.crystal_tree, args.blobs)
    output = (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2) + "\n").encode()
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_bytes() != output:
            raise SystemExit("stale callback registered-flags audit")
    else:
        OUTPUT.write_bytes(output)
    print("CALLBACK_REGISTERED_FLAGS", len(value["rows"]),
          sum(row["metadata_differs"] for row in value["rows"]), hashlib.sha256(output).hexdigest())


if __name__ == "__main__":
    main()
