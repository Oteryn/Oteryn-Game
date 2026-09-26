#!/usr/bin/env python3
"""Stage TibiaWiki Creature observations from three pinned, independent archives.

The earlier bulk extractor artifact was superseded as a qualified G4 result.
Every Creature row is therefore independently joined to the sealed G3 family
decision and G4 raw-byte digest/revision tuple before its extracted fields are
retained. These are source observations, not Oteryn gameplay definitions.
"""
from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime
import hashlib
import json
from pathlib import Path
import re
import zipfile

ARCHIVES = {
    "g3": ("a63635e4cdfcd237cf69b5e2fe0471c30723e45fe3ab2ccc9095730a9337d2e7", "source-family-classified-universe.json"),
    "g4": ("5d8e886e31a0f64a035a61dc5c3f8ce031a8af3b06926117750deebc841394ca", "non-item-source-capture.json"),
    "fields": ("ec0db29ba93e537a3a4253473d76f4482a98d7eee362cae59a797a084a6e683e", "bulk-crosswalk.json"),
}
ALLOWED_FIELDS = {"name", "hp", "exp", "speed", "armor", "armour", "mitigation", "pushable", "pushobjects", "immunities", "resistances"}
SIGNATURE = "G1_CREATURES_STWORZENIA_INFOBOX_CRIATURA_CRIATURAS"


def read_archive(path: Path, role: str) -> dict:
    digest, member = ARCHIVES[role]
    if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise ValueError(f"{role}: archive digest mismatch")
    with zipfile.ZipFile(path) as archive:
        return json.loads(archive.read(member))


def keyed(rows: list[dict], key: str) -> dict[str, dict]:
    result = {str(row[key]): row for row in rows}
    if len(result) != len(rows):
        raise ValueError(f"duplicate {key}")
    return result


def stage(g3: dict, g4: dict, fields: dict) -> list[dict]:
    classified = keyed([
        row for row in g3["pages"]
        if row["source_family_classification"]["primary_definition_family"] == "Creature"
        and row["source_family_classification"]["state"] == "SOURCE_DEFINITION_PRIMARY"
        and row["source_family_classification"]["assignment_rule"] == SIGNATURE
    ], "page_id")
    captured = keyed(g4["pages"], "external_id")
    extracted = keyed(fields["families"]["Creature"], "external_id")
    if len(classified) != 2149 or set(classified) != set(extracted):
        raise ValueError("Creature family census differs from the pinned G3 decision")
    output = []
    for page_id in sorted(classified, key=int):
        g3row, g4row, row = classified[page_id], captured[page_id], extracted[page_id]
        observations = [item for item in g3row["provenance"] if item["lane"] == "G1_LIVE_NON_ITEM"]
        if len(observations) != 1:
            raise ValueError(f"{page_id}: missing unique G3 source observation")
        g3source = observations[0]
        if (row["g3_revision_id"], row["g3_revision_timestamp"], row["g3_title_observation"]) != (
            g3source["revision_id"], g3source["revision_timestamp"], g3source["observed_title"]
        ):
            raise ValueError(f"{page_id}: extracted G3 provenance differs from classification")
        if g3source["redirect"] or "Predefinição:Infobox Criatura" not in g3source["templates"]:
            raise ValueError(f"{page_id}: G3 source is not a direct creature infobox")
        if row["family"] != "Creature" or row["g3_direct_family_signature"] != SIGNATURE:
            raise ValueError(f"{page_id}: wrong family signature")
        if row["source_state"] not in {"SOURCE_REVISION_STABLE", "CURRENT_REVISION_REVALIDATED"}:
            raise ValueError(f"{page_id}: unresolved source state")
        if row["source_shape"] != "STRUCTURED_FAMILY_INFOBOX" or row["current_infobox_template"] != "Infobox_Criatura":
            raise ValueError(f"{page_id}: unsupported source shape")
        checks = {
            "current_title": "title", "current_revision_id": "revision_id",
            "current_revision_timestamp": "revision_timestamp",
            "current_raw_utf8_sha256": "raw_utf8_sha256", "page_key": "page_key",
        }
        for field, captured_field in checks.items():
            if row[field] != g4row[captured_field]:
                raise ValueError(f"{page_id}: {field} differs from independent G4 capture")
        if g4row["page_id"] != int(page_id) or g4row["source"] != "TIBIAWIKI_STRUCTURED":
            raise ValueError(f"{page_id}: bad source identity")
        if not isinstance(row["current_revision_id"], int) or row["current_revision_id"] <= 0:
            raise ValueError(f"{page_id}: invalid revision")
        datetime.strptime(row["current_revision_timestamp"], "%Y-%m-%dT%H:%M:%SZ")
        if not isinstance(row["current_title"], str) or not row["current_title"].strip():
            raise ValueError(f"{page_id}: invalid title")
        if not re.fullmatch("[0-9a-f]{64}", row["current_raw_utf8_sha256"]):
            raise ValueError(f"{page_id}: invalid digest")
        if row["source_state"] == "SOURCE_REVISION_STABLE":
            if row["g3_revision_id"] != row["current_revision_id"] or row["g3_title_observation"] != row["current_title"]:
                raise ValueError(f"{page_id}: invalid stable G3 provenance")
        elif page_id != "63947" or not row["prior_revision_drift"]:
            raise ValueError(f"{page_id}: unexpected revision drift")
        raw = row["structured_fields"]
        if not isinstance(raw, dict) or not raw.keys() <= ALLOWED_FIELDS:
            raise ValueError(f"{page_id}: unexpected fields")
        if any(not isinstance(value, str) or len(value.encode("utf-8")) > 128 for value in raw.values()):
            raise ValueError(f"{page_id}: malformed scalar field")
        if "name" in raw and raw["name"] != row["current_title"]:
            raise ValueError(f"{page_id}: infobox name and page title diverge")
        output.append({
            "source_page_id": page_id,
            "source_title": row["current_title"],
            "source_revision_id": row["current_revision_id"],
            "source_revision_timestamp": row["current_revision_timestamp"],
            "source_raw_utf8_sha256": row["current_raw_utf8_sha256"],
            "observed_categories": sorted(set(g3source["categories"])),
            "observed_fields": raw,
        })
    return output


def emit(rows: list[dict], output: Path) -> None:
    output.mkdir(parents=True, exist_ok=True)
    files = []
    for start in range(0, len(rows), 500):
        filename = f"creatures-{start:05d}-{min(start + 499, len(rows) - 1):05d}.json"
        payload = {"schema": "OTERYN_TIBIAWIKI_CREATURE_SOURCE_OBSERVATIONS/v1", "family": "Creature", "records": rows[start:start + 500]}
        data = (json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()
        (output / filename).write_bytes(data)
        files.append({"path": filename, "records": len(payload["records"]), "sha256": hashlib.sha256(data).hexdigest()})
    counts = Counter(field for row in rows for field in row["observed_fields"])
    manifest = {
        "schema": "OTERYN_TIBIAWIKI_CREATURE_SOURCE_MANIFEST/v1",
        "source": "TIBIAWIKI_STRUCTURED",
        "source_namespace": "mediawiki/tibiawiki.com.br",
        "identity_namespace": "mediawiki/page_id",
        "family": "Creature",
        "record_count": len(rows),
        "field_counts": dict(sorted(counts.items())),
        "inputs": {role: {"archive_sha256": digest, "member": member} for role, (digest, member) in ARCHIVES.items()},
        "source_revision_drift_page_ids": ["63947"],
        "authority": "SOURCE_OBSERVATIONS_ONLY; no Oteryn ProductionKeys, gameplay promotion, or Crystal/Canary match",
        "files": files,
    }
    (output / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, sort_keys=True, indent=2) + "\n")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for role in ARCHIVES:
        parser.add_argument(f"--{role}", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    emit(stage(*(read_archive(getattr(args, role), role) for role in ARCHIVES)), args.output)


if __name__ == "__main__":
    main()
