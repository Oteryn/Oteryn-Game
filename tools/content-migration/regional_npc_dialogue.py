"""Install the regional NPC dialogue facts before canonical WorldProject serialization."""

import json

from questpc7_world_seal import refresh


def install(root, package):
    source = json.loads(
        (root / "content/dialogues/regional.authoring.json").read_bytes()
    )
    path = package / "definitions/declarations.json"
    document = json.loads(path.read_bytes())
    records = document["records"]
    by_key = {row["identity"]["key"]: row for row in records}
    family_order = list(dict.fromkeys(row["kind"] for row in records))
    changed = False
    for name, row in source["npcs"].items():
        key = f"oteryn:npc.{name}"
        npc = by_key[key]
        if npc["kind"] != "NPC":
            raise ValueError(f"{key}: expected NPC")
        if npc.get("dialogue") is None:
            identity = {
                "key": f"oteryn:dialogue.npc.{name}",
                "revision": npc["identity"]["revision"],
            }
            if identity["key"] in by_key:
                raise ValueError(f"{key}: existing unbound dialogue")
            npc["dialogue"] = {"family": "Dialogue", **identity}
            dialogue = {"kind": "Dialogue", "identity": identity, "fields": []}
            records.append(dialogue)
            by_key[identity["key"]] = dialogue
        else:
            dialogue = by_key[npc["dialogue"]["key"]]
        provenance = {
            "field_path": "oteryn:source.npc.regional_dialogue",
            "value": {
                "type": "Text",
                "value": f"{source['repository']}@{source['revision']}:data-global/npc/"
                f"{row['source']} (sha256 {row['sha256']})",
            },
        }
        installed = [
            field
            for field in dialogue["fields"]
            if field["field_path"] == provenance["field_path"]
        ]
        if installed:
            keywords = {
                keyword["key"]: keyword for keyword in dialogue.get("keywords", [])
            }
            if (
                installed != [provenance]
                or any(
                    dialogue.get(field) != lines
                    for field, lines in row["fields"].items()
                )
                or any(
                    keywords.get(keyword["key"]) != keyword
                    for keyword in row["keywords"]
                )
            ):
                raise ValueError(f"{key}: installed regional dialogue has drifted")
            continue
        for field, lines in row["fields"].items():
            if field not in {
                "greet",
                "farewell",
                "walkaway",
                "send_trade",
            } or dialogue.get(field):
                raise ValueError(f"{key}: unexpected existing dialogue field {field}")
            dialogue[field] = lines
        if row["keywords"]:
            keywords = dialogue.setdefault("keywords", [])
            keys = {keyword["key"] for keyword in keywords}
            triggers = {
                trigger for keyword in keywords for trigger in keyword["triggers"]
            }
            for keyword in row["keywords"]:
                if keyword["key"] in keys or triggers.intersection(keyword["triggers"]):
                    raise ValueError(
                        f"{key}: duplicate dialogue keyword {keyword['key']}"
                    )
                keywords.append(keyword)
                keys.add(keyword["key"])
                triggers.update(keyword["triggers"])
        dialogue["fields"].append(provenance)
        changed = True
    if not changed:
        return
    records.sort(
        key=lambda row: (family_order.index(row["kind"]), row["identity"]["key"])
    )
    path.write_text(
        json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    refresh(root, package)
