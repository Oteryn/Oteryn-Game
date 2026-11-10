"""Reproduce immutable defaults historically; independently validate current Items."""

import argparse
import hashlib
import json
import tempfile
import sys
import tarfile
from functools import lru_cache
from collections import defaultdict
from contextlib import contextmanager
from pathlib import Path

import historical_world_owner_context as world_context
import lower_wiki_stack_default_packet as base
import lower_wiki_stack_historical_packet as historical
from engine_items import load_appearance_objects

ROOT = base.ROOT
CONTEXT = (
    "docs/agents/evidence/OTV2-20261002-stack-default-historical-native-context-v1.json"
)
CONTEXT_SHA = "e2bf1480e600159d306f5bf5e759456d4d322ad972036ed156538a934fc4503b"
PARENT = "6a68dbcb64538f9369aeb633d116061f7096ba30"
FROZEN_INPUT_PARENT = "7cc6dfe8f57e879b2f49a6420b15279b3d2c94d8"
FROZEN_INPUT_ARCHIVE = "tools/content-schema/item-authoring/historical-default-inputs-v1.tar.gz"
FROZEN_INPUT_ARCHIVE_SHA256 = "a9ca2e5b9e4c23ce2bd39c17659468ce318e8255b651142236f2a5ece3dbb2bf"


@lru_cache(maxsize=1)
def frozen_archive():
    source = ROOT / FROZEN_INPUT_ARCHIVE
    if base.sha(source.read_bytes()) != FROZEN_INPUT_ARCHIVE_SHA256:
        raise ValueError('historical input archive digest drift')
    with tarfile.open(source, 'r:gz') as stream:
        members = stream.getmembers()
        if any(not member.isfile() or member.name.startswith('/') or '..' in Path(member.name).parts
               for member in members) or sum(member.size for member in members) > 128_000_000:
            raise ValueError('historical input archive membership drift')
        result = {member.name: stream.extractfile(member).read() for member in members}
    if len(result) != len(members):
        raise ValueError('duplicate historical input archive member')
    return result


def frozen_input(root, path, expected):
    raw = (root / path).read_bytes()
    if base.sha(raw) != expected:
        raw = frozen_archive().get(path, b'')
    if base.sha(raw) != expected:
        raise ValueError('historical input byte reproduction drift: ' + path)
    return raw


def overlay_input(root, relative, raw):
    """Detach only the frozen input from a view; never modify its live symlink target."""
    path = root
    for name in Path(relative).parts[:-1]:
        path /= name
        if path.is_symlink():
            source = path.resolve()
            path.unlink()
            path.mkdir()
            for child in source.iterdir():
                (path / child.name).symlink_to(child, target_is_directory=child.is_dir())
        else:
            path.mkdir(exist_ok=True)
    target = root / relative
    if target.is_symlink():
        target.unlink()
    target.write_bytes(raw)


def canonical(value):
    return (
        json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))
        + "\n"
    ).encode()


def git_blob_oid(raw):
    return hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()


def read_context(root=ROOT):
    context = json.loads(base.checked(root, CONTEXT, CONTEXT_SHA))
    source = json.loads(base.checked(root, base.PROOF, base.PROOF_SHA))
    if (
        context["native_parent"] != PARENT
        or source["native_parent"] != PARENT
        or context["purpose"]
        != "HISTORICAL_QUALIFICATION_ONLY_NOT_CURRENT_NATIVE_AUTHORITY"
    ):
        raise ValueError("historical native parent/purpose drift")
    raw = context["source_index"]["raw_utf8"].encode()
    if (
        base.sha(raw) != context["source_index"]["sha256"]
        or git_blob_oid(raw) != context["source_index"]["git_blob_oid"]
    ):
        raise ValueError("historical original index Git blob digest drift")
    shards = {s["path"]: s for s in context["source_shards"]}
    if len(shards) != 69 or set(shards) != set(json.loads(raw)["shards"]):
        raise ValueError("historical69 shard source projection drift")
    definitions, per_shard = {}, defaultdict(int)
    scope = {s["item_key"]: s["source_item_id"] for s in source["records"]}
    for record in context["selected_native_definitions"]:
        definition = record["definition"]
        identity = definition["identity"]
        key = identity["key"]
        if (
            key in definitions
            or identity != {"family": "Item", "key": key, "revision": "definition-r1"}
            or scope.get(key) != record["source_item_id"]
            or base.sha(canonical(definition)) != record["definition_sha256"]
            or shards[record["source_shard"]]["git_blob_oid"]
            != record["source_blob_git_oid"]
        ):
            raise ValueError("historical literal definition/source blob identity drift")
        definitions[key] = definition
        per_shard[record["source_shard"]] += 1
    if (
        len(definitions) != 1651
        or set(definitions) != set(scope)
        or any(s["selected_records"] != per_shard[s["path"]] for s in shards.values())
    ):
        raise ValueError("historical closed1651 definition scope drift")
    for path, digest in context["immutable_artifacts"].items():
        base.checked(root, path, digest)
    return context


@contextmanager
def qualification_context(root=ROOT):
    """Reconstruct historical Item definitions and hash-verified owner receipts."""
    context = read_context(root)
    with tempfile.TemporaryDirectory(prefix="oteryn-historical-default-") as directory:
        path = Path(directory)
        for name in ("tools", "docs", "imports"):
            (path / name).symlink_to((root / name).resolve(), target_is_directory=True)
        (path / "content/items").mkdir(parents=True)
        for name in ("assets",):
            (path / "content" / name).symlink_to(
                (root / "content" / name).resolve(), target_is_directory=True
            )
        packet_path = str(base.OUTPUT.relative_to(base.ROOT))
        frozen = json.loads(base.checked(root, packet_path, context['immutable_artifacts'][packet_path]))
        for relative, expected in frozen['sources']['map_owner_inputs'].items():
            overlay_input(path, relative, frozen_input(root, relative, expected))
        overlay_input(path, base.COMPILER, frozen_input(root, base.COMPILER, frozen['compiler']['sha256']))
        overlay_input(path, base.BINDINGS, frozen_input(root, base.BINDINGS, frozen['sources']['bindings_sha256']))
        # This full-file pin precedes the reviewed ten-key-only relocation.
        overlay_input(path, base.WIKI, frozen_input(root, base.WIKI,
            '24e3e57fddc6c14f00aa5c4976e91117d31bc2102bd3835b3d7891d9e41fbeae'))
        shard = "content/items/historical-qualification-definitions.json"
        (path / shard).write_bytes(
            canonical(
                {
                    "records": [
                        {"definition": r["definition"]}
                        for r in context["selected_native_definitions"]
                    ]
                }
            )
        )
        (path / "content/items/index.json").write_bytes(
            canonical(
                {
                    "purpose": context["purpose"],
                    "native_parent": PARENT,
                    "shards": [shard],
                    "record_count": 1651,
                }
            )
        )
        yield path


def current_inputs(root, context_root=ROOT):
    context = read_context(context_root)
    for path, digest in context["immutable_artifacts"].items():
        base.checked(root, path, digest)
    source = json.loads(base.checked(root, base.PROOF, base.PROOF_SHA))
    objects = load_appearance_objects(base.checked(root, base.CLIENT, base.CLIENT_SHA))
    bound = base.exact_bindings(
        json.loads((root / base.BINDINGS).read_text())["bindings"],
        source["bridge"]["source_revisions"],
    )
    snapshot = json.loads((root / base.WIKI).read_text())
    packet_path = str(base.OUTPUT.relative_to(base.ROOT))
    packet = json.loads(
        base.checked(root, packet_path, context["immutable_artifacts"][packet_path])
    )
    # Identity admission re-keyed ten records without changing observations.
    # Recover and hash the exact frozen dictionary, rather than accepting a
    # new snapshot digest as equivalent evidence.
    frozen_records = dict(snapshot['records'])
    for iid in (40016, 40598, 40601, 40603, 42851, 42853, 43589, 43590, 43625, 43860):
        key = f'oteryn:item.tibia.i{iid}'
        if key in frozen_records:
            if str(iid) in frozen_records:
                raise ValueError('duplicate historical snapshot identity')
            frozen_records[str(iid)] = frozen_records.pop(key)
    sys.path.insert(0, str(context_root / 'tools/content-schema/native-gameplay'))
    from build_spell_item_identities import historical_binding_bytes
    frozen_bindings = historical_binding_bytes(
        (root / base.BINDINGS).read_bytes(), packet['sources']['bindings_sha256'])
    if (
        base.sha(
            json.dumps(
                frozen_records,
                sort_keys=True,
                ensure_ascii=False,
                separators=(",", ":"),
            ).encode()
        )
        != packet["sources"]["snapshot_sha256"]
        or base.sha(frozen_bindings)
        != packet["sources"]["bindings_sha256"]
    ):
        raise ValueError("current unchanged source input bytes drift")
    wiki, pages = {}, defaultdict(set)
    for r in snapshot["records"].values():
        wiki[r["item_id"]] = r["observations"]
        for o in r["observations"]:
            pages[o["page_id"]].add(r["item_id"])
    definitions = {}
    for shard in json.loads((root / "content/items/index.json").read_text())["shards"]:
        for r in json.loads((root / shard).read_text())["records"]:
            d = r["definition"]
            if d["identity"]["key"] in definitions:
                raise ValueError("duplicate current native Item")
            definitions[d["identity"]["key"]] = d
    routed = {
        r["provenance"]["item_pointer"]["key"]
        for family in ("objects", "terrain")
        for p in (root / f"content/world/{family}").glob("*.json")
        for r in json.loads(p.read_text()).get("records", [])
        if r.get("provenance", {}).get("item_pointer")
    }
    return source, objects, bound, wiki, pages, definitions, routed


def current_validation(root=ROOT, context_root=ROOT):
    source, objects, bound, wiki, pages, definitions, routed = current_inputs(
        root, context_root
    )
    packet = json.loads((root / base.OUTPUT.relative_to(base.ROOT)).read_text())
    original = {r["source_item_id"] for r in packet["promotions"]}
    blocked, outside = [], []
    for record in source["records"]:
        iid, key = record["source_item_id"], record["item_key"]
        if base.sha(record["content"].encode()) != record["content_sha256"]:
            raise ValueError("current qualification raw source bytes drift")
        why = base.reasons(
            record,
            definitions.get(key, {}),
            bound.get(key),
            objects.get(iid, {}),
            routed,
            wiki.get(iid, []),
            pages,
            source["qualification_cutoff"],
        )
        if iid in original and why:
            blocked.append({"source_item_id": iid, "reasons": why})
        elif iid not in original and not why:
            outside.append(iid)
    if blocked or len(original) != 1487:
        raise ValueError(f"original1487 current source guards failed: {blocked}")
    frames = json.loads(base.checked(root, historical.PROOF, historical.PROOF_SHA))
    for frame in frames["frames"]:
        iid, key = frame["current"]["source_item_id"], frame["current"]["item_key"]
        historical.qualify(
            frame,
            definitions.get(key, {}),
            bound.get(key),
            objects.get(iid, {}),
            routed,
            wiki.get(iid, []),
            pages,
            frames["qualification_cutoff"],
        )
    if len(frames["frames"]) != 7:
        raise ValueError("original historical7 scope drift")
    return {
        "strict_current_original_defaults": 1487,
        "strict_current_original_historical": 7,
        "historical_default_holds": 164,
        "current_eligible_outside_original_default_cohort": sorted(outside),
        "outside_cohort_promotions_written": 0,
    }


def check(root=ROOT, current_root=None, cohort="both"):
    context = read_context(root)
    with qualification_context(root) as historical_root:
        for name, module in (("default", base), ("historical", historical)):
            if cohort not in {name, "both"}:
                continue
            data = canonical(module.build(historical_root))
            path = str(module.OUTPUT.relative_to(module.ROOT))
            if data != base.checked(root, path, context["immutable_artifacts"][path]):
                raise ValueError(f"historical {name} packet byte reproduction drift")
    return current_validation(current_root or root, root)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--cohort", choices=("default", "historical", "both"), default="both"
    )
    parser.add_argument(
        "--current-root",
        type=Path,
        help="Actual current checkout; never synthesize name/state changes",
    )
    args = parser.parse_args()
    print(
        json.dumps(
            check(current_root=args.current_root, cohort=args.cohort), sort_keys=True
        )
    )


if __name__ == "__main__":
    main()
