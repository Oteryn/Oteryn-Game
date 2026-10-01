"""Validate bounded public facts separately from unknown Global/runtime details."""
from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
from urllib.parse import urlsplit

from behavior_answers import QUESTION_IDS, TARGET

PACKET = Path(__file__).parent / "samples/global-research-closure.json"


def validate(packet: dict) -> None:
    def require(condition, message):
        if not condition:
            raise ValueError(message)

    require(packet["schema"] == "OTERYN_IMBUEMENT_GLOBAL_RESEARCH_CLOSURE/v1",
            "unexpected closure schema")
    require(packet["target"] == TARGET and packet["activation"] == "DRAFT_NOT_RUNTIME_READY",
            "closure cannot activate runtime or change the data target")
    require(packet["full_global_parity_proven"] is False, "bounded research cannot certify full Global parity")
    require(type(packet["research_limits"]["observations_performed"]) is int
            and packet["research_limits"]["observations_performed"] == 0,
            "this batch performed no gameplay observations")
    sources = packet["sources"]
    for source in sources.values():
        url = urlsplit(source["url"])
        require(url.scheme == "https" and bool(url.netloc), "source needs a public HTTPS URL")
        require(source["method"] in {"REMOTE_DESKTOP_CHROME_CDP", "NORMAL_PUBLIC_HTTP_OR_RETAINED_EXTRACT"},
                "unqualified source access method")
        require("OTS" not in source["source_role"], "OTS code cannot certify a public Global fact")
        require(source["captured_text"] and source["captured_text_scope"], "missing capture scope/body")
        require(hashlib.sha256(source["captured_text"].encode()).hexdigest() == source["captured_text_sha256"],
                "captured public text digest mismatch")
        require(source["original_digest"] and source["original_digest_scope"], "missing original capture identity")
        require(re.fullmatch(r"[0-9a-f]{64}", source["original_digest"]) is not None,
                "malformed original source digest")
        require(source["capture_date"] == "2026-10-01" and isinstance(source["publication_dates"], dict),
                "capture date must remain separate from publication dates")
    for conflict in packet["source_conflicts"]:
        require(all(sid in sources for sid in conflict["source_refs"]), "conflict references a missing source")
        for field in ("quote", "literal_quote"):
            if field in conflict:
                require(conflict[field] and any(conflict[field] in sources[sid]["captured_text"]
                                               for sid in conflict["source_refs"]),
                        "conflict quote missing from its attributed source")
        for claim in conflict.get("claims", []):
            require(claim["quote"] in sources[claim["source"]]["captured_text"], "conflict quote missing from source")
    groups = packet["groups"]
    ids = [g["id"] for g in groups]
    require(len(ids) == 11 and set(ids) == QUESTION_IDS - {"exact_target_snapshot"},
            "closure must account for each original behavioral question exactly once")
    facts = packet["facts"]
    fact_ids = [f["id"] for f in facts]
    require(len(set(fact_ids)) == len(fact_ids), "duplicate public fact")
    for fact in facts:
        require(fact["group"] in ids and fact["value"] is not None, "fact needs a bounded selected value")
        require(fact["qualification"] and fact["limits"] and fact["quote_refs"], "fact lacks evidence limits")
        for ref in fact["quote_refs"]:
            require(ref["source_id"] in sources, "fact references a missing source")
            require(ref["text"] and ref["text"] in sources[ref["source_id"]]["captured_text"],
                    "quote is not present in the captured source text")
    for group in groups:
        require(group["status"] == "BOUNDED_PUBLIC_FACTS_AND_EXPLICIT_RESIDUALS",
                "group cannot silently become fully Global-confirmed")
        expected = {f["id"] for f in facts if f["group"] == group["id"]}
        require(set(group["qualified_fact_refs"]) == expected, "group omits or misassigns public facts")
        require(group["known_scope"] and group["sufficient_public_evidence"], "missing scoped acceptance")
        require(group["runtime_contract"]["owner"] == "ARCHITECT_COORDINATOR_162",
                "runtime contract needs its owning handoff")
        for field in group["remaining_public_fields"]:
            require(field["value"] is None and field["status"] == "UNCONFIRMED_OR_SOURCE_CONFLICT"
                    and field["description"], "residual fields cannot acquire invented Global values")
        for ref in group["existing_public_source_refs"]:
            catalogue = json.loads((PACKET.parent / ref["catalogue"]).read_text())
            registry = catalogue["sources"]
            source_ids = set(registry) if isinstance(registry, dict) else {s["id"] for s in registry}
            require(ref["id"] in source_ids, "retained source reference is missing from its catalogue")
    require(packet["counts"] == {"groups": len(groups), "facts": len(facts), "sources": len(sources)},
            "closure inventory counts do not match")


if __name__ == "__main__":
    packet = json.loads(PACKET.read_text())
    validate(packet)
    print(json.dumps({"status": "PASS", **packet["counts"], "full_global_parity_proven": False}))
