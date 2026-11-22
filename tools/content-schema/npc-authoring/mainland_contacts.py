"""Lower the qualified mainland/island contact rows into the existing placement family."""

import json
from collections import defaultdict
from pathlib import Path

from chosen_placement_adjustments import choose_cell

CONTACTS = Path(__file__).with_name("mainland_contacts.json")
CRYSTAL_REVISION = "201189f871d23597b82caf88fd611454a2ddefb8"
CRYSTAL_XML = "data-global/world/world-npc.xml"
CRYSTAL_XML_SHA256 = "db07b021856008bce61efe816e8a8eea00a01cb9750f1be5e750093758d1c029"
BR_FACTS = Path("imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json")


def point(cell):
    return dict(zip(("x", "y", "z"), cell, strict=True))


def evidence(kind, row, cell, pages, candidate):
    if kind == "crystal":
        line = row["line"]
        if type(line) is not int or line < 2:
            raise ValueError("Crystal NPC placement requires its exact XML line")
        return {
            "origin": "crystal",
            "repository": "zimbadev/crystalserver",
            "revision": CRYSTAL_REVISION,
            "source_key": "oteryn:source.crystalserver",
            "source_row": f"{CRYSTAL_XML}:{line}",
            "source_sha256": CRYSTAL_XML_SHA256,
            "url": f"https://github.com/zimbadev/crystalserver/blob/{CRYSTAL_REVISION}/{CRYSTAL_XML}#L{line}",
        }
    if kind == "wiki_br":
        page = pages[row["page_id"]]
        if page["removed"] or cell not in page["positions"]:
            raise ValueError(
                f"{row['npc']}: placement does not match the retained BR revision"
            )
        return {
            "origin": "native_wiki_br",
            "repository": "www.tibiawiki.com.br",
            "source_row": str(BR_FACTS),
            "source_sha256": page["sha256"],
            "pageid": page["pageid"],
            "revid": page["revid"],
            "direction_note": "No source direction; north is the project default",
        }
    if not any(
        p["origin"] == "wiki" and p["position"] == point(cell)
        for p in candidate["placements"]
        if "origin" in p
    ):
        raise ValueError(
            f"{row['npc']}: placement does not match the retained Fandom revision"
        )
    return {
        "origin": "wiki",
        "repository": "tibia.fandom.com",
        **candidate["wiki"],
        "direction_note": "No source direction; north is the project default",
    }


def cell_key(cell):
    return cell["x"], cell["y"], cell["floor"]


def crystal_spawns(original):
    spawns = defaultdict(list)
    for record in original:
        row = record["declaration"]
        for source in row["provenance"]:
            if (
                source.get("revision") == CRYSTAL_REVISION
                and source.get("source_sha256") == CRYSTAL_XML_SHA256
                and source.get("source_row", "").partition(":")[0] == CRYSTAL_XML
            ):
                adjustment = next(
                    (
                        p["source_cell"]
                        for p in row["provenance"]
                        if p["origin"] == "oteryn"
                    ),
                    row["cell"],
                )
                key = row["npc"], cell_key(source.get("source_cell", adjustment))
                spawns[key].append(row)
                break
    return spawns


def contacts(admitted, original, candidates, root):
    rows = json.loads(CONTACTS.read_bytes())
    if set(rows) != {"crystal", "wiki_br", "wiki"}:
        raise ValueError("invalid mainland NPC source groups")
    pages = {
        row["pageid"]: row
        for row in json.loads((root / BR_FACTS).read_bytes())["pages"]
    }
    occupied = {
        (d["cell"]["x"], d["cell"]["y"], d["cell"]["floor"]): d
        for row in original
        for d in [row["declaration"]]
    }
    reserved = set(occupied) | {
        tuple(row["cell"]) for group in rows.values() for row in group
    }
    records = {}
    spawns = crystal_spawns(original)
    replaced = set()
    seen_spawns = set()
    for kind, group in rows.items():
        for row in group:
            npc = f"oteryn:npc.{row['npc']}"
            if npc not in admitted:
                raise ValueError(
                    f"{npc}: mainland placement requires an admitted identity"
                )
            cell = tuple(row["cell"])
            spawn = (npc, cell)
            if kind == "crystal":
                if spawn in seen_spawns:
                    raise ValueError(
                        f"{npc}: duplicate mainland Crystal spawn at {cell}"
                    )
                seen_spawns.add(spawn)
            direction = row.get("direction", "north")
            if direction not in ("north", "east", "south", "west"):
                raise ValueError(f"{npc}: invalid NPC facing")
            provenance = [evidence(kind, row, row["cell"], pages, candidates.get(npc))]
            if nearby := row.get("nearby"):
                choice = {
                    "source": point(cell),
                    "target": point(nearby["cell"]),
                    "reason": nearby["reason"],
                }
                target = tuple(nearby["cell"])
                previous = occupied.get(target)
                shareable = previous and (previous["npc"], previous["direction"]) == (
                    npc,
                    direction,
                )
                cell, adjustment = choose_cell(
                    root,
                    cell,
                    choice,
                    reserved - {target} if shareable else reserved,
                    compiler_qualified=True,
                )
                provenance.append(adjustment)
                reserved.add(cell)
            solid_contact = row.get("solid_contact", False)
            # The mainland selection is compiler-qualified. Fold other projections of
            # its exact Crystal spawn into this contact, even when their final cells differ.
            for previous in spawns.get(spawn, ()) if kind == "crystal" else ():
                previous_cell = cell_key(previous["cell"])
                if previous_cell == cell:
                    continue
                if previous["direction"] != direction:
                    raise ValueError(
                        f"{npc}: conflicting facing for Crystal spawn {spawn[1]}"
                    )
                replaced.add(previous["identity"]["key"])
                del occupied[previous_cell]
            if previous := occupied.get(cell):
                if (previous["npc"], previous["direction"]) != (npc, direction):
                    raise ValueError(
                        f"{npc}: NPC contact cell {cell} already occupied by {previous['npc']}"
                    )
                previous["provenance"].extend(
                    row for row in provenance if row not in previous["provenance"]
                )
                if solid_contact:
                    previous["block_solid_contact"] = True
                continue
            x, y, z = cell
            key = f"oteryn:npc_placement.{row['npc']}.x{x}_y{y}_z{z}"
            records[key] = {
                "declaration": {
                    "identity": {"key": key, "revision": "definition-r1"},
                    "npc": npc,
                    "cell": {"x": x, "y": y, "floor": z},
                    "direction": direction,
                    "provenance": provenance,
                    **({"block_solid_contact": True} if solid_contact else {}),
                }
            }
            occupied[cell] = records[key]["declaration"]
    original[:] = [
        record
        for record in original
        if record["declaration"]["identity"]["key"] not in replaced
    ]
    return [records[key] for key in sorted(records)]
