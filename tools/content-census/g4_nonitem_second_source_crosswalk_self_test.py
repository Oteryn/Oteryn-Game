#!/usr/bin/env python3
"""Synthetic, network-free regression tests for G4 second-source evidence."""
from __future__ import annotations

import importlib.util
from pathlib import Path

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
    valid_revision = {"revid": 100, "timestamp": "2026-09-01T00:00:00Z", "sha1": "a" * 40}
    captured = MODULE.validate_fandom_revision(20, "Demon", valid_revision, "{{Infobox Creature|name=Demon|health=8200|experience=6000|description=long prose.}}", "Creature")
    assert captured["source_sha1"] == "a" * 40 and captured["source_digest"] == MODULE.sha256(b"{{Infobox Creature|name=Demon|health=8200|experience=6000|description=long prose.}}")
    assert captured["facts"] == {"experience": "6000", "health": "8200", "name": "Demon"}
    assert captured["content_retained"] is False
    expect_error("FANDOM_REVISION_PROVENANCE_INVALID", lambda: MODULE.validate_fandom_revision(20, "Demon", {**valid_revision, "sha1": "abc"}, "x", "Creature"))

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
