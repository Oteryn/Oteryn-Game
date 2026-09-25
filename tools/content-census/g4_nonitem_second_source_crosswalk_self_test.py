#!/usr/bin/env python3
"""Synthetic, network-free regression tests for G4 second-source evidence."""
from __future__ import annotations

import importlib.util
import json
from pathlib import Path
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("g4_second_source", HERE / "g4_nonitem_second_source_crosswalk.py")
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def primary(**overrides):
    row = {
        "family": "Creature", "source": "TIBIAWIKI_STRUCTURED", "source_namespace": "mediawiki/tibiawiki.com.br",
        "identity_namespace": "mediawiki/page_id", "external_id": "10", "page_key": "mediawiki/tibiawiki.com.br/page_id/10",
        "current_title": "Demon", "source_state": "SOURCE_REVISION_STABLE",
        "structured_fields": {"name": "Demon", "health": "8200", "experience": "6000", "speed": "260"},
    }
    row.update(overrides)
    return row


def candidate(**overrides):
    row = {
        "family": "Creature", "source": "TIBIAWIKI_FANDOM", "source_namespace": "mediawiki/tibia.fandom.com",
        "identity_namespace": "mediawiki/page_id", "external_id": "20", "page_key": "mediawiki/tibia.fandom.com/page_id/20",
        "title": "Demon", "revision_id": 100, "revision_timestamp": "2026-09-01T00:00:00Z",
        "source_sha1": "a" * 40, "source_digest": "b" * 64, "facts": {"health": "8200", "experience": "6000"},
    }
    row.update(overrides)
    return row


def expect_error(code: str, thunk) -> None:
    try:
        thunk()
    except MODULE.CaptureError as exc:
        assert code in str(exc), (code, str(exc))
    else:
        raise AssertionError(f"expected {code}")


def main() -> None:
    # Two independent non-title facts make an exact candidate, never a binding.
    result = MODULE.classify_pair(primary(), [candidate()], "Creature")
    assert result["state"] == "EXACT_CANDIDATE"
    assert result["matching_facts"] == ["experience", "health"]
    assert "canonical_target" not in result

    # Title/name equality is not an independent discriminator.
    title_only = MODULE.classify_pair(primary(structured_fields={"name": "Demon"}), [candidate(facts={"name": "Demon"})], "Creature")
    assert title_only["state"] == "AMBIGUOUS" and title_only["reason"] == "TITLE_ONLY_NOT_EXACT"

    # Page-name aliases are not guessed or promoted to title matches.
    alias = MODULE.classify_pair(primary(current_title="The Demon"), [candidate(title="Demon")], "Creature")
    assert alias["state"] == "NO_MATCH"

    # Same title with colliding source pages remains ambiguous.
    collision = MODULE.classify_pair(primary(), [candidate(), candidate(external_id="21", page_key="mediawiki/tibia.fandom.com/page_id/21")], "Creature")
    assert collision["state"] == "AMBIGUOUS" and collision["reason"] == "MULTIPLE_EXACT_TITLE_CANDIDATES"

    # Conflicting independent values are preserved as conflict evidence.
    conflict = MODULE.classify_pair(primary(), [candidate(facts={"health": "9000", "experience": "9000"})], "Creature")
    assert conflict["state"] == "CONFLICT" and set(conflict["conflicting_facts"]) == {"experience", "health"}

    # Fandom source identity is mandatory; a derived API or mirror is rejected.
    expect_error("INDEPENDENT_SOURCE_IDENTITY_INVALID", lambda: MODULE.classify_pair(primary(), [candidate(source="TIBIADATA", source_namespace="api.tibiadata.bytewizards.de")], "Creature"))

    # Continuation tokens are opaque, distinct, and loop-checked.
    seen = set()
    next_params, token = MODULE.validate_continuation({"eicontinue": "x", "continue": "-||"}, seen, "Creature")
    assert next_params == {"eicontinue": "x", "continue": "-||"} and token not in seen
    seen.add(token)
    expect_error("FANDOM_CONTINUATION_LOOP", lambda: MODULE.validate_continuation({"eicontinue": "x", "continue": "-||"}, seen, "Creature"))

    # Revision tuples require exact timestamps, SHA-1 and a bounded title.
    content = "{{Infobox Creature|name=Demon|health=8200|experience=6000|description=long prose.}}"
    content_bytes = content.encode("utf-8")
    correct_sha1 = MODULE.mediawiki_sha1_base36(content_bytes)
    valid_revision = {"revid": 100, "timestamp": "2026-09-01T00:00:00Z", "sha1": correct_sha1}
    captured = MODULE.validate_fandom_revision(20, "Demon", valid_revision, content, "Creature")
    assert captured["source_sha1"] == correct_sha1 and captured["source_digest"] == MODULE.sha256(content_bytes)
    assert captured["facts"] == {"experience": "6000", "health": "8200", "name": "Demon"}
    assert captured["content_retained"] is False
    expect_error("FANDOM_REVISION_PROVENANCE_INVALID", lambda: MODULE.validate_fandom_revision(20, "Demon", {**valid_revision, "sha1": "X" * 40}, "x", "Creature"))
    expect_error("FANDOM_REVISION_SHA1_CONTENT_MISMATCH", lambda: MODULE.validate_fandom_revision(20, "Demon", {**valid_revision, "sha1": "0"}, content, "Creature"))

    # Exercise accessible enumeration + revision capture end-to-end. This catches
    # local-variable lifetime errors that isolated parser tests would miss.
    revision_body = "{{Infobox Creature|name=Demon|health=8200|experience=6000}}"
    revision_sha1 = MODULE.mediawiki_sha1_base36(revision_body.encode("utf-8"))
    def fake_api(params):
        if params.get("list") == "embeddedin":
            return {"query": {"embeddedin": [{"pageid": 20, "title": "Demon"}]}}, {}
        assert params.get("prop") == "revisions"
        return {"query": {"pages": [{"pageid": 20, "title": "Demon", "revisions": [{"revid": 101, "timestamp": "2026-09-02T00:00:00Z", "sha1": revision_sha1, "slots": {"main": {"content": revision_body}}}]}]}}, {}
    with patch.object(MODULE, "_api_json", side_effect=fake_api), patch.object(MODULE.time, "sleep"):
        captured_family = MODULE.capture_family("Creature")
    assert len(captured_family) == 1
    assert captured_family[0]["revision_id"] == 101 and captured_family[0]["source_sha1"] == revision_sha1

    # Terms need explicit license and attribution signals, not just HTTP 200.
    project_terms = b"<p>All text under Creative Commons Attribution-Share Alike (CC BY-SA); attribution required.</p>"
    license_terms = b"<p>Content license: CC BY-SA. Attribution to authors is required.</p>"
    marker_state = MODULE._license_marker_status(project_terms, license_terms)
    assert marker_state["verified"] is True
    assert MODULE._license_marker_status(project_terms, b"<p>Welcome to Fandom.</p>")["verified"] is False

    api_responses = {
        "siteinfo": json.dumps({"query": {"general": {}}}).encode(),
        "api_help": b"MediaWiki query revisions API help",
        "enumeration_probe": json.dumps({"query": {"embeddedin": []}}).encode(),
        "project_terms": project_terms,
        "license_terms": license_terms,
    }
    def fake_receipt(url, *, token=None, max_bytes=MODULE.MAX_HTTP_BYTES):
        name = next(key for key, value in {"siteinfo": MODULE._api_url({"action": "query", "meta": "siteinfo", "siprop": "general|statistics", "format": "json", "formatversion": "2"}), "api_help": MODULE._api_url({"action": "help", "modules": "query+revisions", "format": "json"}), "enumeration_probe": MODULE._api_url({"action": "query", "list": "embeddedin", "eititle": "Template:Infobox Creature", "eilimit": "1", "format": "json", "formatversion": "2"}), "project_terms": MODULE.FANDOM_ABOUT, "license_terms": MODULE.FANDOM_LICENSE}.items() if value == url)
        body = api_responses[name]
        return ({"url": url, "final_url": url, "state": "HTTP_200", "http_status": 200, "response_bytes": len(body), "response_sha256": MODULE.sha256(body), "rate_limit_headers": {}}, body)
    with patch.object(MODULE, "_request_receipt", side_effect=fake_receipt), patch.object(MODULE.time, "sleep"):
        accessible_terms = MODULE.preflight_fandom()
    assert accessible_terms["state"] == "ACCESSIBLE" and accessible_terms["terms_verified"] is True
    api_responses["license_terms"] = b"<p>Welcome to Fandom.</p>"
    with patch.object(MODULE, "_request_receipt", side_effect=fake_receipt), patch.object(MODULE.time, "sleep"):
        unverified_terms = MODULE.preflight_fandom()
    assert unverified_terms["state"] == "SOURCE_UNAVAILABLE" and unverified_terms["terms_verified"] is False

    # Redirect receipts retain the final URL and reject cross-origin content.
    with patch.object(MODULE, "_http_get", return_value=(b"ok", {}, "https://tibia.fandom.com/about")):
        same_origin_receipt, same_origin_body = MODULE._request_receipt("https://tibia.fandom.com/api.php", max_bytes=16)
    assert same_origin_receipt["state"] == "HTTP_200" and same_origin_body == b"ok"
    with patch.object(MODULE, "_http_get", return_value=(b"untrusted", {}, "https://evil.example/redirect")):
        redirected_receipt, redirected_body = MODULE._request_receipt("https://tibia.fandom.com/api.php", max_bytes=16)
    assert redirected_receipt["state"] == "REDIRECT_ORIGIN_MISMATCH" and redirected_body is None
    api_body = json.dumps({"query": {"general": {}}}).encode()
    api_url = MODULE._api_url({"action": "query", "meta": "siteinfo"})
    with patch.object(MODULE, "_http_get", return_value=(api_body, {}, api_url)):
        assert MODULE._api_json({"action": "query", "meta": "siteinfo"})[0]["query"]
    with patch.object(MODULE, "_http_get", return_value=(api_body, {}, "https://evil.example/api.php")):
        expect_error("FANDOM_API_REDIRECT_ORIGIN_MISMATCH", lambda: MODULE._api_json({"action": "query", "meta": "siteinfo"}))

    # Ability is explicitly only the spell subset; unsupported primary shapes fail closed.
    ability = primary(family="Ability", current_title="Test Spell", source_state="UNSUPPORTED_SOURCE_SHAPE", structured_fields={"name": "Test Spell", "mana": "20"})
    ability_candidate = candidate(family="Ability", title="Test Spell", facts={"mana": "20", "level": "8"})
    unsupported = MODULE.classify_pair(ability, [ability_candidate], "Ability")
    assert unsupported["state"] == "AMBIGUOUS" and unsupported["reason"] == "UNSUPPORTED_PRIMARY_FAMILY_SHAPE"

    # Source namespaces are part of identity; equal page IDs across wikis differ.
    same_decimal = candidate(external_id="10", page_key="mediawiki/tibia.fandom.com/page_id/10")
    assert primary()["external_id"] == same_decimal["external_id"]
    assert primary()["page_key"] != same_decimal["page_key"]
    assert MODULE.FANDOM_TEMPLATES["Ability"] == "Template:Infobox Spell"
    print("G4 non-Item second-source focused self-test: PASS")


if __name__ == "__main__":
    main()
