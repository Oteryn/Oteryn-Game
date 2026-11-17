"""Version W07 owner footprints without rewriting frozen qualification receipts."""

import subprocess
import sys
from pathlib import Path

import refresh_spkg2_qualification_receipts as previous
from historical_world_owner_context import SOULPIT, original_marker


def main():
    original_marker(
        previous.ROOT
    )  # Requires the exact qualified W07 marker/shard pair.
    previous.RECIPES = tuple(
        (*recipe[:-1], recipe[-1] + 1) for recipe in previous.RECIPES
    )
    for recipe in previous.RECIPES:
        previous.refresh(
            previous.ROOT,
            recipe,
            additional_owner_inputs=("content/world/terrain/" + SOULPIT,),
            base_head="4620c6b8e12f6a9dd827bff3ca6dbd273ccd5c1f",
            reason="W07 owner footprint; original source records and closed scope unchanged",
            generator=previous.HERE + Path(__file__).name,
        )
        # Later receipts pin these producer outputs. Rebuild in the recipe's
        # dependency order; the successor8 packet needs both its proof and receipt.
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


if __name__ == "__main__":
    main()
