"""Replace one NPC's offers without moving other producers' rows."""


def replace_npc_offers(rows, npc, replacements):
    if any(row["npc"] != npc for row in replacements):
        raise ValueError("replacement offer belongs to another NPC")
    result = []
    inserted = False
    for row in rows:
        if row["npc"] != npc:
            result.append(row)
        elif not inserted:
            result.extend(replacements)
            inserted = True
    if not inserted:
        result.extend(replacements)
    return result
