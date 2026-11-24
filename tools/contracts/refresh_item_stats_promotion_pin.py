"""Refresh Rust pins for the Wiki stats packet and its linked Item packets."""

import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EVIDENCE = ROOT / "docs/agents/evidence"
PROMOTION = ROOT / "crates/gameplay/content-promotion/src/content"


def refresh(source: Path, replacements: dict[str, str]) -> None:
    text = source.read_text()
    for name, value in replacements.items():
        pattern = rf"(pub const {name}: [^=]+?=\s*)(?:\"[0-9a-f]{{64}}\"|[0-9_]+)(;)"
        text, count = re.subn(
            pattern, lambda match, value=value: match[1] + value + match[2], text
        )
        if count != 1:
            raise ValueError(f"expected one Rust packet pin: {name}")
    source.write_text(text)


def main() -> None:
    stats = (EVIDENCE / "OTV2-20260930-item-stats-promotion-v2.json").read_bytes()
    counts = json.loads(stats)["counts"]
    refresh(
        PROMOTION / "item_stats_promotion.rs",
        {
            "ITEM_STATS_PROMOTION_V2_PACKET_SHA256": f'"{hashlib.sha256(stats).hexdigest()}"',
            "ITEM_STATS_PROMOTION_V2_FIELD_COUNT": f"{counts['fields']:_}",
            "ITEM_STATS_PROMOTION_V2_ITEM_COUNT": f"{counts['items']:_}",
        },
    )
    for packet, source, pin in (
        (
            "OTV2-20261002-item-mantra-bond-promotion-v1.json",
            "item_mantra_bond_modifier_promotion.rs",
            "ITEM_MANTRA_BOND_MODIFIER_PACKET_SHA256",
        ),
        (
            "OTV2-20261002-item-stack-default-successor8-promotion-v1.json",
            "item_stack_default_successor8_promotion.rs",
            "ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET_SHA256",
        ),
    ):
        raw = (EVIDENCE / packet).read_bytes()
        refresh(PROMOTION / source, {pin: f'"{hashlib.sha256(raw).hexdigest()}"'})


if __name__ == "__main__":
    main()
