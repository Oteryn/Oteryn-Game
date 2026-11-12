#!/usr/bin/env python3
"""Regenerate accepted G-8 owner-only interval wire fixtures (compact UTF-8)."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "docs/contracts/fixtures/public-projections"
CHARACTER = "01934f10-7c04-7001-805b-3b1122334401"
STATISTICS = "01934f10-7c08-7001-805b-3b1122334401"
WORLD = "01934f10-7c02-7001-805b-3b1122334401"


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name, revision, observed, sessions in [
        ("open", "1", "1790000000", [{"world_id": WORLD, "started_at": "1790000000", "ended_at": []}]),
        ("closed", "2", "1790003600", [{"world_id": WORLD, "started_at": "1790000000", "ended_at": ["1790003600"]}]),
        ("withdrawn", "3", "1790003700", []),
    ]:
        wire = {"contract_version": 2, "operation": "PublishCharacterOnlineSessionV2",
                "source_authority": "oteryn:game:primary", "character_id": CHARACTER,
                "statistics_id": STATISTICS, "projection_epoch": "1",
                "projection_revision": revision, "source_observed_at": observed,
                "sessions": sessions}
        (OUT / f"character_online_session_{name}_v2.json").write_bytes(
            json.dumps(wire, separators=(",", ":"), ensure_ascii=False).encode())

    for name, end in [("invalid_null_end", None), ("invalid_end_before_start", ["1789999999"]),
                      ("invalid_future_end", ["1790000001"])]:
        wire = {"contract_version": 2, "operation": "PublishCharacterOnlineSessionV2",
                "source_authority": "oteryn:game:primary", "character_id": CHARACTER,
                "statistics_id": STATISTICS, "projection_epoch": "1",
                "projection_revision": "1", "source_observed_at": "1790000000",
                "sessions": [{"world_id": WORLD, "started_at": "1790000000", "ended_at": end}]}
        (OUT / f"character_online_session_{name}_v2.json").write_bytes(
            json.dumps(wire, separators=(",", ":"), ensure_ascii=False).encode())


if __name__ == "__main__":
    main()
