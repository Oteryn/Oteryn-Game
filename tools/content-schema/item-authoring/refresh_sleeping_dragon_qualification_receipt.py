"""Replay successor8 after the Sleeping Dragon spawn ground change."""

from pathlib import Path

import refresh_spkg2_qualification_receipts as previous
from refresh_blood_vestibule_qualification_receipt import MODULE, rebuild_promotion

BASE = "6ed77c6d79779e36778d3c73c8008c6bebcd8819"
RECEIPT = "docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v21.json"
RECEIPT_SHA = "c4c524b1f89629d98c39ac1d12982422c50c728d2268ae5528d13aea4126e42f"


def main():
    previous.ALLOWED.add("tools/content-schema/world-authoring/scoped_spawn_ground.py")
    recipes = list(previous.RECIPES)
    recipes[2] = (*recipes[2][:-1], 4)
    previous.RECIPES = tuple(recipes)
    previous.refresh(
        previous.ROOT,
        (MODULE, "RECEIPT", RECEIPT, RECEIPT_SHA, 22),
        base_head=BASE,
        reason="Sleeping Dragon qualified spawn ground producer update",
        generator=previous.HERE + Path(__file__).name,
        require_new_or_identical=True,
    )
    rebuild_promotion()


if __name__ == "__main__":
    main()
