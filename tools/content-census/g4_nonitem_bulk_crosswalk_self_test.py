#!/usr/bin/env python3
"""Focused deterministic tests for the artifact-only non-Item bulk engine."""
import g4_nonitem_bulk_crosswalk as census
from g4_nonitem_bulk_crosswalk import (
    classify_concepts,
    current_provenance_errors,
    parse_infobox,
    _redirect_target,
)


def test_allowlisted_fields_and_no_prose_or_external_appearance_id() -> None:
    source = (
        "{{Predefinição:Infobox Criatura|name=Rat|health=25|experience=5|"
        "speed=130|description=Long copyrighted prose is deliberately omitted."
        "|appearanceid=123|loot={{Infobox Item|name=Rat Tail}}}}"
    )
    state, fields, template = parse_infobox(source, "Creature")
    assert state == "STRUCTURED_FAMILY_INFOBOX"
    assert template == "Predefinição:Infobox Criatura"
    assert fields == {"experience": "5", "health": "25", "name": "Rat", "speed": "130"}


def test_nested_template_separators_do_not_split_parent_fields() -> None:
    source = "{{Infobox Achievement|name=Explorer|points=10|secret=no|description={{Small|story|text}}}}"
    state, fields, template = parse_infobox(source, "Achievement")
    assert state == "STRUCTURED_FAMILY_INFOBOX"
    assert template == "Infobox Achievement"
    assert fields == {"name": "Explorer", "points": "10", "secret": "no"}


def test_zero_or_multiple_family_templates_fail_closed() -> None:
    assert parse_infobox("{{Infobox Item|name=Some item}}", "Creature")[0] == "UNSUPPORTED_SOURCE_SHAPE"
    ambiguous = "{{Infobox Creature|name=Rat}}{{Infobox Creature|name=Other Rat}}"
    assert parse_infobox(ambiguous, "Creature")[0] == "AMBIGUOUS_SOURCE_SHAPE"


def test_redirect_is_not_followed_or_treated_as_definition() -> None:
    assert _redirect_target("  #REDIRECT [[Rat]]\n") == "Rat"
    assert _redirect_target("{{Infobox Creature|name=Rat}}") is None


def test_content_bearing_record_provenance_fails_closed() -> None:
    assert current_provenance_errors("Rat", 123, "2026-09-25T05:00:00Z") == []
    invalid_cases = [
        ("Rat", None, "2026-09-25T05:00:00Z", "INVALID_CURRENT_REVID"),
        ("Rat", True, "2026-09-25T05:00:00Z", "INVALID_CURRENT_REVID"),
        ("Rat", "123", "2026-09-25T05:00:00Z", "INVALID_CURRENT_REVID"),
        ("Rat", 123, None, "INVALID_CURRENT_TIMESTAMP"),
        ("Rat", 123, 20260925, "INVALID_CURRENT_TIMESTAMP"),
        ("Rat", 123, "2026-09-25T05:00:00+00:00", "INVALID_CURRENT_TIMESTAMP"),
        (None, 123, "2026-09-25T05:00:00Z", "INVALID_CURRENT_TITLE"),
        (4, 123, "2026-09-25T05:00:00Z", "INVALID_CURRENT_TITLE"),
        ("  ", 123, "2026-09-25T05:00:00Z", "INVALID_CURRENT_TITLE"),
    ]
    for title, revision_id, timestamp, expected_error in invalid_cases:
        assert expected_error in current_provenance_errors(title, revision_id, timestamp)

    original_api_json, original_sleep = census._api_json, census.time.sleep
    try:
        census.time.sleep = lambda _seconds: None
        invalid_records = [
            ("revid", None, "INVALID_CURRENT_REVID"),
            ("revid", True, "INVALID_CURRENT_REVID"),
            ("timestamp", None, "INVALID_CURRENT_TIMESTAMP"),
            ("timestamp", 20260925, "INVALID_CURRENT_TIMESTAMP"),
            ("title", None, "INVALID_CURRENT_TITLE"),
            ("title", 42, "INVALID_CURRENT_TITLE"),
        ]
        for key, invalid_value, expected_error in invalid_records:
            page = {
                "pageid": 123,
                "title": "Rat",
                "revisions": [{
                    "revid": 456,
                    "timestamp": "2026-09-25T05:00:00Z",
                    "slots": {"main": {"content": "{{Infobox Creature|name=Rat}}"}},
                }],
            }
            if key == "title":
                page["title"] = invalid_value
            elif key == "revid":
                page["revisions"][0][key] = invalid_value
            else:
                page["revisions"][0][key] = invalid_value
            if invalid_value is None:
                if key == "title":
                    page.pop("title")
                else:
                    page["revisions"][0].pop(key)
            census._api_json = lambda _ids, synthetic_page=page: {"query": {"pages": [synthetic_page]}}
            result, errors = census.fetch_current_pages({123: "Creature"})
            assert result[123]["fetch_state"] == "MALFORMED_CURRENT_PROVENANCE"
            assert expected_error in result[123]["provenance_errors"]
            assert "structured_fields" not in result[123]
            assert not errors
    finally:
        census._api_json, census.time.sleep = original_api_json, original_sleep


def test_concept_resolution_uses_two_signals_and_keeps_conflicts() -> None:
    rows = [
        {"family": "Creature", "external_id": "1", "current_title": "Rat", "structured_fields": {"name": "Rat", "health": "25"}},
        {"family": "Creature", "external_id": "2", "current_title": "Rat", "structured_fields": {"name": "Rat", "health": "25"}},
        {"family": "Creature", "external_id": "3", "current_title": "Rat", "structured_fields": {"name": "Rat", "health": "30"}},
        {"family": "NPC", "external_id": "4", "current_title": "Guide", "structured_fields": {"name": "Guide", "occupation": "Guide"}},
        {"family": "NPC", "external_id": "5", "current_title": "Guide", "structured_fields": {"occupation": "Guide"}},
    ]
    classify_concepts(rows)
    assert rows[0]["concept_resolution"] == "CONFLICTING_DUPLICATE_SOURCE_CONCEPT"
    assert rows[1]["concept_resolution"] == "CONFLICTING_DUPLICATE_SOURCE_CONCEPT"
    assert rows[2]["concept_resolution"] == "CONFLICTING_DUPLICATE_SOURCE_CONCEPT"
    assert rows[3]["concept_resolution"] == "UNIQUE_MULTI_SIGNAL_SOURCE_CONCEPT_CANDIDATE"
    assert rows[4]["concept_resolution"] == "UNRESOLVED_MULTI_SIGNAL"


def run() -> None:
    test_allowlisted_fields_and_no_prose_or_external_appearance_id()
    test_nested_template_separators_do_not_split_parent_fields()
    test_zero_or_multiple_family_templates_fail_closed()
    test_redirect_is_not_followed_or_treated_as_definition()
    test_content_bearing_record_provenance_fails_closed()
    test_concept_resolution_uses_two_signals_and_keeps_conflicts()


if __name__ == "__main__":
    run()
    print("G4 Non-Item bulk crosswalk self-test: PASS (6 tests)")
