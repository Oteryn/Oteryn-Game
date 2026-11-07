"""Extend an admitted catalogue without replacing its identities, mounts or palette."""

import copy


def merge(base, supplement, tooling):
    result = copy.deepcopy(base)
    target = result["document"]
    incoming = supplement["document"]
    for field in ("source_manifest_sha256", "source_appearances_sha256"):
        if target[field] != incoming[field]:
            raise ValueError("cosmetics supplement appearance source differs")
    if target["palette"]["colours"] != incoming["palette"]["colours"]:
        raise ValueError("cosmetics supplement palette differs")
    outfits = {row["key"]: row for row in target["outfits"]}
    pairs = {}
    for row in outfits.values():
        pair = (row["female"]["id"], row["male"]["id"])
        pairs.setdefault(pair, []).append(row)
    aliases = {}
    for row in incoming["outfits"]:
        pair = (row["female"]["id"], row["male"]["id"])
        admitted = outfits.get(row["key"])
        if admitted is None:
            candidates = pairs.get(pair, [])
            if len(candidates) > 1:
                raise ValueError("cosmetics supplement has ambiguous admitted identities")
            admitted = next(iter(candidates), None)
        if admitted is not None:
            if (any(admitted[sex] != row[sex] for sex in ("female", "male"))
                    or admitted["addon_count"] < row["addon_count"]):
                raise ValueError("cosmetics supplement conflicts with admitted appearance")
            aliases[row["key"]] = admitted["key"]
        else:
            if row["key"] in outfits:
                raise ValueError("cosmetics supplement retargets an admitted identity")
            admitted = copy.deepcopy(row)
            outfits[row["key"]] = admitted
            pairs.setdefault(pair, []).append(admitted)
            aliases[row["key"]] = row["key"]
    target["outfits"] = sorted(outfits.values(), key=lambda row: row["key"])
    grants = {row["grant_key"]: row for row in target.get("grant_mappings", [])}
    for original in incoming.get("grant_mappings", []):
        row = copy.deepcopy(original)
        if "outfit_key" in row["target"]:
            row["target"]["outfit_key"] = aliases[row["target"]["outfit_key"]]
        elif row["target"].get("kind") == "Mount":
            mount = row["target"].get("mount_key")
            if not any(admitted["key"] == mount for admitted in target["mounts"]):
                raise ValueError("cosmetics supplement references an unadmitted mount")
        else:
            raise ValueError("cosmetics supplement has an unknown grant target")
        existing = grants.get(row["grant_key"])
        if existing is not None:
            if existing["target"] != row["target"]:
                raise ValueError("cosmetics supplement retargets an admitted grant")
            existing["compatible_provenance"] = sorted(set(
                existing["compatible_provenance"] + row["compatible_provenance"]))
        else:
            grants[row["grant_key"]] = row
    target["grant_mappings"] = sorted(grants.values(), key=lambda row: row["grant_key"])
    target["content_revision"] = incoming["content_revision"]
    prior = {row["path"]: row for row in base["promotions"]}
    promotions = []
    for path in sorted(tooling.promotion_paths(target)):
        value = tooling.pointer(target, path)
        if path in prior and prior[path]["value"] == value:
            promotions.append(copy.deepcopy(prior[path]))
            continue
        if path.startswith("/outfits/"):
            source = target["outfits"][int(path.split("/")[2])]["source"]
        else:
            source = supplement["promotions"][0]["source"]
        promotions.append({"path": path, "value": value, "state": "PROMOTED",
                           "decision": "owner:QENGINE-2:canonical-donor-outfit-and-atomic-quest-grants:2026-11-06",
                           "source_kind": "OWNER_DECISION" if path == "/grant_mappings" else "STRUCTURED_REFERENCE_DATA",
                           "source": copy.deepcopy(source)})
    result["promotions"] = promotions
    return result
