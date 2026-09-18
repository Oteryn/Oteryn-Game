#!/usr/bin/env python3
from __future__ import annotations

import hashlib
from types import SimpleNamespace

import producer


class Tile:
    def __init__(self, x: int, y: int, z: int, items: tuple[object, ...]):
        self.position = SimpleNamespace(x=x, y=y, z=z)
        self.ground = None
        self.items = items


class FakeBounded:
    def __init__(self) -> None:
        self.delegated = 0

    def _tile_record(self, tile, *, appearances, sheets, sheet_for_sprite):
        self.delegated += 1
        return {"delegated": True}, {"presentation_count": 1, "primitive_count": 1, "appearance_ids": {1}, "sprite_ids": {99}}

    @staticmethod
    def _stable_id(domain: str, *parts: object) -> str:
        payload = "\0".join([domain, *(str(part) for part in parts)]).encode()
        return f"{domain}:{hashlib.sha256(payload).hexdigest()[:32]}"


def runtime(appearances):
    bounded = FakeBounded()
    legacy = SimpleNamespace(Tile=Tile)
    return producer.Runtime(None, None, None, None, bounded, legacy, appearances, [], None), bounded


def test_resolved_delegates() -> None:
    rt, bounded = runtime({1: SimpleNamespace(hook_direction=None)})
    tile = Tile(10, 20, 7, (SimpleNamespace(server_id=1),))
    record, stats = producer.project_tile(rt, tile)
    assert record == {"delegated": True}
    assert stats["primitive_count"] == 1
    assert bounded.delegated == 1


def test_missing_is_explicit() -> None:
    rt, bounded = runtime({})
    item = SimpleNamespace(server_id=2141)
    tile = Tile(33572, 32528, 14, (item,))
    record, stats = producer.project_tile(rt, tile)
    assert bounded.delegated == 0
    assert record["position"] == {"floor": -14, "x": 33572, "y": 32528}
    presentation = record["presentation"][0]
    assert presentation["appearance_source_id"] == 2141
    assert presentation["presentation_resolution_state"] == "UNRESOLVED_APPEARANCE"
    assert presentation["resolved_primitives"] == []
    assert stats["unresolved_appearance_ids"] == {2141}
    assert stats["unresolved_presentation_count"] == 1


def test_explicit_source_identity_binding() -> None:
    presentation = {
        "export_record_id": "presentation:abc",
        "appearance_source_id": 2141,
        "source_role": "tile_item",
        "presentation_order": {"plane": 0, "order": 3},
        "canonical_entity_id": None,
        "entity_identity_state": "UNRESOLVED",
    }
    binding = producer.SourceIdentityBinding(
        export_record_id="presentation:abc",
        appearance_source_id=2141,
        definition_family="LOCAL_OBJECT",
        production_key="oteryn:reference.object.test-door",
        definition_revision="definition-r1",
        placement_key="oteryn:reference.placement.test-door",
    )
    adapted = producer.adapt_presentation_source_identity(presentation, binding)
    assert adapted["identity_disposition"] == "EXPLICITLY_BOUND"
    assert adapted["typed_definition_ref"] == {
        "definition_family": "LOCAL_OBJECT",
        "production_key": "oteryn:reference.object.test-door",
        "definition_revision": "definition-r1",
    }
    assert adapted["placement_key"] == "oteryn:reference.placement.test-door"
    assert presentation["canonical_entity_id"] is None
    assert presentation["entity_identity_state"] == "UNRESOLVED"


def test_unbound_source_identity_stays_unresolved() -> None:
    presentation = {
        "export_record_id": "presentation:def",
        "appearance_source_id": 3687,
        "source_role": "tile_item",
        "presentation_order": {"plane": 0, "order": 1},
    }
    adapted = producer.adapt_presentation_source_identity(presentation, None)
    assert adapted["identity_disposition"] == "UNRESOLVED_SOURCE_IDENTITY"
    assert adapted["typed_definition_ref"] is None
    assert adapted["placement_key"] is None


def test_binding_mismatch_fails_closed() -> None:
    presentation = {
        "export_record_id": "presentation:abc",
        "appearance_source_id": 2141,
        "source_role": "tile_item",
        "presentation_order": {"plane": 0, "order": 0},
    }
    mismatched = producer.SourceIdentityBinding(
        export_record_id="presentation:other",
        appearance_source_id=2141,
        definition_family="LOCAL_OBJECT",
        production_key="oteryn:reference.object.test-door",
        definition_revision="definition-r1",
        placement_key="oteryn:reference.placement.test-door",
    )
    try:
        producer.adapt_presentation_source_identity(presentation, mismatched)
    except producer.ProducerError as exc:
        assert "export_record_id mismatch" in str(exc)
    else:
        raise AssertionError("mismatched source occurrence was accepted")


def test_tile_adapter_rejects_stale_binding() -> None:
    record = {
        "record_type": "tile",
        "presentation": [
            {
                "export_record_id": "presentation:abc",
                "appearance_source_id": 2141,
                "source_role": "tile_item",
                "presentation_order": {"plane": 0, "order": 0},
            }
        ],
    }
    stale = producer.SourceIdentityBinding(
        export_record_id="presentation:stale",
        appearance_source_id=2141,
        definition_family="LOCAL_OBJECT",
        production_key="oteryn:reference.object.test-door",
        definition_revision="definition-r1",
        placement_key="oteryn:reference.placement.test-door",
    )
    try:
        producer.adapt_tile_source_identities(record, {"presentation:stale": stale})
    except producer.ProducerError as exc:
        assert "does not match tile occurrence" in str(exc)
    else:
        raise AssertionError("stale source binding was accepted")


def main() -> int:
    test_resolved_delegates()
    test_missing_is_explicit()
    test_explicit_source_identity_binding()
    test_unbound_source_identity_stays_unresolved()
    test_binding_mismatch_fails_closed()
    test_tile_adapter_rejects_stale_binding()
    print("game-atlas-fullworld-source self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
