"""Reissue the QMECH current-parent receipt without rewriting frozen source proofs."""

import subprocess
import sys
from pathlib import Path

import refresh_spkg2_qualification_receipts as previous


def main():
    recipes = list(previous.RECIPES)
    # The current receipt retains this exact source proof, not the refresher's
    # original v2 target. Only the receipt recipe below is regenerated.
    recipes[2] = (
        "lower_wiki_stack_default_successor8_packet",
        "PROOF",
        "docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-source-qualification-v4.json",
        "f9465526c2c133f436a414bd9702df23aeb3739b0d9023f718407108bec163b7",
        4,
    )
    previous.RECIPES = tuple(recipes)
    recipe = (
        "lower_wiki_stack_default_successor8_packet",
        "RECEIPT",
        "docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v12.json",
        "3710a8004013da875328166385eab161e8e806840d9f5a2d76b09dec7960013c",
        13,
    )
    previous.refresh(
        previous.ROOT,
        recipe,
        base_head="e37b636da17a65c1e3f45616fd7f4aef84416d0b",
        reason="QMECH terrain owner restore and input footprint refresh; frozen source facts unchanged",
        generator=previous.HERE + Path(__file__).name,
        require_new_or_identical=True,
    )
    subprocess.run(
        [sys.executable, str(previous.ROOT / previous.HERE / (recipe[0] + ".py"))],
        cwd=previous.ROOT,
        check=True,
    )


if __name__ == "__main__":
    main()
