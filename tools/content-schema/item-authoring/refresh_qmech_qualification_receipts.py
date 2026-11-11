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
        "docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v14.json",
        "ea1a9eabe07fa69884e56f27b2ea47e71b5b089df9493a56a47dce8de6035a67",
        15,
    )
    previous.refresh(
        previous.ROOT,
        recipe,
        base_head="d75ade6cc7ceb0d2c8aa2e0a3b0f39d8e74e0c2f",
        reason="QMECH preserves the pre-existing i23049 Item-to-Terrain route; complete Item input footprint refreshed with frozen source facts unchanged",
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
