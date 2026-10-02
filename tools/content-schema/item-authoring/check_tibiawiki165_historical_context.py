"""Replay old165 import bytes explicitly; independently guard current Forge3332."""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from contextlib import contextmanager
from pathlib import Path

import lower_item_forge3332_packet as forge
from engine_items import load_appearance_objects

ROOT = forge.ROOT
CONTEXT = (
    "docs/agents/evidence/OTV2-20261002-tibiawiki165-historical-import-context-v1.json"
)
CONTEXT_SHA = "254675adf42a677eeefdb3ed18a42aee90ee8c5ec78d6ac08dff6a999181fb76"
BINDINGS = "imports/tibiawiki/bindings/items.json"


def canonical(value):
    return (
        json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()


def read_context(root=ROOT):
    c = json.loads(forge.checked(root, CONTEXT, CONTEXT_SHA))
    raw = c["old165_bindings"]["raw_utf8"].encode()
    if (
        c["purpose"] != "HISTORICAL_IMPORT_BYTES_NOT_CURRENT_SOURCE_AUTHORITY"
        or forge.sha(raw) != c["old165_bindings"]["sha256"]
        or hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
        != c["old165_bindings"]["git_blob_oid"]
    ):
        raise ValueError("historical165 byte/purpose/Git witness drift")
    old = json.loads(raw)["bindings"]
    current = json.loads((root / BINDINGS).read_bytes())["bindings"]
    for binding in old:
        forge.exact_binding(current, binding, binding["target"])
    if len(old) != 165:
        raise ValueError("historical165 scope drift")
    for name in ("immutable_forge3332_packet", "immutable_forge3332_compiler"):
        row = c[name]
        forge.checked(root, row["path"], row["sha256"])
    return c


def hardlink_tree(source, target, excluded, detached=()):
    for p in source.rglob("*"):
        rel = p.relative_to(source)
        if "__pycache__" in rel.parts or str(rel) in excluded:
            continue
        q = target / rel
        if p.is_dir():
            q.mkdir(parents=True, exist_ok=True)
        elif p.is_file():
            q.parent.mkdir(parents=True, exist_ok=True)
            if any(
                rel == Path(prefix) or Path(prefix) in rel.parents
                for prefix in detached
            ):
                shutil.copy2(p, q)
            else:
                os.link(p, q)


@contextmanager
def historical_context(root=ROOT, native=False, weapon=False):
    c = read_context(root)
    with tempfile.TemporaryDirectory(
        prefix="oteryn-explicit-old165-", dir=root.parent
    ) as directory:
        path = Path(directory)
        # Writable Item-authoring outputs and their byte-identical decoders are
        # detached copies: formal verification rebuilds schemas/catalogs/templates.
        # Read-only reference inputs use hardlinks. No code/catalog substitution.
        for name in ("tools", "imports"):
            hardlink_tree(
                root / name,
                path / name,
                {"tibiawiki/bindings/items.json"} if name == "imports" else set(),
                detached=("content-schema/item-authoring",) if name == "tools" else (),
            )
        (path / BINDINGS).write_bytes(c["old165_bindings"]["raw_utf8"].encode())
        (path / "docs").symlink_to(root / "docs", target_is_directory=True)
        if not native and not weapon:
            (path / "content").symlink_to(root / "content", target_is_directory=True)
        else:
            (path / "content").mkdir()
            for p in (root / "content").iterdir():
                if not (native and p.name == "items" or weapon and p.name == "world"):
                    (path / "content" / p.name).symlink_to(
                        p, target_is_directory=p.is_dir()
                    )
        if native:
            (path / "content/items").mkdir()
            (path / "content/items/aliases.json").symlink_to(
                root / "content/items/aliases.json"
            )
            shard = "content/items/historical-forge3332.json"
            (path / shard).write_bytes(
                canonical(
                    {"records": [{"definition": c["forge3332_historical_definition"]}]}
                )
            )
            (path / "content/items/index.json").write_bytes(
                canonical({"shards": [shard]})
            )
        if weapon:
            # Historical declarations are an explicit input view, never written
            # through the live World tree. All other current definitions remain.
            (path / "content/world/definitions").mkdir(parents=True)
            for p in (root / "content/world").iterdir():
                if p.name != "definitions":
                    (path / "content/world" / p.name).symlink_to(
                        p, target_is_directory=p.is_dir()
                    )
            for p in (root / "content/world/definitions").iterdir():
                if p.name != "declarations.json":
                    (path / "content/world/definitions" / p.name).symlink_to(
                        p, target_is_directory=p.is_dir()
                    )
            import lower_item_forge289_packet as successor

            proof = json.loads(
                forge.checked(root, successor.PROOF, successor.PROOF_SHA)
            )
            declarations = json.loads(
                (root / "content/world/definitions/declarations.json").read_bytes()
            )
            declarations["item_authoring"] = proof["current_parent_authoring"]
            (path / "content/world/definitions/declarations.json").write_bytes(
                canonical(declarations)
            )
        yield path


def weapon_owner_scope(root=ROOT):
    """Only the immutable411 parent or its explicit sealed289 successor is live."""
    import lower_item_forge289_packet as successor

    proof = json.loads(forge.checked(root, successor.PROOF, successor.PROOF_SHA))
    packet = json.loads(
        forge.checked(
            root,
            "docs/agents/evidence/OTV2-20261002-item-forge289-promotion-v1.json",
            "376d668b5148d9c5f7b85eb3ef7e2f747c08fdc10170f62ed22e49ce4c6157ab",
        )
    )
    parent = proof["current_parent_authoring"]
    expected = parent + packet["promotions"]
    if (
        len(parent) != 411
        or len(packet["promotions"]) != 289
        or len({r["item"]["key"] for r in expected}) != 700
    ):
        raise ValueError("closed411/289 owner context scope drift")
    current = json.loads(
        (root / "content/world/definitions/declarations.json").read_bytes()
    )["item_authoring"]
    encode = lambda rows: canonical(sorted(rows, key=lambda r: r["item"]["key"]))
    if encode(current) not in (encode(parent), encode(expected)):
        raise ValueError("current owners outside exact sealed411/700 closure")
    # Recheck actual newest source, exact bridges, whole289 raw pairs and all
    # current target/name/header/stat/owner conflicts independently of replay.
    if canonical(successor.build(root)) != canonical(packet):
        raise ValueError("live289 source qualification drift")
    return proof, len(current)


def current_weapon103(root=ROOT):
    import lower_item_weapon_metadata_packet as weapon

    _, owner_count = weapon_owner_scope(root)
    # The unchanged compiler runs all103 raw/source/binding and scoped current
    # Native name/type/weapon checks; ONLY declarations411 are historical input.
    # This does not admit any unsealed future source-owner batch.
    with historical_context(root, weapon=True) as path:
        actual = canonical(weapon.build(path))
    expected = forge.checked(
        root,
        "docs/agents/evidence/OTV2-20261002-item-weapon-metadata-promotion-v1.json",
        "33e95859691705e550a45458a64a15ddb04b6981834e66ce76a618f7efc66e1b",
    )
    if actual != expected:
        raise ValueError("historical411/current103 packet reproduction drift")
    return {
        "current_owner_count": owner_count,
        "historical_owners": 411,
        "current103_native_source_raw_binding_guards": "PASS",
    }


def current_forge3332(root=ROOT):
    c = read_context(root)
    proof = json.loads(forge.checked(root, forge.PROOF, forge.PROOF_SHA))
    source = proof["source_pair"]
    row = json.loads(
        forge.checked(root, source["artifact_path"], source["artifact_sha256"])
    )["rows"][source["row_ordinal"]]
    bridge = proof["identity_bridge"]
    for path, name in (
        (BINDINGS, "br_to_full_native"),
        (
            "imports/crystalserver/bindings/items.json",
            "crystal_appearance_to_full_native",
        ),
    ):
        forge.exact_binding(
            json.loads((root / path).read_bytes())["bindings"],
            bridge[name]["row"],
            proof["target"],
        )
    frame = proof["official_membership_and_name"]
    index = json.loads(
        forge.checked(
            root, frame["admitted_index_path"], frame["admitted_index_sha256"]
        )
    )
    if (
        index["newest"] != frame["newest_descriptor"]["label"]
        or max(index["files"], key=lambda x: x["order"]) != frame["newest_descriptor"]
    ):
        raise ValueError("current3332 latest official frame drift")
    membership = json.loads(
        forge.checked(root, frame["membership_path"], frame["membership_sha256"])
    )
    if [x for x in membership["entries"] if x[0] == 3332] != [frame["entry"]]:
        raise ValueError("current3332 membership drift")
    sha = frame["complete_official_appearance_sha256"]
    obj = load_appearance_objects(
        forge.checked(root, f"content/assets/files/appearances-{sha}.dat", sha)
    )[3332]
    definitions = [
        r["definition"]
        for s in json.loads((root / "content/items/index.json").read_bytes())["shards"]
        for r in json.loads((root / s).read_bytes())["records"]
        if r["definition"]["identity"] == proof["target"]
    ]
    if len(definitions) != 1:
        raise ValueError("current3332 unique full identity drift")
    d = definitions[0]
    old = c["forge3332_historical_definition"]
    if canonical(
        {
            k: d[k]
            for k in (
                "identity",
                "kind",
                "stack_class",
                "materializable",
                "client_projection",
            )
        }
    ) != canonical(
        {
            k: old[k]
            for k in (
                "identity",
                "kind",
                "stack_class",
                "materializable",
                "client_projection",
            )
        }
    ):
        raise ValueError("current3332 scoped Item headers drift")
    routed = {
        r["provenance"]["item_pointer"]["key"]
        for family in ("objects", "terrain")
        for p in (root / f"content/world/{family}").glob("*.json")
        for r in json.loads(p.read_bytes()).get("records", [])
        if r.get("provenance", {}).get("item_pointer")
    }
    owners = json.loads(
        (root / "content/world/definitions/declarations.json").read_bytes()
    )["item_authoring"]
    draft = json.loads((root / "content/world/provenance/sources.json").read_bytes())[
        "source_identity_bindings"
    ]
    forge.exact_binding(draft, bridge["br_to_full_native"]["row"], proof["target"])
    forge.qualify(proof, row, d, obj, owners, routed)
    return {"historical_import_bindings": 165, "current3332_scoped_guards": "PASS"}


def check(root=ROOT, cohort="forge"):
    if cohort in {"forge", "all"}:
        with historical_context(root, native=True) as path:
            actual = canonical(forge.build(path))
        packet = read_context(root)["immutable_forge3332_packet"]
        if actual != forge.checked(root, packet["path"], packet["sha256"]):
            raise ValueError("historical3332 packet reproduction drift")
        current_forge3332(root)
    if cohort in {"weapon", "all"}:
        current_weapon103(root)
    if cohort == "weapon-tests":
        current_weapon103(root)
        with historical_context(root, weapon=True) as path:
            subprocess.run(
                [
                    sys.executable,
                    str(
                        path
                        / "tools/content-schema/item-authoring/test_lower_item_weapon_metadata_packet.py"
                    ),
                ],
                cwd=path,
                check=True,
                env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
            )
    for name, script in (
        ("engine", "test_engine_items.py"),
        ("formal", "verify_formal_schema.py"),
    ):
        if cohort in {name, "all"}:
            with historical_context(root) as path:
                subprocess.run(
                    [
                        sys.executable,
                        str(path / "tools/content-schema/item-authoring" / script),
                    ],
                    cwd=path,
                    check=True,
                    env={**os.environ, "PYTHONDONTWRITEBYTECODE": "1"},
                )
    return {
        "cohort": cohort,
        "status": "PASS",
        "scope": "explicit old165/411 input views; selected current3332/current103 guards independently live",
    }


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--cohort",
        choices=("forge", "engine", "formal", "weapon", "weapon-tests", "all"),
        default="forge",
    )
    print(json.dumps(check(cohort=p.parse_args().cohort)))


if __name__ == "__main__":
    main()
