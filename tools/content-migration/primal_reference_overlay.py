"""Closed Crystal Primal explicit-empty loot reference; never infer null loot facts."""
from __future__ import annotations

import argparse
import ast
import copy
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[2]
SOURCE = "data-global/monster/quests/primal_ordeal_quest/the_primal_menace.lua"
CAPTURE = "tools/content-schema/native-gameplay/primal-empty-loot-reference.json"
CAPTURE_SHA256 = "f7fb0aa1ba6fc2030834d7a9684e62b72610b6b4a4a0629068dfe723238c6a62"
CREATURE = "oteryn:creature.the_primal_menace"
LOOT = "oteryn:loot.creature.the_primal_menace"
BODY = "oteryn:item.tibia.i39530"
REVISION = "definition-r1"


def reference(family, key):
    return {"family": family, "key": key, "revision": REVISION}


def source_pin(root=ROOT):
    tree = ast.parse((root / "tools/content-schema/encounters/import_boss_levers.py").read_text())
    values = {}
    for node in tree.body:
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id in {"PIN", "PRIMAL_HOOK_BLOBS"}:
                    values[target.id] = ast.literal_eval(node.value)
    return values["PIN"], values["PRIMAL_HOOK_BLOBS"][SOURCE]


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def capture(source_root, root=ROOT):
    raw = (source_root / SOURCE).read_bytes()
    revision, blob = source_pin(root)
    actual = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
    if actual != blob:
        raise ValueError("Primal source blob changed")
    text = re.sub(r"--\[\[.*?\]\]", "", raw.decode(), flags=re.S)
    text = re.sub(r"--[^\n]*", "", text)
    # Crystal201189f the_primal_menace.lua:79,118 explicitly declares both facts.
    if re.findall(r"\bmonster\.corpse\s*=\s*([^\n]+)", text) != ["39530"]:
        raise ValueError("Primal source corpse changed")
    if re.findall(r"\bmonster\.loot\s*=\s*([^\n]+)", text) != ["{}"]:
        raise ValueError("Primal source empty loot changed")
    return {"schema": "OTERYN_PRIMAL_EXPLICIT_EMPTY_LOOT/v1", "source_revision": revision,
            "source_path": SOURCE, "source_git_blob": blob,
            "source_sha256": hashlib.sha256(raw).hexdigest(),
            "creature": reference("Creature", CREATURE), "corpse": reference("Item", BODY),
            "loot": {"kind": "Loot", "identity": reference("Loot", LOOT),
                     "algorithm": "IndependentBernoulliPpm", "entries": []}}


def witness(root=ROOT):
    raw = (root / CAPTURE).read_bytes()
    if hashlib.sha256(raw).hexdigest() != CAPTURE_SHA256:
        raise ValueError("Primal empty loot witness digest changed")
    row = json.loads(raw)
    revision, blob = source_pin(root)
    if (set(row) != {"schema", "source_revision", "source_path", "source_git_blob", "source_sha256", "creature", "corpse", "loot"}
        or row["schema"] != "OTERYN_PRIMAL_EXPLICIT_EMPTY_LOOT/v1"
        or row["source_revision"] != revision or row["source_path"] != SOURCE
        or row["source_git_blob"] != blob
        or row["creature"] != reference("Creature", CREATURE)
        or row["corpse"] != reference("Item", BODY)
        or row["loot"] != {"kind": "Loot", "identity": reference("Loot", LOOT), "algorithm": "IndependentBernoulliPpm", "entries": []}):
        raise ValueError("Primal empty loot witness facts changed")
    return row


def apply_reference(records, root=ROOT):
    """Mutate canonical reference records in place; repeat application is idempotent."""
    facts = witness(root)
    rows = {}
    for record in records:
        key = record["identity"]["key"]
        if key in rows:
            raise ValueError("Primal reference duplicate identity")
        rows[key] = record
    creature, body = rows.get(CREATURE), rows.get(BODY)
    if (creature is None or creature.get("kind") != "Creature"
        or creature["identity"] != facts["creature"] or body is None
        or body.get("kind") != "Item" or body["identity"] != facts["corpse"]):
        raise ValueError("Primal reference creature or corpse identity changed")
    loot = facts["loot"]
    if creature.get("loot") not in [None, loot["identity"]]:
        raise ValueError("Primal reference loot binding changed")
    if LOOT in rows and rows[LOOT] != loot:
        raise ValueError("Primal reference explicit-empty loot changed")
    creature["loot"] = copy.deepcopy(loot["identity"])
    if LOOT not in rows:
        records.append(copy.deepcopy(loot))
    return records


NATIVE_LOOT = "tools/content-schema/native-gameplay/loot-tables.json"


def apply_native(root, generated, manifest_path, encode):
    """Project the same explicit source witness into the existing native loot provider."""
    facts = witness(root)
    raw = generated[NATIVE_LOOT] if NATIVE_LOOT in generated else (root / NATIVE_LOOT).read_bytes()
    document = json.loads(raw)
    if document.get("schema") != "OTERYN_NATIVE_LOOT_TABLES/v1":
        raise ValueError("Primal native loot schema changed")
    bindings = [row for row in document["creature_loot"]
                if row["creature"]["key"] == CREATURE]
    expected = facts["loot"]["identity"]
    if (len(bindings) != 1 or bindings[0]["creature"] != facts["creature"]
            or bindings[0]["loot"] not in [None, expected]):
        raise ValueError("Primal native loot binding changed")
    bodies = [row for row in document["items"] if row["item"]["key"] == BODY]
    if (len(bodies) != 1 or bodies[0]["item"] != facts["corpse"]
            or bodies[0]["materializable"] is not True or bodies[0]["container_capacity"] != 30):
        raise ValueError("Primal native corpse closure changed")
    table = {key: copy.deepcopy(facts["loot"][key]) for key in ("identity", "algorithm", "entries")}
    tables = [row for row in document["tables"] if row["identity"]["key"] == LOOT]
    if len(tables) > 1 or (tables and tables[0] != table):
        raise ValueError("Primal native explicit-empty loot changed")
    bindings[0]["loot"] = copy.deepcopy(expected)
    if not tables:
        at = next((index for index, row in enumerate(document["tables"])
                   if row["identity"]["key"] > LOOT), len(document["tables"]))
        document["tables"].insert(at, table)
    data = encode(document)
    manifest = json.loads(generated[manifest_path])
    # The baseline manifest gains repository-input providers later in registration.
    # Admit this known provider now while preserving any existing path and metadata.
    manifest.setdefault("loot_tables", {"path": Path(NATIVE_LOOT).name})["sha256"] = hashlib.sha256(data).hexdigest()
    # Publish the complete pair only after every witness and conflict check passed.
    generated[NATIVE_LOOT] = data
    generated[manifest_path] = encode(manifest)


def main():
    global CAPTURE_SHA256
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.source_root:
        raw = canonical(capture(args.source_root))
        if args.check:
            if (ROOT / CAPTURE).read_bytes() != raw:
                raise ValueError("Primal empty loot capture stale")
        else:
            (ROOT / CAPTURE).write_bytes(raw)
            path = Path(__file__)
            path.write_text(re.sub(r'^CAPTURE_SHA256 = "[^"]+"$',
                'CAPTURE_SHA256 = "' + hashlib.sha256(raw).hexdigest() + '"',
                path.read_text(), flags=re.M))
            CAPTURE_SHA256 = hashlib.sha256(raw).hexdigest()
    witness()


if __name__ == "__main__":
    main()
