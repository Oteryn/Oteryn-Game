"""Refresh the Rust pin for the packet emitted by lower_wiki_stats_packet.py."""

import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / "docs/agents/evidence/OTV2-20260930-item-stats-promotion-v2.json"
SOURCE = ROOT / "crates/gameplay/content-promotion/src/content/item_stats_promotion.rs"


def main() -> None:
    raw = PACKET.read_bytes()
    counts = json.loads(raw)["counts"]
    replacements = {
        "ITEM_STATS_PROMOTION_V2_PACKET_SHA256": f'"{hashlib.sha256(raw).hexdigest()}"',
        "ITEM_STATS_PROMOTION_V2_FIELD_COUNT": f"{counts['fields']:_}",
        "ITEM_STATS_PROMOTION_V2_ITEM_COUNT": f"{counts['items']:_}",
    }
    text = SOURCE.read_text()
    for name, value in replacements.items():
        pattern = rf"(pub const {name}: [^=]+?=\s*)(?:\"[0-9a-f]{{64}}\"|[0-9_]+)(;)"
        text, count = re.subn(
            pattern, lambda match, value=value: match[1] + value + match[2], text
        )
        if count != 1:
            raise ValueError(f"expected one Rust packet pin: {name}")
    SOURCE.write_text(text)


if __name__ == "__main__":
    main()
