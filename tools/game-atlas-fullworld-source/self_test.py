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


def main() -> int:
    test_resolved_delegates()
    test_missing_is_explicit()
    print("game-atlas-fullworld-source self-test: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
