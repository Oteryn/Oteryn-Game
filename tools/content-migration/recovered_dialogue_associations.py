"""Offline exact-speaker extraction; no prompt-to-keyword or identity qualification."""

import argparse
import hashlib
import json
import re
from pathlib import Path


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load(path):
    raw = path.read_bytes()
    return json.loads(raw), {"path": path.name, "sha256": sha(raw)}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--recovery", required=True, type=Path)
    ap.add_argument("--inventory", required=True, type=Path)
    ap.add_argument("--r4-index", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    args = ap.parse_args()
    recovered, recovery_proof = load(args.recovery)
    inventory, inventory_proof = load(args.inventory)
    original, original_proof = load(args.r4_index)
    no_primary = {r["npc_key"] for r in original["records"] if not r.get("source_id")}
    nonresponse = re.compile(
        r"^\(?\s*(?:does not respond|doesn.t respond|no response|nothing|silence)\s*\)?[.!]?$",
        re.IGNORECASE,
    )
    captures = []
    associations = {}
    for index, row in enumerate(recovered["records"]):
        raw = (args.recovery.parent / row["capture"]).read_bytes()
        blob = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
        assert (
            sha(raw) == row["sha256"]
            and blob == row["git_blob_sha"]
            and len(raw) == row["bytes"]
        ), row["npc_key"]
        lines = []
        silence = []
        offset = 0
        for number, line in enumerate(raw.splitlines(keepends=True), 1):
            body = line.rstrip(b"\r\n")
            text = body.decode("utf-8")
            match = re.match(r"^([^:]+):[ \t]*(.*)$", text)
            if (
                match
                and match[1].strip().casefold() == row["name"].casefold()
                and match[2].strip()
            ):
                speech = match[2].strip()
                left = text.find(speech, match.start(2))
                item = {
                    "line_1based": number,
                    "speaker_literal": match[1],
                    "speech": speech,
                    "line_byte_start_0based": offset,
                    "line_byte_end_exclusive": offset + len(body),
                    "speech_byte_start_0based": offset + len(text[:left].encode()),
                    "speech_byte_end_exclusive": offset
                    + len(text[: left + len(speech)].encode()),
                    "speech_sha256": sha(speech.encode()),
                }
                (silence if nonresponse.fullmatch(speech) else lines).append(item)
            offset += len(line)
        assert (
            len(lines) == row["literal_actor_lines"]
            and len(silence) == row["nonresponse_observations"]
        ), row["npc_key"]
        capture = {
            k: row[k]
            for k in (
                "npc_key",
                "name",
                "repository",
                "revision",
                "path",
                "url",
                "sha256",
                "git_blob_sha",
                "bytes",
                "declared_status",
                "other_actor_labels",
            )
        }
        capture.update(
            source_index_pointer=f"/records/{index}",
            capture_locator="capture-artifact:" + Path(row["capture"]).name,
            literal_speaker_lines=lines,
            nonresponse_observations=silence,
            state="LITERAL_SPEAKER_OBSERVATION"
            if lines
            else "SOURCE_WITHOUT_MATCHED_SPEECH",
            identity_binding_qualified=False,
            runtime_qualified=False,
        )
        captures.append(capture)
        associations.setdefault(row["npc_key"], []).append(index)
    proposed = []
    for index, npc in enumerate(inventory["proposed_records"]):
        field = next(
            f
            for f in npc["fields"]
            if f["field_path"] == "oteryn:source.npc.inventory_identity_qualification"
        )
        qualification = json.loads(field["value"]["value"])
        key = npc["identity"]["key"]
        refs = associations.get(key, [])
        literal = any(captures[i]["literal_speaker_lines"] for i in refs)
        proposed.append(
            {
                "npc_key": key,
                "name": qualification["name"],
                "identity_revision": npc["identity"]["revision"],
                "inventory_pointer": f"/proposed_records/{index}",
                "identity_source_proofs": qualification["source_proofs"],
                "capture_indices": refs,
                "state": "NAMED_LITERAL_OBSERVATIONS_AVAILABLE"
                if literal
                else "NO_NAMED_LITERAL_SPEECH",
                "native_dialogue_admission": "HELD_MATCHER_GUARDS_AND_IDENTITY_UNKNOWN",
                "identity_binding_qualified": False,
                "runtime_qualified": False,
            }
        )
    observed = {r["npc_key"] for r in captures if r["literal_speaker_lines"]}
    new_observed = [
        r["npc_key"]
        for r in proposed
        if r["state"] == "NAMED_LITERAL_OBSERVATIONS_AVAILABLE"
    ]
    new_missing = [
        r["npc_key"]
        for r in proposed
        if r["state"] != "NAMED_LITERAL_OBSERVATIONS_AVAILABLE"
    ]
    closed = sorted(no_primary & observed)
    missing = sorted(no_primary - observed)
    contamination = [
        {
            "npc_key": r["npc_key"],
            "capture_index": i,
            "other_actor_labels": r["other_actor_labels"],
            "state": "FOREIGN_SPEAKER_CONTAMINATION_HOLD",
        }
        for i, r in enumerate(captures)
        if r["other_actor_labels"] and not r["literal_speaker_lines"]
    ]
    result = {
        "schema": "oteryn.recovered-npc-literal-associations.v1",
        "scope": "LITERAL_SPEAKER_OBSERVATIONS_ONLY",
        "input_custody": [recovery_proof, inventory_proof, original_proof],
        "runtime_qualified": False,
        "identity_bindings_qualified": False,
        "global_complete": False,
        "independent_confirmation": False,
        "counts": {
            "new_proposals": len(proposed),
            "new_with_named_literal_speech": len(new_observed),
            "new_without_named_literal_speech": len(new_missing),
            "fresh_no_primary_observation_closures": len(closed),
            "remaining_no_primary": len(missing),
            "literal_speaker_lines": sum(
                len(r["literal_speaker_lines"]) for r in captures
            ),
        },
        "proposed_identity_associations": proposed,
        "source_captures": captures,
        "fresh_no_primary_observation_closures": [
            {
                "npc_key": key,
                "capture_indices": associations[key],
                "runtime_qualified": False,
            }
            for key in closed
        ],
        "missing_new_64": new_missing,
        "missing_existing_170": missing,
        "speaker_contamination_holds": contamination,
        "qualification_holds": [
            "Recorded prompts do not prove native matchers.",
            "Literal speaker names do not qualify identity binding.",
            "Quest guards, actions, handler precedence and Global completeness remain unknown.",
        ],
    }
    assert (
        len(proposed),
        len(new_observed),
        len(new_missing),
        len(closed),
        len(missing),
    ) == (159, 95, 64, 6, 170)
    args.out.write_bytes(
        (
            json.dumps(
                result, ensure_ascii=False, sort_keys=True, separators=(",", ":")
            )
            + "\n"
        ).encode()
    )
    print(json.dumps(result["counts"], sort_keys=True))


if __name__ == "__main__":
    main()
