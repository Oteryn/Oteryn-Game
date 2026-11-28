"""Read indexed row selection and requirements from the Rust dispatch source."""
import os
from pathlib import Path
import re

PHASES = Path(__file__).resolve().parents[3] / "apps/game-server/src/qualification/client_phases"


def selected_rows():
    groups = os.environ.get("NODE_BOOT_E2E_ONLY", "").strip()
    if groups and "rows" not in {group.strip() for group in groups.split(",")}:
        return {}
    batch = os.environ.get("OTERYN_E2E_ROW_BATCH")
    if batch is None:
        default = (PHASES / "e2e_row_batch.rs").read_text()
        start, length = map(int, re.search(
            r"E2E_ROW_BATCH_DEFAULT: .*? = \((\d+), (\d+)\);", default
        ).groups())
    elif batch == "all":
        start, length = 0, 2**64 - 1
    else:
        match = re.fullmatch(r"\+?([0-9]+):\+?([0-9]+)", batch)
        if not match:
            return {}
        values = [value.lstrip("0") or "0" for value in match.groups()]
        if any(len(value) > 20 for value in values):
            return {}
        start, length = map(int, values)
        if max(start, length) >= 2**64:
            return {}

    selected = {}
    dispatch = (PHASES / "world.rs").read_text()
    for index, premium, scenario, direct_client in re.findall(
        r'e2e_row_selected\((\d+)\) \{\s*'
        r'(// Requires current Premium\.\s*)?'
        r'(?:verdict::indexed\("([^"]+)"|client_row_(\w+)::run_client_packet)',
        dispatch,
    ):
        if start <= int(index) < start + length:
            scenario = scenario or (direct_client[0].upper() + direct_client[1:] + ".client")
            selected[scenario] = bool(premium)
    return selected


if __name__ == "__main__":
    print(int(any(selected_rows().values())))
