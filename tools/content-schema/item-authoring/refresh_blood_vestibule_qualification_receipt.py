"""Refresh the successor8 current-parent receipt for the Blood Vestibule ground."""

import re
import subprocess
import sys
from pathlib import Path

import refresh_spkg2_qualification_receipts as previous

BASELINE = "d9fcedeb82d8fc930ba46576fc12c1c98fbe0687"
MODULE = "lower_wiki_stack_default_successor8_packet"
SHARD = "content/world/terrain/terrain-08886-08887.json"


def rebuild_promotion():
    subprocess.run(
        [sys.executable, previous.HERE + MODULE + ".py"], cwd=previous.ROOT, check=True
    )
    packet = previous.ROOT / (
        "docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-promotion-v1.json"
    )
    source = previous.ROOT / (
        "crates/gameplay/content-promotion/src/content/item_stack_default_successor8_promotion.rs"
    )
    text, count = re.subn(
        r'(ITEM_STACK_DEFAULT_SUCCESSOR8_PACKET_SHA256: &str =\s*")[a-f0-9]{64}(")',
        lambda match: match[1] + previous.digest(packet.read_bytes()) + match[2],
        source.read_text(),
    )
    if count != 1:
        raise ValueError("expected one successor8 consumer pin")
    source.write_text(text)


def main():
    proof = "docs/agents/evidence/OTV2-20261002-item-stack-default-successor8-source-qualification-v4.json"
    receipt = "docs/agents/evidence/OTV2-20261003-item-stack-default-successor8-current-parent-receipt-v17.json"
    proof_pin, receipt_pin = (
        previous.digest(
            subprocess.check_output(
                ["git", "show", f"{BASELINE}:{path}"], cwd=previous.ROOT
            )
        )
        for path in (proof, receipt)
    )
    recipes = list(previous.RECIPES)
    recipes[2] = (MODULE, "PROOF", proof, proof_pin, 4)
    previous.RECIPES = tuple(recipes)
    previous.refresh(
        previous.ROOT,
        (MODULE, "RECEIPT", receipt, receipt_pin, 18),
        additional_owner_inputs=(SHARD,),
        base_head=BASELINE,
        reason="Blood Vestibule adds qualified appearance-only ground and refreshes Item provenance",
        generator=previous.HERE + Path(__file__).name,
        require_new_or_identical=True,
        regenerate_owned_successor=True,
    )
    rebuild_promotion()


if __name__ == "__main__":
    main()
