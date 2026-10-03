"""Reproduce immutable defaults historically; independently validate current Items."""

import argparse
import hashlib
import json
import tempfile
from collections import defaultdict
from contextlib import contextmanager
from pathlib import Path

import lower_wiki_stack_default_packet as base
import lower_wiki_stack_historical_packet as historical
from engine_items import load_appearance_objects

ROOT = base.ROOT
CONTEXT = (
    "docs/agents/evidence/OTV2-20261002-stack-default-historical-native-context-v1.json"
)
CONTEXT_SHA = "d8a31202c7eec25b68b7c0fe95dad5e02c5e6298935b695497262d07a6f3fb20"
PARENT = "6a68dbcb64538f9369aeb633d116061f7096ba30"


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
    """Only explicit historical Item definitions differ; builds read symlinked inputs."""
    context = read_context(root)
    with tempfile.TemporaryDirectory(prefix="oteryn-historical-default-") as directory:
        path = Path(directory)
        for name in ("tools", "docs", "imports"):
            (path / name).symlink_to((root / name).resolve(), target_is_directory=True)
        (path / "content/items").mkdir(parents=True)
        for name in ("assets", "world"):
            (path / "content" / name).symlink_to(
                (root / "content" / name).resolve(), target_is_directory=True
            )
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
    if (
        base.sha(
            json.dumps(
                snapshot["records"],
                sort_keys=True,
                ensure_ascii=False,
                separators=(",", ":"),
            ).encode()
        )
        != packet["sources"]["snapshot_sha256"]
        or base.sha((root / base.BINDINGS).read_bytes())
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
