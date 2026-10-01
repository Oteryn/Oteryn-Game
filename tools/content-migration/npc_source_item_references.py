"""Canonicalize Item keys in NPC source fields using protected exact aliases.

The immutable R4 field remains the original proof. Active source evidence uses
canonical Item keys and records an exact R4 locator/field hash plus the protected
alias version/digest. Historical namespaces/local IDs are separate values; no
retired contiguous Item key is copied into authored content.
"""

import copy
import hashlib
import importlib.util
import json
import re
from pathlib import Path

ITEM_TOKEN = re.compile(r"oteryn:item\.[a-z0-9_.-]+")


class SourceItemReferenceError(ValueError):
    pass


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def encode(value):
    return (
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        + "\n"
    ).encode("utf-8")


def aliases_bytes(value):
    # Matches the protected item_id_alias_table canonical digest contract.
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def load_registry(repo, index_path=None):
    repo = Path(repo)
    path = Path(index_path) if index_path else repo / "content/items/index.json"
    index = json.loads(path.read_bytes())
    if index.get("family") != "Item":
        raise SourceItemReferenceError("active registry must be the Item family")
    active = set()
    for shard in index["shards"]:
        for record in json.loads((repo / shard).read_bytes())["records"]:
            identity = record["definition"]["identity"]
            if identity.get("family") != "Item" or identity["key"] in active:
                raise SourceItemReferenceError(
                    "invalid or duplicate active Item identity"
                )
            active.add(identity["key"])
    if len(active) != index["record_count"]:
        raise SourceItemReferenceError("active Item count mismatch")
    return active


class SourceItemNormalizer:
    def __init__(
        self,
        table_raw,
        active_items,
        r4,
        custody,
        table_locator="content/items/aliases.json",
    ):
        table = json.loads(table_raw)
        if (
            table.get("schema") != "OTERYN_ITEM_KEY_ALIAS_TABLE/v1"
            or table.get("key_rule") != "OTERYN_TIBIA_ID_KEY_RULE_V1"
        ):
            raise SourceItemReferenceError("unsupported protected alias table")
        entries = table["entries"]
        if digest(aliases_bytes(entries)) != table["entries_sha256"]:
            raise SourceItemReferenceError("protected alias entries digest mismatch")
        self.aliases = {}
        for index, entry in enumerate(entries):
            key, version = entry["key"], entry["version"]
            if not isinstance(version, int) or isinstance(version, bool) or version < 1:
                raise SourceItemReferenceError("invalid alias version")
            old = self.aliases.get(key)
            if old and version <= old[1]["version"]:
                raise SourceItemReferenceError("alias versions must increase")
            self.aliases[key] = (index, entry)
        self.active = set(active_items)
        self.table_proof = {
            "locator": table_locator,
            "sha256": digest(table_raw),
            "entries_sha256": table["entries_sha256"],
        }
        self.r4_fields = {}
        for index, record in enumerate(r4["records"]):
            for position, source_field in enumerate(record["fields"]):
                key = (record["identity"]["key"], source_field["field_path"])
                if key in self.r4_fields:
                    raise SourceItemReferenceError("duplicate original R4 source field")
                self.r4_fields[key] = (
                    source_field,
                    f"/records/{index}/fields/{position}",
                )
        self.custody = copy.deepcopy(custody)
        if (
            not custody
            or not custody.get("locator")
            or not re.fullmatch(r"[a-f0-9]{64}", custody.get("sha256", ""))
        ):
            raise SourceItemReferenceError("exact immutable R4 custody is required")

    def normalize(self, service_key, source_field):
        value = source_field.get("value", {})
        if (
            not source_field["field_path"].startswith("oteryn:source.npc.")
            or value.get("type") != "Text"
        ):
            return copy.deepcopy(source_field)
        raw_text = value.get("value")
        if not isinstance(raw_text, str):
            raise SourceItemReferenceError("NPC source Text value is not a string")
        if not any(
            token in self.aliases or token.startswith("oteryn:item.registry.")
            for token in ITEM_TOKEN.findall(raw_text)
        ):
            return copy.deepcopy(source_field)
        try:
            original = json.loads(raw_text)
        except json.JSONDecodeError as error:
            raise SourceItemReferenceError(
                "retired Item reference is not structured source JSON"
            ) from error
        changes = []

        def normalize_value(node, pointer=""):
            if isinstance(node, list):
                return [
                    normalize_value(child, f"{pointer}/{index}")
                    for index, child in enumerate(node)
                ]
            if isinstance(node, dict):
                if any(
                    "oteryn:item.registry." in key or key in self.aliases
                    for key in node
                ):
                    raise SourceItemReferenceError(
                        "retired Item reference used as a JSON member name"
                    )
                return {
                    key: normalize_value(
                        child, pointer + "/" + key.replace("~", "~0").replace("/", "~1")
                    )
                    for key, child in node.items()
                }
            if not isinstance(node, str):
                return node
            if node not in self.aliases:
                if any(
                    token in self.aliases or token.startswith("oteryn:item.registry.")
                    for token in ITEM_TOKEN.findall(node)
                ):
                    raise SourceItemReferenceError(
                        "unknown or embedded retired Item reference"
                    )
                return node
            index, entry = self.aliases[node]
            target = entry.get("target")
            if entry.get("state") != "ALIAS" or target not in self.active:
                raise SourceItemReferenceError(
                    "retired Item alias has no active target"
                )
            if target in self.aliases or not target.startswith("oteryn:item.tibia.i"):
                raise SourceItemReferenceError("Item alias target is not canonical")
            qualified_namespace, local_id = node.rsplit(".", 1)
            namespace = qualified_namespace.removeprefix("oteryn:item.")
            changes.append(
                {
                    "pointer": pointer,
                    "historical_identity": {
                        "family": "Item",
                        "namespace": namespace,
                        "local_id": local_id,
                    },
                    "canonical_key": target,
                    "alias_version": entry["version"],
                    "alias_pointer": f"/entries/{index}",
                    "alias_entry_sha256": digest(aliases_bytes(entry)),
                }
            )
            return target

        normalized = normalize_value(original)
        if not changes:
            return copy.deepcopy(source_field)
        if (
            not isinstance(normalized, dict)
            or "item_reference_canonicalization" in normalized
        ):
            raise SourceItemReferenceError(
                "unexpected source evidence shape or normalization collision"
            )
        old = self.r4_fields.get((service_key, source_field["field_path"]))
        if old is None or old[0] != source_field:
            raise SourceItemReferenceError(
                "original source field does not match immutable R4 custody"
            )
        normalized["item_reference_canonicalization"] = {
            "state": "PROTECTED_EXACT_ITEM_ALIASES",
            "runtime_qualified": False,
            "original_field": {
                **self.custody,
                "pointer": old[1],
                "field_sha256": digest(encode(source_field)),
                "text_sha256": digest(raw_text.encode("utf-8")),
            },
            "alias_table": self.table_proof,
            "references": changes,
        }
        result = copy.deepcopy(source_field)
        result["value"]["value"] = encode(normalized).decode().rstrip("\n")
        return result


def load_protected_normalizer(repo, r4, custody):
    """Use the existing protected history/membership validator before alias resolution."""
    repo = Path(repo)
    verifier = repo / "tools/content-census/item_id_alias_table.py"
    spec = importlib.util.spec_from_file_location(
        "npc_protected_item_alias_verifier", verifier
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.verify()
    return SourceItemNormalizer(
        (repo / "content/items/aliases.json").read_bytes(),
        load_registry(repo),
        r4,
        custody,
    )
