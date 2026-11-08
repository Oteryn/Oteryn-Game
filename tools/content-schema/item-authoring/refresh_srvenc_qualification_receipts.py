"""Emit combined Soulpit/Sugar successors; preserve all frozen qualification bytes."""

import subprocess
import sys
from pathlib import Path

import refresh_spkg2_qualification_receipts as previous
from historical_world_owner_context import SOULPIT, original_marker


def main():
    original_marker(previous.ROOT)  # Strict independent Soulpit and Sugar witnesses.
    # W07 already published +1. A combined owner footprint gets a fresh +2,
    # never replacing W07's v3/v11/v4/v9 receipts with different bytes.
    recipes = tuple((*recipe[:-1], recipe[-1] + 2) for recipe in previous.RECIPES)
    previous.RECIPES = recipes
    # Mantra is an input of successor8; retain the canonical tuple for proof
    # version lookup, but rebuild producers in dependency order.
    for index in (0, 1, 5, 2, 3, 4, 6):
        recipe = recipes[index]
        previous.refresh(
            previous.ROOT,
            recipe,
            additional_owner_inputs=("content/world/terrain/" + SOULPIT,),
            base_head="98d6ae8ce5e12d60916532c72353126f534fa20a",
            reason="Combined qualified Soulpit and Sugar owner footprints; frozen source facts unchanged",
            generator=previous.HERE + Path(__file__).name,
            require_new_or_identical=True,
            regenerate_owned_successor=True,
        )
        if (
            recipe[0] != "lower_wiki_stack_default_successor8_packet"
            or recipe[1] == "RECEIPT"
        ):
            subprocess.run(
                [
                    sys.executable,
                    str(previous.ROOT / previous.HERE / (recipe[0] + ".py")),
                ],
                cwd=previous.ROOT,
                check=True,
            )
    previous.refresh_consumer_pins(previous.ROOT)


if __name__ == "__main__":
    main()
