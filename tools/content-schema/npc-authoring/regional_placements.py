"""Apply regional Crystal anchors and exact retained TibiaWiki fallback positions."""

import hashlib
import json
import re
from pathlib import Path

SELECTION = Path(__file__).with_suffix(".json")
WIKI = Path("imports/tibiawiki/npc-br/2026-09-28/tibiawiki-br-npc-facts.json")
REPOSITORY = "zimbadev/crystalserver"
DIRECTIONS = ("NORTH", "EAST", "SOUTH", "WEST")


def normalize(name):
    return re.sub(r"[^a-z0-9]", "", name.lower())


def apply(candidates, donor_candidates, scope, admitted, root):
    document = json.loads(SELECTION.read_text())
    source = document["crystal_source"]
    if (source["origin"] != "crystal"
            or source["path"] != "data-global/world/world-npc.xml"
            or not re.fullmatch(r"[0-9a-f]{40}", source["revision"])
            or not re.fullmatch(r"[0-9a-f]{64}", source["sha256"])):
        raise ValueError("invalid regional Crystal source pin")
    crystal_url = f"https://github.com/{REPOSITORY}/blob/{source['revision']}/{source['path']}"
    wiki_bytes = (root / WIKI).read_bytes()
    wiki = {}
    for page in json.loads(wiki_bytes)["pages"]:
        wiki.setdefault(normalize(page["title"]), []).append(page)
    wiki_digest = hashlib.sha256(wiki_bytes).hexdigest()
    earlier = set(scope["npcs"])
    seen = set()
    regional = {}
    for row in document["placements"]:
        key = f"oteryn:npc.{row['npc']}"
        cell = tuple(row["cell"])
        if (key not in admitted or cell in seen or len(cell) != 3
                or any(type(value) is not int for value in cell)
                or not all(0 <= value <= limit for value, limit in zip(cell, (65535, 65535, 15)))):
            raise ValueError(f"{key}: invalid or duplicate regional placement {cell}")
        seen.add(cell)
        if line := row.get("xml_line"):
            if type(line) is not int or line < 2:
                raise ValueError(f"{key}: invalid Crystal XML source row")
            evidence = {
                "origin": "crystal", "repository": REPOSITORY, "revision": source["revision"],
                "source_key": "oteryn:source.crystalserver",
                "source_row": f"{source['path']}:{line}", "source_sha256": source["sha256"],
                "url": f"{crystal_url}#L{line}",
            }
        else:
            pages = [page for page in wiki.get(normalize(row["npc"]), [])
                     if list(cell) in page["positions"]]
            if len(pages) != 1:
                raise ValueError(f"{key}: regional fallback requires one retained Wiki page")
            page = pages[0]
            evidence = {
                "origin": "native_wiki_br", "repository": "www.tibiawiki.com.br",
                "source_row": f"{WIKI}#{page['title']}", "source_sha256": wiki_digest,
                "pageid": page["pageid"], "revid": page["revid"],
            }
        if alternate := row.get("supersedes"):
            alternate_key = f"oteryn:npc.{alternate}"
            position = dict(zip(("x", "y", "z"), cell))
            original = donor_candidates[alternate_key]["placements"]
            if alternate_key in earlier or not any(p["position"] == position for p in original):
                raise ValueError(f"{key}: no unscoped alternative at {cell}")
            candidates[alternate_key]["placements"] = [
                p for p in candidates[alternate_key]["placements"] if p["position"] != position
            ]
            evidence["supersedes_placement_hypothesis"] = alternate_key
        direction = row.get("direction", 0)
        if type(direction) is not int or not 0 <= direction < len(DIRECTIONS):
            raise ValueError(f"{key}: invalid regional direction")
        if "direction" not in row:
            evidence["direction_note"] = "The source leaves facing unspecified; use north."
        if adjustment := row.get("adjacent"):
            target = tuple(adjustment["cell"])
            if (not line or target in seen or len(target) != 3 or target[2] != cell[2]
                    or sum(abs(a - b) for a, b in zip(cell[:2], target[:2])) != 1
                    or not adjustment["reason"]):
                raise ValueError(f"{key}: invalid adjacent contact for {cell}")
            evidence["source_cell"] = dict(zip(("x", "y", "floor"), cell))
            evidence["arbitration"] = [{
                "fact": "placements", "chosen": "oteryn", "rule": "CHOSEN_OTERYN_APPROXIMATION",
                "basis": adjustment["reason"],
            }]
            cell = target
            seen.add(cell)
        candidate = regional.setdefault(key, {"identity": {"key": key}, "placements": []})
        candidate["placements"].append({
            "position": dict(zip(("x", "y", "z"), cell)),
            "direction": DIRECTIONS[direction],
            "regional_provenance": evidence,
            "block_solid_contact": row.get("block_solid_contact", False),
        })
    for key, candidate in regional.items():
        if key in earlier:
            candidates[key]["placements"].extend(candidate["placements"])
        else:
            scope["npcs"].append(key)
            candidates[key] = candidate
