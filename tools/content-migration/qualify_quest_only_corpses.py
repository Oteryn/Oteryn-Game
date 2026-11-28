"""Qualify captured Grizzly bodies without changing the source decay witnesses."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ORDINARY = ROOT / "imports/crystalserver/ordinary-corpses"
QUARTER = (
    ROOT / "tools/content-schema/native-gameplay/fourth-quarter-corpse-source.json"
)
REASON = "Grizzly quest-only settlement"


def main():
    admissions = ORDINARY / "admissions.json"
    rows = json.loads(admissions.read_bytes())
    scope_file = ORDINARY / "scope.json"
    scope = json.loads(scope_file.read_bytes())
    crocodile = ["crocodile", 6046]
    if crocodile not in rows:
        if scope["excluded"].get("crocodile") != REASON:
            raise ValueError("Missing captured Crocodile admission")
        rows.append(crocodile)
        rows.sort()
    scope["excluded"].pop("crocodile", None)
    admissions.write_text(
        "[\n"
        + ",\n".join(json.dumps(row, separators=(",", ":")) for row in rows)
        + "\n]\n"
    )
    scope_file.write_text(json.dumps(scope, indent=2, ensure_ascii=False) + "\n")

    raw = QUARTER.read_text()
    packet = json.loads(raw)
    key = "oteryn:creature.the_snapper"
    if packet["decay_creatures"][key] != 6046:
        raise ValueError("The Snapper source body changed")
    admitted = packet["admitted"]
    deferred = packet["deferred_admission"]
    if admitted.get(key) != 6046 and deferred.get(key) != REASON:
        raise ValueError("Missing captured Snapper admission")
    admitted[key] = 6046
    deferred.pop(key, None)
    # Keep the packet's compact physical observations in their captured encoding.
    for field in ("admitted", "deferred_admission"):
        start = raw.index("{", raw.index(f'  "{field}":'))
        _, length = json.JSONDecoder().raw_decode(raw[start:])
        encoded = json.dumps(packet[field], indent=2, sort_keys=True)
        raw = raw[:start] + encoded.replace("\n", "\n  ") + raw[start + length :]
    QUARTER.write_text(raw)


if __name__ == "__main__":
    main()
