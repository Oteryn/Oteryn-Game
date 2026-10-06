"""Bounded downloaded-tuple join; no Native definitions or SDK execution."""

import argparse
import gzip
import hashlib
import json
import resource
import sqlite3
import sys
from pathlib import Path

from literal import capture_assignments, emit_literal

CUTS = {
    "47dfd51f45280a59a1d3e50ba7edd573d7234446": "canary-47df",
    "ff7ede593c69d4c658b382c97443e8155926924a": "crystal-ff7",
    "00ce02a57ca5a12e48f32a3476e37471167e4c3f": "crystal-00ce",
}
ENUMS = {
    "CANARY_47DF": "canary-47df",
    "CRYSTAL_FF7": "crystal-ff7",
    "CRYSTAL_00CE": "crystal-00ce",
}
POOL = Path("/dev/shm/oteryn-source-flags-run-v6-7e/produced")
LOCK = Path("/tmp/oteryn-v6-root/source-heavy-operation.lock")
PINS = {
    "source-flag-read-write-registry.json": "f9b0cae9b18722142c8afad2abb50ea7c453c604d6ec561b4265388d50fe5c2c",
    "source-binding-phase-proof.json": "63a2fb3f3f11418e4dde261bf99c28b82f1b5537ff781563de17bc853fb5f670",
}


def sha_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for data in iter(lambda: stream.read(65536), b""):
            digest.update(data)
    return digest.hexdigest()


def blocks(path, marker, opening, closing):
    # Exact frozen Python indent2 producer framing, bounded one target/record only.
    active, lines, size = False, [], 0
    with path.open("rb") as stream:
        for line in stream:
            if not active:
                active = line.rstrip() == marker
                continue
            if line.startswith(opening):
                if lines:
                    raise ValueError("unclosed frozen JSON record")
                lines = [line]
                size = len(line)
            elif lines:
                lines.append(line)
                size += len(line)
                if size > 2 * 1024 * 1024:
                    raise ValueError("bounded record exceeds2MiB")
            elif line.startswith((b"  ]", b"  }")):
                return
            if lines and line.rstrip().rstrip(b",") == closing:
                raw = b"".join(lines).rstrip().rstrip(b",")
                yield (
                    json.loads(b"{" + raw + b"}")
                    if opening != b"    {"
                    else json.loads(raw)
                )
                lines, size = [], 0
    if lines or not active:
        raise ValueError("truncated frozen JSON stream")


def prepare(root, out, *, root_terminal_authorized=False):
    if not root_terminal_authorized or LOCK.exists():
        raise ValueError("HOLD Root exclusive release+lockabsence before cohort reads")
    if out.exists() or out.resolve().is_relative_to(root.resolve()):
        raise ValueError("new external output directory required")
    resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
    receipt = Path(
        "/tmp/oteryn-flags-whole-source-independent-review/qualified-source-phase-receipt.json"
    )
    if (
        sha_file(receipt)
        != "5e28444c0d48d3f5840ce8a49bc72955ab3c024a919fbccc5b29c39bd6149031"
    ):
        raise ValueError("wrong independently qualified downloaded tuple receipt")
    for name, expected in PINS.items():
        if sha_file(POOL / name) != expected:
            raise ValueError("downloaded tuple source changed")
    summary_path = Path("/dev/shm/oteryn-expanded-source-eligibility/summary.json")
    if (
        sha_file(summary_path)
        != "e1929f4b59cda2b46d3d232883271b5f034e7d22253397645bf22b1df5c8f137"
    ):
        raise ValueError("qualified own fullXML/source archive pins changed")
    summary = json.loads(summary_path.read_bytes())
    sys.path.insert(0, "/dev/shm/oteryn-source-flags-readwrite/restricted-cast-v5")
    from fixed16_parsers_v5 import attribute_value

    out.mkdir()
    db = sqlite3.connect(out / "join.sqlite3")
    db.execute("PRAGMA cache_size=-8192")
    db.execute(
        "CREATE TABLE rows(cut TEXT,id INTEGER,ordinal INTEGER,target TEXT,revision TEXT,header TEXT,binding TEXT,seen INTEGER DEFAULT0,PRIMARY KEY(cut,id))".replace(
            "DEFAULT0", "DEFAULT 0"
        )
    )
    db.execute("CREATE INDEX by_xml ON rows(cut,ordinal)")
    for target in blocks(
        POOL / "source-flag-read-write-registry.json",
        b'  "targets": {',
        b'    "oteryn:item',
        b"    }",
    ):
        key, revisions = next(iter(target.items()))
        for revision, entries in revisions.items():
            for entry in entries:
                if entry["external_item_id"] == 901:
                    raise ValueError("protected901")
                header = {
                    k: v
                    for k, v in entry.items()
                    if k not in {"ordered_assignments", "parameter"}
                }
                db.execute(
                    "INSERT INTO rows(cut,id,ordinal,target,revision,header)VALUES(?,?,?,?,?,?)",
                    (
                        ENUMS[entry["source_cut"]],
                        entry["external_item_id"],
                        entry["xml_record_ordinal"],
                        key,
                        revision,
                        json.dumps(header),
                    ),
                )
    for proof in blocks(
        POOL / "source-binding-phase-proof.json", b'  "records": [', b"    {", b"    }"
    ):
        binding = proof["own_binding"]
        cut = CUTS[binding["source_revision"]]
        identity = proof["target"]
        row = db.execute(
            "SELECT target,revision FROM rows WHERE cut=? AND id=?",
            (cut, int(binding["external_id"])),
        ).fetchone()
        if (
            row != (identity["key"], identity["revision"])
            or binding["target"] != identity
        ):
            raise ValueError("own loaded tuple target mismatch")
        if (
            db.execute(
                "UPDATE rows SET binding=? WHERE cut=? AND id=? AND binding IS NULL",
                (json.dumps(binding), cut, int(binding["external_id"])),
            ).rowcount
            != 1
        ):
            raise ValueError("duplicate own loaded tuple")
    count, maximum = (
        0,
        {
            "events": 0,
            "direct_assignments": 0,
            "lexeme_UTF8_bytes": 0,
            "serialized_bytes": 0,
        },
    )
    pins = dict(PINS)
    with gzip.GzipFile(
        filename=str(out / "literal-observations.jsonl.gz"), mode="wb", mtime=0
    ) as output:
        for cut in ENUMS.values():
            path = (
                root
                / f"imports/ots-source-evidence/upstream-items-source-first/xml/{cut}.observations.jsonl.gz"
            )
            expected = summary["input_sha256"].get(str(path))
            if not expected or sha_file(path) != expected:
                raise ValueError("own complete XML capture changed")
            pins[str(path)] = expected
            with gzip.open(path, "rb") as stream:
                for line in stream:
                    if len(line) > 2 * 1024 * 1024:
                        raise ValueError("XML capture record exceeds2MiB")
                    xml = json.loads(line)
                    ordinal = xml["source_record"]["ordinal"]
                    for source_id, key, revision, raw_header, raw_binding in db.execute(
                        "SELECT id,target,revision,header,binding FROM rows WHERE cut=? AND ordinal=?",
                        (cut, ordinal),
                    ):
                        header = json.loads(raw_header)
                        if (
                            header["xml_record_sha256"]
                            != xml["source_record"]["raw_xml_sha256"]
                            or raw_binding is None
                        ):
                            raise ValueError("exact own XML/prototype tuple mismatch")
                        assignments = capture_assignments(xml, attribute_value)
                        value = emit_literal(cut, assignments)
                        observed = {
                            "target": {
                                "family": "Item",
                                "key": key,
                                "revision": revision,
                            },
                            "own_binding": json.loads(raw_binding),
                            "header": header,
                            "parameter": value,
                        }
                        raw = (
                            json.dumps(observed, sort_keys=True, ensure_ascii=False)
                            + "\n"
                        ).encode()
                        output.write(raw)
                        if (
                            db.execute(
                                "UPDATE rows SET seen=1 WHERE cut=? AND id=? AND seen=0",
                                (cut, source_id),
                            ).rowcount
                            != 1
                        ):
                            raise ValueError("duplicate applied XML")
                        count += 1
                        maximum["events"] = max(
                            maximum["events"], len(value["ordered_events"])
                        )
                        maximum["direct_assignments"] = max(
                            maximum["direct_assignments"], len(assignments)
                        )
                        maximum["lexeme_UTF8_bytes"] = max(
                            maximum["lexeme_UTF8_bytes"],
                            max(
                                (len(a["value_lexeme"].encode()) for a in assignments),
                                default=0,
                            ),
                        )
                        maximum["serialized_bytes"] = max(
                            maximum["serialized_bytes"], len(raw)
                        )
    if (
        count != 45721
        or db.execute("SELECT COUNT(DISTINCT target)FROM rows").fetchone()[0] != 33975
        or db.execute("SELECT COUNT(*)FROM rows WHERE seen!=1").fetchone()[0]
    ):
        raise ValueError("incomplete exact downloaded cohort")
    db.commit()
    db.close()
    report = {
        "status": "DECLARATION_ONLY_FULL_LITERAL_CENSUS_NOT_NATIVE_ADMISSION",
        "targets": 33975,
        "own_rows": count,
        "maxima": maximum,
        "inputs": pins,
        "output_sha256": sha_file(out / "literal-observations.jsonl.gz"),
        "actual_CPP_execution": "UNKNOWN",
        "final_sparse_members": "UNKNOWN",
        "Native_promotions": 0,
    }
    (out / "receipt.json").write_text(json.dumps(report, indent=2) + "\n")
    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--root-terminal-authorized", action="store_true")
    args = parser.parse_args()
    prepare(args.root, args.out, root_terminal_authorized=args.root_terminal_authorized)
