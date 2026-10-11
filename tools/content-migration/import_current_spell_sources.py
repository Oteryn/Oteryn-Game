"""Import reviewed current-source bindings and supported ordinary spell data.

Canonical identities, native profiles and exclusions stay with the current runtime.
Partial source imports retain their missing-mechanic records.
"""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INPUT = "content/abilities/source-imports/current-sources.json"
# This receipt was qualified against immutable donor bytes and the current model.
# Refreshing it requires repeating source and model qualification, then review.
INPUT_SHA256 = "f27fbdff9ed45b5c803e871b2ef154cb0ac21d5582baa29c39389c61a9248f4f"
CATALOG = "content/abilities/definitions/player-spells.json"
SELECTION = "content/abilities/definitions/player-spell-selection.json"
MANIFEST = "content/spells.manifest.json"
CREATURE_PROFILES = "content/creatures/definitions/spell-native-profiles.json"
CLASSES = {"ADAPTER_MISSING", "CONTRACT_GAP", "RUNTIME_GAP", "SOURCE_CONFLICT_KEPT", "EXCLUDED"}


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n").encode()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def payload(entry):
    return digest(encoded({key: entry[key] for key in ("bundle", "dependencies", "catalog")}))


def adapter(root):
    path = root / "tools/content-schema/spell-authoring/current_spell_adapter.py"
    spec = importlib.util.spec_from_file_location("current_spell_adapter", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def partial_native(binding, entry, held, source_selection):
    """Replay the closed native authoring gate against the immutable baseline."""
    q = binding["qualification"]
    receipt = q.get("source_receipt")
    source = binding["source"]
    if binding["projection"] != "identity_only" or \
            q.get("accepted_native_profile") != {"bundle": entry["bundle"], "dependencies": entry["dependencies"]} or \
            q.get("normalized_controller_equality") is not True or \
            q.get("whole_source_controller_equivalence") is not False or \
            q.get("raw_source_parity") is not False or q.get("runtime_activation") is not False or \
            not isinstance(q.get("remaining_source_scope"), list) or not q["remaining_source_scope"] or \
            not isinstance(receipt, dict) or digest(encoded(receipt)) != q.get("source_receipt_sha256") or \
            not isinstance(receipt.get("raw_source_model"), dict) or \
            set(receipt["raw_source_model"]) != {"bundle", "dependencies", "catalog"} or \
            receipt.get("scope") != "accepted_partial_native_profile" or \
            any(receipt.get(flag) is not q.get(flag) for flag in (
                "normalized_controller_equality", "whole_source_controller_equivalence",
                "raw_source_parity", "runtime_activation")) or \
            receipt.get("partial_source_scope") != q["remaining_source_scope"] or \
            held.get("target") != binding["target"] or \
            held.get("carrier") != entry["bundle"]["spell"]["carrier"] or \
            held.get("scope") != "donor_native_controller_and_lifecycle" or \
            held.get("source_scope") != q["remaining_source_scope"] or \
            held.get("source_model_sha256") != q.get("source_model_sha256") or \
            held.get("source_receipt_sha256") != q.get("source_receipt_sha256") or \
            any(held.get(field) != source.get(field) for field in ("source_id", "path", "revision", "sha256")):
        raise ValueError("CURRENT_SOURCE_PARTIAL_NATIVE_SCOPE")
    raw = receipt["raw_source_model"]
    if held.get("name", "").casefold() != raw.get("bundle", {}).get("spell", {}).get("name", "").casefold():
        raise ValueError("CURRENT_SOURCE_PARTIAL_NATIVE_SCOPE")
    path = ROOT / "tools/content-schema/spell-authoring/native_source_projection.py"
    spec = importlib.util.spec_from_file_location("_current_native_projection", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    try:
        bundle, dependencies, reproduced = module.project_native_controller_candidate(
            raw["bundle"], raw["dependencies"], entry, binding["target"], raw["catalog"], source_selection)
    except (ValueError, KeyError, TypeError) as error:
        raise ValueError("CURRENT_SOURCE_PARTIAL_NATIVE_SCOPE:" + str(error)) from error
    if bundle != entry["bundle"] or dependencies != entry["dependencies"] or reproduced != receipt:
        raise ValueError("CURRENT_SOURCE_PARTIAL_NATIVE_SCOPE")


def monster_melee(data, generated, donors):
    if set(data) != {"base_profiles_sha256", "refresh_bindings", "new_melee_bindings",
                     "runtime_activation", "schema", "scope_flags", "held_path", "held_sha256"} or \
            data["runtime_activation"] is not False or \
            data["schema"] != "OTERYN_CURRENT_MONSTER_MELEE_IMPORT/v1" or \
            digest(generated[CREATURE_PROFILES]) != data["base_profiles_sha256"]:
        raise ValueError("CURRENT_MONSTER_INPUT_CHANGED")
    document = json.loads(generated[CREATURE_PROFILES])
    by_id = {encoded(row["profile"]["target"]): row for row in document["records"]}
    seen = set()
    for binding in data["refresh_bindings"] + data["new_melee_bindings"]:
        key = encoded(binding["creature"])
        row = by_id.get(key)
        # Qualified record receipts use sorted, ASCII-escaped compact JSON,
        # without a trailing newline; collection files use UTF-8 plus newline.
        record_sha256 = digest(json.dumps(row, sort_keys=True, separators=(",", ":")).encode())
        if key in seen or row is None or record_sha256 != binding["base_record_sha256"]:
            raise ValueError("CURRENT_MONSTER_RECORD_CHANGED")
        seen.add(key)
        if "source_id" in binding:
            if set(binding) != {"creature", "base_record_sha256", "source_id"} or not row.get("monster_melee"):
                raise ValueError("CURRENT_MONSTER_REFRESH_SHAPE")
            donor = donors.get(binding["source_id"])
            if donor is None or donor[0] != row["monster_melee"]["source_repository"]:
                raise ValueError("CURRENT_MONSTER_DONOR_MISMATCH")
            row["monster_melee"]["source_revision"] = donor[2]
        else:
            if set(binding) != {"creature", "base_record_sha256", "monster_melee"} or row.get("monster_melee"):
                raise ValueError("CURRENT_MONSTER_NEW_SHAPE")
            melee = binding["monster_melee"]
            if (melee["source_repository"], melee["source_revision"]) not in {(d[0], d[2]) for d in donors.values()} or \
                    not any(a["ability"] == melee["ability"] and a["interval_ms"] == melee["interval_ms"] and
                            a["chance_ppm"] == melee["chance_ppm"] and a.get("range_tiles", 1) == 1
                            for a in row["behavior"]["data"]["profile"]["attacks"]):
                raise ValueError("CURRENT_MONSTER_ABILITY_UNBOUND")
            row["monster_melee"] = copy.deepcopy(melee)
    return encoded(document)


def outputs(root, generated):
    path = root / INPUT
    if not path.exists():
        return generated
    input_bytes = path.read_bytes()
    if digest(input_bytes) != INPUT_SHA256:
        raise ValueError("CURRENT_SOURCE_UNREVIEWED_INPUT")
    data = json.loads(input_bytes)
    fields = {"schema", "base_catalog_sha256", "bindings", "held_path", "held_sha256", "runtime_activation"}
    if "monster_melee" in data:
        fields.add("monster_melee")
    if set(data) != fields or \
            data["schema"] != "OTERYN_CURRENT_SPELL_SOURCE_BINDINGS/v1" or data["runtime_activation"] is not False:
        raise ValueError("CURRENT_SOURCE_INPUT_SHAPE")
    if digest(generated[CATALOG]) != data["base_catalog_sha256"]:
        raise ValueError("CURRENT_SOURCE_BASE_CHANGED")
    held = data["held_path"]
    if held != "content/abilities/source-imports/unavailable-spells.jsonl":
        raise ValueError("CURRENT_SOURCE_HELD_PATH")
    held_bytes = (root / held).read_bytes()
    if digest(held_bytes) != data["held_sha256"]:
        raise ValueError("CURRENT_SOURCE_HELD_CHANGED")
    hold_keys = set()
    hold_rows = {}
    for line in held_bytes.splitlines():
        row = json.loads(line)
        if row["missing_class"] not in CLASSES or row["runtime_activation"] is not False or row["registration"] in hold_keys:
            raise ValueError("CURRENT_SOURCE_HELD_INVALID")
        hold_keys.add(row["registration"])
        hold_rows[row["registration"]] = row
    catalog = json.loads(generated[CATALOG])
    by_id = {(entry["bundle"]["spell"]["identity"]["key"],
              entry["bundle"]["spell"]["identity"]["revision"]): entry for entry in catalog["bundles"]}
    if len(by_id) != len(catalog["bundles"]):
        raise ValueError("CURRENT_SOURCE_AMBIGUOUS_IDENTITY")
    base_payloads = {key: payload(entry) for key, entry in by_id.items()}
    base_entries = copy.deepcopy(by_id)
    baseline_selection = json.loads(generated[SELECTION])
    project = adapter(root)
    seen = set()
    replacements = {}
    for binding in data["bindings"]:
        ordinary = binding.get("projection") == "ordinary_data"
        fields = {"target", "payload_sha256", "source", "registration", "projection"}
        if "qualification" in binding:
            fields.add("qualification")
        if ordinary:
            fields.add("replacement")
        if set(binding) != fields or binding["projection"] not in {"identity_only", "local_references", "ordinary_data"}:
            raise ValueError("CURRENT_SOURCE_BINDING_SHAPE")
        qualification = binding.get("qualification", {})
        if qualification and (qualification.get("runtime_activation") is not False or
                              qualification.get("raw_source_parity") is not False):
            raise ValueError("CURRENT_SOURCE_QUALIFICATION_SHAPE")
        partial_identity = qualification.get("scope") == "accepted_partial_model"
        native_identity = qualification.get("scope") == "accepted_partial_native_profile"
        key = binding["registration"]
        source = binding["source"]
        if partial_identity and (binding["projection"] != "identity_only" or key not in hold_keys or
                                 not qualification.get("remaining_source_scope")):
            raise ValueError("CURRENT_SOURCE_PARTIAL_SCOPE")
        if native_identity and (binding["projection"] != "identity_only" or key not in hold_keys):
            raise ValueError("CURRENT_SOURCE_PARTIAL_NATIVE_SCOPE")
        if key in seen or (key in hold_keys and not ordinary and not partial_identity and not native_identity) or not key.startswith(source["source_id"] + "/" + source["path"] + "#"):
            raise ValueError("CURRENT_SOURCE_REGISTRATION_CONFLICT")
        seen.add(key)
        target = project.identity(binding["target"])
        identity = (target["key"], target["revision"])
        entry = by_id.get(identity)
        if entry is None or base_payloads[identity] != binding["payload_sha256"]:
            raise ValueError("CURRENT_SOURCE_PAYLOAD_CHANGED")
        if native_identity:
            partial_native(binding, base_entries[identity], hold_rows[key], baseline_selection)
        if partial_identity and (entry["bundle"]["spell"]["name"] not in {
                "Enchant Party", "Enlighten Party", "Heal Party", "Protect Party", "Train Party"} or
                entry["bundle"]["spell"].get("execution", {}).get("native_behavior", {}).get("key") != "party_buff" or
                hold_rows[key].get("name", "").casefold() != entry["bundle"]["spell"]["name"].casefold() or
                hold_rows[key].get("scope") != "donor_party_lifecycle_and_selection" or
                hold_rows[key].get("source_scope") != qualification["remaining_source_scope"]):
            raise ValueError("CURRENT_SOURCE_PARTIAL_SCOPE")
        if ordinary:
            replacement = binding["replacement"]
            if set(replacement) != {"bundle", "dependencies", "scope"} or \
                    replacement["scope"] not in {"canonical_base", "accepted_ordinary_model"} or \
                    (replacement["scope"] == "canonical_base" and key not in hold_keys):
                raise ValueError("CURRENT_SOURCE_PARTIAL_SCOPE")
            if qualification.get("scope") == "accepted_partial_ordinary_base" and (
                    replacement["scope"] != "canonical_base" or key not in hold_keys or
                    replacement["bundle"] != entry["bundle"] or
                    replacement["dependencies"] != entry["dependencies"] or
                    not qualification.get("remaining_source_scope") or
                    not qualification.get("remaining") or
                    any(row.get("missing_class") != "SOURCE_CONFLICT_KEPT"
                        for row in qualification["remaining"]) or
                    hold_rows[key].get("target") != binding["target"] or
                    hold_rows[key].get("scope") != "donor_controller_primary_base_only" or
                    hold_rows[key].get("source_scope") != qualification["remaining_source_scope"] or
                    hold_rows[key].get("name", "").casefold() != entry["bundle"]["spell"]["name"].casefold()):
                raise ValueError("CURRENT_SOURCE_PARTIAL_ORDINARY_SCOPE")
            entry["bundle"], entry["dependencies"] = project.project_supported_candidate(
                replacement["bundle"], replacement["dependencies"], entry, target)
            replacement_digest = payload(entry)
            if identity in replacements and replacements[identity] != replacement_digest:
                raise ValueError("CURRENT_SOURCE_REPLACEMENT_CONFLICT")
            replacements[identity] = replacement_digest
        elif payload(entry) != base_payloads[identity]:
            raise ValueError("CURRENT_SOURCE_BINDING_PAYLOAD_CHANGED")
        updated = project.attach_source(entry, source)
        entry.clear()
        entry.update(updated)
    result = copy.deepcopy(generated)
    result[CATALOG] = encoded(catalog)
    if "monster_melee" in data:
        monsters = data["monster_melee"]
        if monsters["held_path"] != "content/abilities/source-imports/unavailable-monster-melee.jsonl" or \
                digest((root / monsters["held_path"]).read_bytes()) != monsters["held_sha256"]:
            raise ValueError("CURRENT_MONSTER_HELD_CHANGED")
        result[CREATURE_PROFILES] = monster_melee(monsters, generated, project.DONORS)
    selection = json.loads(result[SELECTION])
    if selection["catalog_sha256"] != digest(generated[CATALOG]):
        raise ValueError("CURRENT_SOURCE_SELECTION_BASE_MISMATCH")
    selection["catalog_sha256"] = digest(result[CATALOG])
    result[SELECTION] = encoded(selection)
    manifest = json.loads(result[MANIFEST])
    for name, path in (("catalog", CATALOG), ("source_selection", SELECTION)):
        manifest[name]["sha256"] = digest(result[path])
    if "monster_melee" in data:
        manifest["creature_profiles"]["sha256"] = digest(result[CREATURE_PROFILES])
    result[MANIFEST] = encoded(manifest)
    for path, value in result.items():
        if path == CATALOG or not path.startswith("content/abilities/"):
            continue
        document = json.loads(value)
        if document.get("schema") == "OTERYN_SPELL_DEPENDENCY_COLLECTION/v1":
            section = {"Ability": "abilities", "Effect": "effects", "Formula": "formulas"}[document["family"]]
            records = {}
            for entry in catalog["bundles"]:
                for record in entry["dependencies"][section]:
                    identity = (record["identity"]["key"], record["identity"]["revision"])
                    if identity in records and encoded(records[identity]) != encoded(record):
                        raise ValueError("CURRENT_SOURCE_DEPENDENCY_CONFLICT")
                    records[identity] = record
            document["records"] = [records[key] for key in sorted(records)]
            document["record_count"] = len(records)
            document["source_catalog"]["sha256"] = digest(result[CATALOG])
            result[path] = encoded(document)
    return result
