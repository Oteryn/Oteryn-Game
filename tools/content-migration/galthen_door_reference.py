"""Expose the Galthen gate's two nonportable map appearances as Item references."""

from boss_conditional_item_reference import definition

# Crystal 201189f8 data/libs/tables/doors.lua:251 pairs the closed and open
# forms. Both objects exist in the pinned 15.30 client appearances.
APPEARANCES = (36547, 36548)


def extend_records(records):
    existing = {row["identity"]["key"]: row for row in records}
    for appearance in APPEARANCES:
        key = f"oteryn:item.tibia.i{appearance}"
        expected = definition(key)
        if key not in existing:
            records.append(expected)
        elif existing[key] != expected:
            raise ValueError(f"Galthen door reference changed: {key}")
