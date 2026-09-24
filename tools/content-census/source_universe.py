#!/usr/bin/env python3
"""Bounded source-universe discovery for the full Oteryn Tibia content census.

G1 discovers source identities and structured source metadata only. It does not
perform Oteryn identity resolution, semantic promotion, WorldProject population,
or long-form prose collection. The protected Item wiki-first census is reused as
a sealed lane and is never re-fetched here.
"""
from __future__ import annotations

import argparse
from collections import Counter, defaultdict, deque
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import re
from typing import Any, Iterable

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
DEFAULT_REGISTRY = HERE / "source-surfaces.json"
PREDECESSOR_PATH = (
    REPO_ROOT
    / "tools"
    / "reference-world-corridor-census"
    / "item_current_source_tibiawiki.py"
)

_spec = importlib.util.spec_from_file_location(
    "item_current_source_tibiawiki", PREDECESSOR_PATH
)
if _spec is None or _spec.loader is None:
    raise RuntimeError("bounded TibiaWiki predecessor import failed")
predecessor = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(predecessor)

SCHEMA = "OTERYN_FULL_CONTENT_SOURCE_UNIVERSE/v1"
MANIFEST_SCHEMA = "OTERYN_FULL_CONTENT_SOURCE_CENSUS_MANIFEST/v1"
REGISTRY_SCHEMA = "OTERYN_FULL_CONTENT_SOURCE_SURFACES/v1"
PROFILE = "OTERYN_FULL_CONTENT_SOURCE_UNIVERSE_COLLECTOR/v1"
SOURCE_ROLE = "STRUCTURED_REFERENCE_DATA"

ALLOWED_ROOT_KINDS = {
    "protected_manifest",
    "category_tree",
    "page_links",
    "page_only",
    "registry_only",
}
ALLOWED_SURFACE_DISPOSITIONS = {
    "MAPPED_TO_OTERYN_FAMILY",
    "RELATIONSHIP_SYSTEM",
    "PROVENANCE_ONLY",
    "DUPLICATE_OVERLAP",
    "SOURCE_NAVIGATION_ONLY",
}
FAMILY_STATES = {
    "EXACT_FAMILY",
    "MULTI_FAMILY_RELATION",
    "SOURCE_NAVIGATION_ONLY",
}
SOURCE_SHAPES = {
    "STRUCTURED_PRIMARY",
    "STRUCTURED_ALTERNATE",
    "CATEGORY_ONLY",
    "RELATIONSHIP_PAGE",
    "REDIRECT",
    "SOURCE_CLASSIFICATION_UNRESOLVED",
}

# Registry values are caller-selected evidence limits, but the collector also
# has fixed safety ceilings so a malformed registry cannot silently unbound it.
HARD_LIMITS = {
    "max_requests": 10_000,
    "max_non_item_pages": 100_000,
    "max_category_nodes": 5_000,
    "max_category_depth": 8,
    "max_links_per_root": 20_000,
    "metadata_batch_size": 20,
    "max_categories_per_page": 512,
    "max_templates_per_page": 512,
}
MAX_ROOTS = 128
MAX_SURFACES = 128
MAX_TITLE_BYTES = 256
MAX_REGISTRY_BYTES = 512 * 1024
MAX_MANIFEST_BYTES = 2 * 1024 * 1024
MAX_CATEGORY_BATCH = 500
MAX_GENERATOR_BATCH = 500


class CensusError(RuntimeError):
    pass


def canonical_bytes(value: Any) -> bytes:
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        )
        + "\n"
    ).encode("utf-8")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def utc_now_iso() -> str:
    return (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )


def _bounded_string(value: Any, *, label: str, max_bytes: int = MAX_TITLE_BYTES) -> str:
    if not isinstance(value, str):
        raise CensusError(f"{label}_NOT_STRING")
    if not value or value != value.strip():
        raise CensusError(f"{label}_INVALID")
    size = len(value.encode("utf-8"))
    if size > max_bytes:
        raise CensusError(f"{label}_MAX_PLUS_ONE:{size}")
    return value


def normalized_title(value: str) -> str:
    value = _bounded_string(value, label="TITLE")
    value = value.replace("_", " ")
    value = re.sub(r"\s+", " ", value).strip().casefold()
    for prefix in ("categoria:", "category:"):
        if value.startswith(prefix):
            value = value[len(prefix) :].strip()
            break
    return value


def _safe_repo_path(value: str) -> Path:
    text = _bounded_string(value, label="REPOSITORY_PATH", max_bytes=512)
    path = Path(text)
    if path.is_absolute() or ".." in path.parts:
        raise CensusError("REPOSITORY_PATH_UNSAFE")
    resolved = (REPO_ROOT / path).resolve()
    try:
        resolved.relative_to(REPO_ROOT.resolve())
    except ValueError as exc:
        raise CensusError("REPOSITORY_PATH_ESCAPES_ROOT") from exc
    return resolved


def load_json_file(path: Path, *, max_bytes: int, label: str) -> dict[str, Any]:
    payload = path.read_bytes()
    if len(payload) > max_bytes:
        raise CensusError(f"{label}_MAX_PLUS_ONE:{len(payload)}")
    try:
        value = json.loads(payload)
    except json.JSONDecodeError as exc:
        raise CensusError(f"{label}_JSON_INVALID") from exc
    if not isinstance(value, dict):
        raise CensusError(f"{label}_ROOT_NOT_OBJECT")
    return value


def load_registry(path: Path) -> dict[str, Any]:
    return load_json_file(path, max_bytes=MAX_REGISTRY_BYTES, label="REGISTRY")


def _unique_strings(value: Any, *, label: str, max_items: int = 256) -> list[str]:
    if not isinstance(value, list) or len(value) > max_items:
        raise CensusError(f"{label}_INVALID")
    result: list[str] = []
    seen: set[str] = set()
    for raw in value:
        item = _bounded_string(raw, label=label)
        if item in seen:
            raise CensusError(f"{label}_DUPLICATE:{item}")
        seen.add(item)
        result.append(item)
    return result


def validate_registry(registry: dict[str, Any]) -> dict[str, Any]:
    if registry.get("schema") != REGISTRY_SCHEMA:
        raise CensusError("REGISTRY_SCHEMA_INVALID")
    source = registry.get("source")
    if not isinstance(source, dict):
        raise CensusError("REGISTRY_SOURCE_INVALID")
    if source.get("role") != SOURCE_ROLE:
        raise CensusError("REGISTRY_SOURCE_ROLE_INVALID")
    api = _bounded_string(source.get("api"), label="SOURCE_API", max_bytes=512)
    if api != predecessor.API_BASE:
        raise CensusError("REGISTRY_SOURCE_API_MISMATCH")

    limits = registry.get("limits")
    if not isinstance(limits, dict):
        raise CensusError("REGISTRY_LIMITS_INVALID")
    validated_limits: dict[str, int] = {}
    for key, hard_max in HARD_LIMITS.items():
        value = limits.get(key)
        if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
            raise CensusError(f"REGISTRY_LIMIT_INVALID:{key}")
        if value > hard_max:
            raise CensusError(f"REGISTRY_LIMIT_HARD_MAX_PLUS_ONE:{key}:{value}")
        validated_limits[key] = value

    exclusions_raw = registry.get("hard_exclusions")
    if not isinstance(exclusions_raw, list) or not exclusions_raw:
        raise CensusError("REGISTRY_HARD_EXCLUSIONS_INVALID")
    exclusion_aliases: set[str] = set()
    for row in exclusions_raw:
        if not isinstance(row, dict):
            raise CensusError("REGISTRY_HARD_EXCLUSION_ROW_INVALID")
        surface = _bounded_string(row.get("surface"), label="HARD_EXCLUSION")
        aliases = _unique_strings(
            row.get("aliases"), label="HARD_EXCLUSION_ALIAS", max_items=32
        )
        aliases.append(surface)
        for alias in aliases:
            exclusion_aliases.add(normalized_title(alias))

    roots_raw = registry.get("roots")
    if not isinstance(roots_raw, list) or not roots_raw or len(roots_raw) > MAX_ROOTS:
        raise CensusError("REGISTRY_ROOTS_INVALID")
    roots: dict[str, dict[str, Any]] = {}
    for row in roots_raw:
        if not isinstance(row, dict):
            raise CensusError("REGISTRY_ROOT_ROW_INVALID")
        root_id = _bounded_string(row.get("id"), label="ROOT_ID", max_bytes=128)
        if root_id in roots:
            raise CensusError(f"REGISTRY_ROOT_ID_DUPLICATE:{root_id}")
        kind = _bounded_string(row.get("kind"), label="ROOT_KIND", max_bytes=64)
        if kind not in ALLOWED_ROOT_KINDS:
            raise CensusError(f"REGISTRY_ROOT_KIND_INVALID:{kind}")
        normalized = dict(row)
        normalized["id"] = root_id
        normalized["kind"] = kind
        if kind == "protected_manifest":
            path_text = _bounded_string(
                row.get("manifest_path"), label="MANIFEST_PATH", max_bytes=512
            )
            _safe_repo_path(path_text)
            normalized["manifest_path"] = path_text
            normalized["expected_schema"] = _bounded_string(
                row.get("expected_schema"), label="EXPECTED_MANIFEST_SCHEMA"
            )
            digest = _bounded_string(
                row.get("expected_stable_digest"),
                label="EXPECTED_STABLE_DIGEST",
                max_bytes=64,
            )
            if re.fullmatch(r"[0-9a-f]{64}", digest) is None:
                raise CensusError("REGISTRY_EXPECTED_DIGEST_INVALID")
            normalized["expected_stable_digest"] = digest
        else:
            title = _bounded_string(row.get("title"), label="ROOT_TITLE")
            if normalized_title(title) in exclusion_aliases:
                raise CensusError(f"REGISTRY_HARD_EXCLUSION_ROOT:{title}")
            normalized["title"] = title
            include_root = row.get("include_root", False)
            if not isinstance(include_root, bool):
                raise CensusError(f"REGISTRY_ROOT_INCLUDE_INVALID:{root_id}")
            normalized["include_root"] = include_root
        roots[root_id] = normalized

    surfaces_raw = registry.get("surfaces")
    if (
        not isinstance(surfaces_raw, list)
        or not surfaces_raw
        or len(surfaces_raw) > MAX_SURFACES
    ):
        raise CensusError("REGISTRY_SURFACES_INVALID")
    surfaces: list[dict[str, Any]] = []
    seen_surfaces: set[str] = set()
    for row in surfaces_raw:
        if not isinstance(row, dict):
            raise CensusError("REGISTRY_SURFACE_ROW_INVALID")
        name = _bounded_string(row.get("name"), label="SURFACE_NAME")
        if name in seen_surfaces:
            raise CensusError(f"REGISTRY_SURFACE_DUPLICATE:{name}")
        seen_surfaces.add(name)
        if normalized_title(name) in exclusion_aliases:
            raise CensusError(f"REGISTRY_HARD_EXCLUSION_SURFACE:{name}")
        disposition = _bounded_string(
            row.get("disposition"), label="SURFACE_DISPOSITION", max_bytes=64
        )
        if disposition not in ALLOWED_SURFACE_DISPOSITIONS:
            raise CensusError(
                f"REGISTRY_SURFACE_DISPOSITION_INVALID:{name}:{disposition}"
            )
        families = _unique_strings(
            row.get("families"), label="SURFACE_FAMILY", max_items=64
        )
        root_ids = _unique_strings(
            row.get("root_ids"), label="SURFACE_ROOT_ID", max_items=64
        )
        if not root_ids:
            raise CensusError(f"REGISTRY_SURFACE_WITHOUT_ROOT:{name}")
        unknown = [root_id for root_id in root_ids if root_id not in roots]
        if unknown:
            raise CensusError(
                f"REGISTRY_SURFACE_UNKNOWN_ROOT:{name}:{unknown[0]}"
            )
        surfaces.append(
            {
                "name": name,
                "disposition": disposition,
                "families": sorted(families),
                "root_ids": sorted(root_ids),
            }
        )

    # Every root must be reachable from at least one in-scope surface.
    used_roots = {root for row in surfaces for root in row["root_ids"]}
    orphaned = sorted(set(roots) - used_roots)
    if orphaned:
        raise CensusError(f"REGISTRY_ORPHAN_ROOT:{orphaned[0]}")

    return {
        "schema": REGISTRY_SCHEMA,
        "source": {
            "id": _bounded_string(source.get("id"), label="SOURCE_ID"),
            "role": SOURCE_ROLE,
            "api": api,
        },
        "hard_exclusions": exclusions_raw,
        "exclusion_aliases": exclusion_aliases,
        "limits": validated_limits,
        "roots": roots,
        "surfaces": sorted(surfaces, key=lambda row: row["name"].casefold()),
    }


def is_excluded_title(value: str, exclusion_aliases: set[str]) -> bool:
    return normalized_title(value) in exclusion_aliases


class RequestBudget:
    def __init__(self, limit: int) -> None:
        self.limit = limit
        self.used = 0

    def take(self) -> None:
        if self.used >= self.limit:
            raise CensusError(f"REQUEST_BUDGET_MAX_PLUS_ONE:{self.used + 1}")
        self.used += 1


def _query(
    client: Any,
    budget: RequestBudget,
    params: dict[str, str],
) -> dict[str, Any]:
    budget.take()
    value = client.get_json(params)
    if not isinstance(value, dict):
        raise CensusError("API_ROOT_INVALID")
    if "error" in value:
        raise CensusError("API_ERROR_RESPONSE")
    query = value.get("query")
    if query is not None and not isinstance(query, dict):
        raise CensusError("API_QUERY_INVALID")
    return value


def _continuation(value: Any) -> dict[str, str] | None:
    if value is None:
        return None
    if not isinstance(value, dict) or not value:
        raise CensusError("API_CONTINUATION_INVALID")
    result: dict[str, str] = {}
    for key, raw in value.items():
        if not isinstance(key, str) or not key:
            raise CensusError("API_CONTINUATION_KEY_INVALID")
        if not isinstance(raw, (str, int)) or isinstance(raw, bool):
            raise CensusError("API_CONTINUATION_VALUE_INVALID")
        result[key] = str(raw)
    return result


def iter_continued(
    client: Any,
    budget: RequestBudget,
    base_params: dict[str, str],
) -> Iterable[dict[str, Any]]:
    continuation: dict[str, str] = {}
    seen: set[tuple[tuple[str, str], ...]] = set()
    while True:
        params = dict(base_params)
        params.update(continuation)
        value = _query(client, budget, params)
        yield value
        next_value = _continuation(value.get("continue"))
        if next_value is None:
            return
        signature = tuple(sorted(next_value.items()))
        if signature in seen:
            raise CensusError("API_CONTINUATION_LOOP")
        seen.add(signature)
        continuation = next_value


def _validate_page_id(value: Any) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise CensusError("API_PAGE_ID_INVALID")
    return value


def _page_title(value: Any) -> str:
    return _bounded_string(value, label="PAGE_TITLE")


def resolve_page(
    client: Any,
    budget: RequestBudget,
    title: str,
) -> dict[str, Any]:
    value = _query(
        client,
        budget,
        {
            "action": "query",
            "prop": "info|revisions",
            "titles": title,
            "rvprop": "ids|timestamp",
            "redirects": "1",
            "format": "json",
            "formatversion": "2",
        },
    )
    query = value.get("query")
    if not isinstance(query, dict):
        raise CensusError("ROOT_QUERY_MISSING")
    pages = query.get("pages")
    if not isinstance(pages, list) or len(pages) != 1:
        raise CensusError("ROOT_PAGE_CARDINALITY_INVALID")
    page = pages[0]
    if not isinstance(page, dict) or page.get("missing") is True:
        raise CensusError(f"ROOT_PAGE_MISSING:{title}")
    page_id = _validate_page_id(page.get("pageid"))
    resolved_title = _page_title(page.get("title"))
    revisions = page.get("revisions")
    if not isinstance(revisions, list) or len(revisions) != 1:
        raise CensusError("ROOT_REVISION_CARDINALITY_INVALID")
    revision = revisions[0]
    if not isinstance(revision, dict):
        raise CensusError("ROOT_REVISION_INVALID")
    revision_id = _validate_page_id(revision.get("revid"))
    timestamp = revision.get("timestamp")
    if not isinstance(timestamp, str) or not timestamp.endswith("Z"):
        raise CensusError("ROOT_REVISION_TIMESTAMP_INVALID")
    return {
        "page_id": page_id,
        "title": resolved_title,
        "revision_id": revision_id,
        "revision_timestamp": timestamp,
        "redirect": bool(page.get("redirect") is True),
    }


class DiscoveryIndex:
    def __init__(self, *, max_pages: int, exclusion_aliases: set[str]) -> None:
        self.max_pages = max_pages
        self.exclusion_aliases = exclusion_aliases
        self.by_id: dict[int, dict[str, Any]] = {}
        self.id_by_title: dict[str, int] = {}

    def add(
        self,
        page_id: int,
        title: str,
        *,
        root_id: str,
        discovery_kind: str,
    ) -> bool:
        page_id = _validate_page_id(page_id)
        title = _page_title(title)
        if is_excluded_title(title, self.exclusion_aliases):
            return False
        normalized = normalized_title(title)
        existing_id = self.id_by_title.get(normalized)
        if existing_id is not None and existing_id != page_id:
            raise CensusError(
                f"DISCOVERY_TITLE_PAGE_ID_CONFLICT:{title}:{existing_id}:{page_id}"
            )
        existing = self.by_id.get(page_id)
        if existing is not None and existing["title"] != title:
            raise CensusError(
                f"DISCOVERY_PAGE_ID_TITLE_CONFLICT:{page_id}:{existing['title']}:{title}"
            )
        if existing is None:
            if len(self.by_id) >= self.max_pages:
                raise CensusError(
                    f"DISCOVERY_PAGE_COUNT_MAX_PLUS_ONE:{len(self.by_id) + 1}"
                )
            existing = {
                "page_id": page_id,
                "title": title,
                "root_ids": set(),
                "discovery_kinds": set(),
            }
            self.by_id[page_id] = existing
            self.id_by_title[normalized] = page_id
        existing["root_ids"].add(root_id)
        existing["discovery_kinds"].add(discovery_kind)
        return True


def discover_category_tree(
    client: Any,
    budget: RequestBudget,
    index: DiscoveryIndex,
    *,
    root_id: str,
    title: str,
    limits: dict[str, int],
) -> dict[str, Any]:
    queue: deque[tuple[str, int]] = deque([(title, 0)])
    seen_categories: set[str] = set()
    page_ids: set[int] = set()
    category_nodes = 0
    while queue:
        category_title, depth = queue.popleft()
        normalized = normalized_title(category_title)
        if normalized in seen_categories:
            continue
        seen_categories.add(normalized)
        category_nodes += 1
        if category_nodes > limits["max_category_nodes"]:
            raise CensusError(
                f"CATEGORY_NODE_COUNT_MAX_PLUS_ONE:{category_nodes}"
            )
        for value in iter_continued(
            client,
            budget,
            {
                "action": "query",
                "list": "categorymembers",
                "cmtitle": category_title,
                "cmnamespace": "0|14",
                "cmlimit": str(MAX_CATEGORY_BATCH),
                "format": "json",
                "formatversion": "2",
            },
        ):
            query = value.get("query")
            if not isinstance(query, dict):
                raise CensusError("CATEGORY_QUERY_MISSING")
            rows = query.get("categorymembers")
            if not isinstance(rows, list):
                raise CensusError("CATEGORY_MEMBERS_INVALID")
            for row in rows:
                if not isinstance(row, dict):
                    raise CensusError("CATEGORY_MEMBER_ROW_INVALID")
                ns = row.get("ns")
                member_title = _page_title(row.get("title"))
                if ns == 0:
                    page_id = _validate_page_id(row.get("pageid"))
                    if index.add(
                        page_id,
                        member_title,
                        root_id=root_id,
                        discovery_kind="CATEGORY_MEMBER",
                    ):
                        page_ids.add(page_id)
                elif ns == 14:
                    if depth < limits["max_category_depth"]:
                        if not is_excluded_title(
                            member_title, index.exclusion_aliases
                        ):
                            queue.append((member_title, depth + 1))
                else:
                    raise CensusError(f"CATEGORY_NAMESPACE_INVALID:{ns}")
    return {
        "root_id": root_id,
        "kind": "category_tree",
        "root_title": title,
        "page_ids": sorted(page_ids),
        "category_nodes": category_nodes,
    }


def discover_page_links(
    client: Any,
    budget: RequestBudget,
    index: DiscoveryIndex,
    *,
    root_id: str,
    title: str,
    include_root: bool,
    max_links: int,
) -> dict[str, Any]:
    root = resolve_page(client, budget, title)
    page_ids: set[int] = set()
    if include_root and index.add(
        root["page_id"],
        root["title"],
        root_id=root_id,
        discovery_kind="ROOT_PAGE",
    ):
        page_ids.add(root["page_id"])

    link_count = 0
    missing_links = 0
    for value in iter_continued(
        client,
        budget,
        {
            "action": "query",
            "generator": "links",
            "titles": root["title"],
            "gplnamespace": "0",
            "gpllimit": str(MAX_GENERATOR_BATCH),
            "prop": "info",
            "format": "json",
            "formatversion": "2",
        },
    ):
        query = value.get("query")
        if not isinstance(query, dict):
            # A page with no links may return no query payload.
            continue
        pages = query.get("pages", [])
        if not isinstance(pages, list):
            raise CensusError("PAGE_LINKS_INVALID")
        for page in pages:
            if not isinstance(page, dict):
                raise CensusError("PAGE_LINK_ROW_INVALID")
            page_title = _page_title(page.get("title"))
            if page.get("missing") is True:
                missing_links += 1
                continue
            page_id = _validate_page_id(page.get("pageid"))
            if is_excluded_title(page_title, index.exclusion_aliases):
                continue
            link_count += 1
            if link_count > max_links:
                raise CensusError(
                    f"PAGE_LINK_COUNT_MAX_PLUS_ONE:{root_id}:{link_count}"
                )
            if index.add(
                page_id,
                page_title,
                root_id=root_id,
                discovery_kind="PAGE_LINK",
            ):
                page_ids.add(page_id)
    return {
        "root_id": root_id,
        "kind": "page_links",
        "root_title": root["title"],
        "root_page_id": root["page_id"],
        "page_ids": sorted(page_ids),
        "discovered_links": link_count,
        "missing_links": missing_links,
    }


def discover_page_only(
    client: Any,
    budget: RequestBudget,
    index: DiscoveryIndex,
    *,
    root_id: str,
    title: str,
) -> dict[str, Any]:
    root = resolve_page(client, budget, title)
    index.add(
        root["page_id"],
        root["title"],
        root_id=root_id,
        discovery_kind="ROOT_PAGE",
    )
    return {
        "root_id": root_id,
        "kind": "page_only",
        "root_title": root["title"],
        "root_page_id": root["page_id"],
        "page_ids": [root["page_id"]],
    }


def load_protected_manifest(root: dict[str, Any]) -> dict[str, Any]:
    path = _safe_repo_path(root["manifest_path"])
    manifest = load_json_file(
        path, max_bytes=MAX_MANIFEST_BYTES, label="PROTECTED_MANIFEST"
    )
    if manifest.get("schema") != root["expected_schema"]:
        raise CensusError(
            f"PROTECTED_MANIFEST_SCHEMA_MISMATCH:{root['id']}"
        )
    full_output = manifest.get("full_output")
    if not isinstance(full_output, dict):
        raise CensusError("PROTECTED_MANIFEST_FULL_OUTPUT_INVALID")
    stable_digest = full_output.get("stable_without_retrieval_timestamp_sha256")
    if stable_digest != root["expected_stable_digest"]:
        raise CensusError(
            f"PROTECTED_MANIFEST_DIGEST_MISMATCH:{root['id']}"
        )
    counts = manifest.get("counts")
    if not isinstance(counts, dict):
        raise CensusError("PROTECTED_MANIFEST_COUNTS_INVALID")
    discovered_pages = counts.get("discovered_pages")
    fetched_pages = counts.get("fetched_pages")
    if (
        not isinstance(discovered_pages, int)
        or isinstance(discovered_pages, bool)
        or discovered_pages <= 0
        or fetched_pages != discovered_pages
    ):
        raise CensusError("PROTECTED_MANIFEST_PAGE_COUNT_INVALID")
    return {
        "root_id": root["id"],
        "kind": "protected_manifest",
        "manifest_path": root["manifest_path"],
        "manifest_schema": manifest["schema"],
        "retrieval_timestamp": manifest.get("retrieval_timestamp"),
        "source_pages": discovered_pages,
        "source_shapes": counts.get("source_shapes", {}),
        "stable_digest": stable_digest,
        "full_output_digest": full_output.get("sha256"),
        "authority": manifest.get("authority"),
    }


def chunks(values: list[int], size: int) -> Iterable[list[int]]:
    for offset in range(0, len(values), size):
        yield values[offset : offset + size]


def _merge_page_metadata(
    aggregate: dict[int, dict[str, Any]],
    raw: dict[str, Any],
    *,
    requested: set[int],
    limits: dict[str, int],
    exclusion_aliases: set[str],
) -> None:
    page_id = _validate_page_id(raw.get("pageid"))
    if page_id not in requested:
        raise CensusError(f"METADATA_UNREQUESTED_PAGE:{page_id}")
    title = _page_title(raw.get("title"))
    revisions = raw.get("revisions")
    if not isinstance(revisions, list) or len(revisions) != 1:
        raise CensusError(f"METADATA_REVISION_CARDINALITY_INVALID:{page_id}")
    revision = revisions[0]
    if not isinstance(revision, dict):
        raise CensusError(f"METADATA_REVISION_INVALID:{page_id}")
    revision_id = _validate_page_id(revision.get("revid"))
    timestamp = revision.get("timestamp")
    if not isinstance(timestamp, str) or not timestamp.endswith("Z"):
        raise CensusError(f"METADATA_REVISION_TIMESTAMP_INVALID:{page_id}")

    current = aggregate.setdefault(
        page_id,
        {
            "page_id": page_id,
            "title": title,
            "revision_id": revision_id,
            "revision_timestamp": timestamp,
            "redirect": bool(raw.get("redirect") is True),
            "categories": set(),
            "templates": set(),
        },
    )
    if (
        current["title"] != title
        or current["revision_id"] != revision_id
        or current["revision_timestamp"] != timestamp
    ):
        raise CensusError(f"SOURCE_SNAPSHOT_DRIFT:{page_id}")
    current["redirect"] = current["redirect"] or bool(raw.get("redirect") is True)

    categories = raw.get("categories", [])
    if not isinstance(categories, list):
        raise CensusError(f"METADATA_CATEGORIES_INVALID:{page_id}")
    for row in categories:
        if not isinstance(row, dict):
            raise CensusError(f"METADATA_CATEGORY_ROW_INVALID:{page_id}")
        category_title = _page_title(row.get("title"))
        if is_excluded_title(category_title, exclusion_aliases):
            continue
        current["categories"].add(category_title)
        if len(current["categories"]) > limits["max_categories_per_page"]:
            raise CensusError(
                f"PAGE_CATEGORY_COUNT_MAX_PLUS_ONE:{page_id}:"
                f"{len(current['categories'])}"
            )

    templates = raw.get("templates", [])
    if not isinstance(templates, list):
        raise CensusError(f"METADATA_TEMPLATES_INVALID:{page_id}")
    for row in templates:
        if not isinstance(row, dict):
            raise CensusError(f"METADATA_TEMPLATE_ROW_INVALID:{page_id}")
        template_title = _page_title(row.get("title"))
        if is_excluded_title(template_title, exclusion_aliases):
            continue
        current["templates"].add(template_title)
        if len(current["templates"]) > limits["max_templates_per_page"]:
            raise CensusError(
                f"PAGE_TEMPLATE_COUNT_MAX_PLUS_ONE:{page_id}:"
                f"{len(current['templates'])}"
            )


def fetch_metadata(
    client: Any,
    budget: RequestBudget,
    page_ids: list[int],
    *,
    limits: dict[str, int],
    exclusion_aliases: set[str],
) -> dict[int, dict[str, Any]]:
    aggregate: dict[int, dict[str, Any]] = {}
    for batch in chunks(page_ids, limits["metadata_batch_size"]):
        requested = set(batch)
        for value in iter_continued(
            client,
            budget,
            {
                "action": "query",
                "pageids": "|".join(str(value) for value in batch),
                "prop": "info|revisions|categories|templates",
                "rvprop": "ids|timestamp",
                "cllimit": "max",
                "tllimit": "max",
                "format": "json",
                "formatversion": "2",
            },
        ):
            query = value.get("query")
            if not isinstance(query, dict):
                raise CensusError("METADATA_QUERY_MISSING")
            pages = query.get("pages")
            if not isinstance(pages, list):
                raise CensusError("METADATA_PAGES_INVALID")
            for raw in pages:
                if not isinstance(raw, dict) or raw.get("missing") is True:
                    raise CensusError("METADATA_PAGE_MISSING")
                _merge_page_metadata(
                    aggregate,
                    raw,
                    requested=requested,
                    limits=limits,
                    exclusion_aliases=exclusion_aliases,
                )
        missing = requested - set(aggregate)
        if missing:
            raise CensusError(f"METADATA_PAGE_PARTITION_MISMATCH:{min(missing)}")
    return aggregate


def verify_revisions(
    client: Any,
    budget: RequestBudget,
    metadata: dict[int, dict[str, Any]],
    *,
    batch_size: int,
) -> None:
    page_ids = sorted(metadata)
    seen: set[int] = set()
    for batch in chunks(page_ids, batch_size):
        value = _query(
            client,
            budget,
            {
                "action": "query",
                "pageids": "|".join(str(item) for item in batch),
                "prop": "revisions",
                "rvprop": "ids|timestamp",
                "format": "json",
                "formatversion": "2",
            },
        )
        query = value.get("query")
        if not isinstance(query, dict):
            raise CensusError("REVERIFY_QUERY_MISSING")
        pages = query.get("pages")
        if not isinstance(pages, list):
            raise CensusError("REVERIFY_PAGES_INVALID")
        for raw in pages:
            if not isinstance(raw, dict) or raw.get("missing") is True:
                raise CensusError("REVERIFY_PAGE_MISSING")
            page_id = _validate_page_id(raw.get("pageid"))
            expected = metadata.get(page_id)
            if expected is None:
                raise CensusError(f"REVERIFY_UNREQUESTED_PAGE:{page_id}")
            title = _page_title(raw.get("title"))
            revisions = raw.get("revisions")
            if not isinstance(revisions, list) or len(revisions) != 1:
                raise CensusError(
                    f"REVERIFY_REVISION_CARDINALITY_INVALID:{page_id}"
                )
            revision = revisions[0]
            if not isinstance(revision, dict):
                raise CensusError(f"REVERIFY_REVISION_INVALID:{page_id}")
            revision_id = _validate_page_id(revision.get("revid"))
            timestamp = revision.get("timestamp")
            if (
                title != expected["title"]
                or revision_id != expected["revision_id"]
                or timestamp != expected["revision_timestamp"]
            ):
                raise CensusError(f"SOURCE_SNAPSHOT_DRIFT:{page_id}")
            seen.add(page_id)
    if seen != set(metadata):
        missing = sorted(set(metadata) - seen)
        raise CensusError(f"REVERIFY_PARTITION_MISMATCH:{missing[0]}")


def _root_surface_index(
    surfaces: list[dict[str, Any]],
) -> dict[str, list[dict[str, Any]]]:
    result: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for surface in surfaces:
        for root_id in surface["root_ids"]:
            result[root_id].append(surface)
    for root_id in result:
        result[root_id].sort(key=lambda row: row["name"].casefold())
    return dict(result)


def classify_source_shape(
    metadata: dict[str, Any],
    discovery_kinds: set[str],
) -> str:
    if metadata["redirect"]:
        return "REDIRECT"
    templates = metadata["templates"]
    if any("infobox" in normalized_title(value) for value in templates):
        return "STRUCTURED_PRIMARY"
    if templates or metadata["categories"]:
        return "STRUCTURED_ALTERNATE"
    if "ROOT_PAGE" in discovery_kinds:
        return "RELATIONSHIP_PAGE"
    if discovery_kinds == {"CATEGORY_MEMBER"}:
        return "CATEGORY_ONLY"
    return "SOURCE_CLASSIFICATION_UNRESOLVED"


def classification_state(families: list[str]) -> str:
    if not families:
        return "SOURCE_NAVIGATION_ONLY"
    if len(families) == 1:
        return "EXACT_FAMILY"
    return "MULTI_FAMILY_RELATION"


def compile_full_output(
    registry: dict[str, Any],
    index: DiscoveryIndex,
    metadata: dict[int, dict[str, Any]],
    root_results: list[dict[str, Any]],
    sealed_lanes: list[dict[str, Any]],
    *,
    request_count: int,
    retrieval_timestamp: str,
    registry_sha256: str,
) -> dict[str, Any]:
    if set(index.by_id) != set(metadata):
        raise CensusError("COMPILE_DISCOVERY_METADATA_SET_MISMATCH")
    root_surfaces = _root_surface_index(registry["surfaces"])
    pages: list[dict[str, Any]] = []
    source_shapes: Counter[str] = Counter()
    family_states: Counter[str] = Counter()
    live_family_candidates: Counter[str] = Counter()
    surface_live_ids: dict[str, set[int]] = defaultdict(set)

    for page_id in sorted(
        metadata,
        key=lambda item: (
            normalized_title(metadata[item]["title"]),
            metadata[item]["title"],
            item,
        ),
    ):
        discovered = index.by_id[page_id]
        if discovered["title"] != metadata[page_id]["title"]:
            raise CensusError(f"COMPILE_TITLE_DRIFT:{page_id}")
        root_ids = sorted(discovered["root_ids"])
        surfaces_by_name: dict[str, dict[str, Any]] = {}
        for root_id in root_ids:
            for surface in root_surfaces[root_id]:
                surfaces_by_name[surface["name"]] = surface
        surfaces = sorted(surfaces_by_name, key=str.casefold)
        families = sorted(
            {
                family
                for surface in surfaces_by_name.values()
                for family in surface["families"]
            }
        )
        dispositions = sorted(
            {surface["disposition"] for surface in surfaces_by_name.values()}
        )
        shape = classify_source_shape(
            metadata[page_id], discovered["discovery_kinds"]
        )
        state = classification_state(families)
        if shape not in SOURCE_SHAPES or state not in FAMILY_STATES:
            raise CensusError("COMPILE_CLASSIFICATION_INTERNAL_ERROR")
        source_shapes[shape] += 1
        family_states[state] += 1
        live_family_candidates.update(families)
        for surface in surfaces:
            surface_live_ids[surface].add(page_id)
        pages.append(
            {
                "source": registry["source"]["id"],
                "source_role": registry["source"]["role"],
                "page_id": page_id,
                "title": metadata[page_id]["title"],
                "revision_id": metadata[page_id]["revision_id"],
                "revision_timestamp": metadata[page_id][
                    "revision_timestamp"
                ],
                "redirect": metadata[page_id]["redirect"],
                "source_shape": shape,
                "family_classification_state": state,
                "candidate_families": families,
                "source_surfaces": surfaces,
                "surface_dispositions": dispositions,
                "discovery_roots": root_ids,
                "discovery_kinds": sorted(discovered["discovery_kinds"]),
                "categories": sorted(
                    metadata[page_id]["categories"], key=str.casefold
                ),
                "templates": sorted(
                    metadata[page_id]["templates"], key=str.casefold
                ),
            }
        )

    protected_item_pages = sum(
        int(row["source_pages"])
        for row in sealed_lanes
        if row["root_id"] == "items-protected"
    )
    if protected_item_pages <= 0:
        raise CensusError("COMPILE_PROTECTED_ITEM_LANE_MISSING")

    protected_by_root = {
        row["root_id"]: int(row["source_pages"]) for row in sealed_lanes
    }
    surface_counts: list[dict[str, Any]] = []
    for surface in registry["surfaces"]:
        protected_pages = sum(
            protected_by_root.get(root_id, 0)
            for root_id in surface["root_ids"]
        )
        live_pages = len(surface_live_ids.get(surface["name"], set()))
        surface_counts.append(
            {
                "surface": surface["name"],
                "disposition": surface["disposition"],
                "candidate_families": surface["families"],
                "live_unique_pages": live_pages,
                "protected_lane_pages": protected_pages,
                "nominal_pages_before_g2_overlap": live_pages
                + protected_pages,
            }
        )

    identity_snapshot = [
        {
            "page_id": page["page_id"],
            "title": page["title"],
            "revision_id": page["revision_id"],
            "revision_timestamp": page["revision_timestamp"],
            "categories": page["categories"],
            "templates": page["templates"],
        }
        for page in pages
    ]
    source_snapshot_digest = sha256_bytes(
        canonical_bytes(
            {
                "live_pages": identity_snapshot,
                "sealed_lanes": [
                    {
                        "root_id": row["root_id"],
                        "source_pages": row["source_pages"],
                        "stable_digest": row["stable_digest"],
                    }
                    for row in sealed_lanes
                ],
            }
        )
    )
    return {
        "schema": SCHEMA,
        "collector_profile": PROFILE,
        "source": registry["source"],
        "retrieval_timestamp": retrieval_timestamp,
        "registry_sha256": registry_sha256,
        "authority": {
            "gameplay_truth": "NONE",
            "identity_resolution": "NOT_PERFORMED",
            "identity_minting": "FORBIDDEN",
            "semantic_promotion": "FORBIDDEN",
            "worldproject_population": "NOT_PERFORMED",
            "long_form_prose_collection": "FORBIDDEN",
            "asset_collection": "FORBIDDEN",
        },
        "sealed_lanes": sealed_lanes,
        "root_results": sorted(root_results, key=lambda row: row["root_id"]),
        "pages": pages,
        "surface_counts": sorted(
            surface_counts, key=lambda row: row["surface"].casefold()
        ),
        "counts": {
            "registry_surfaces": len(registry["surfaces"]),
            "registry_roots": len(registry["roots"]),
            "live_unique_pages": len(pages),
            "protected_item_pages": protected_item_pages,
            "nominal_pages_before_g2_overlap": len(pages)
            + protected_item_pages,
            "global_unique_pages_after_g2_overlap": None,
            "requests": request_count,
            "source_shapes": dict(sorted(source_shapes.items())),
            "family_classification_states": dict(
                sorted(family_states.items())
            ),
            "live_family_candidate_occurrences": dict(
                sorted(live_family_candidates.items())
            ),
            "protected_family_counts": {"Item": protected_item_pages},
        },
        "source_snapshot_sha256": source_snapshot_digest,
    }


def build_manifest(
    full: dict[str, Any],
    registry: dict[str, Any],
    *,
    collector_sha256: str,
) -> dict[str, Any]:
    if full.get("schema") != SCHEMA or not isinstance(full.get("pages"), list):
        raise CensusError("FULL_OUTPUT_INVALID")
    forbidden_keys = {"content", "wikitext", "extract", "text"}
    for page in full["pages"]:
        if not isinstance(page, dict):
            raise CensusError("FULL_PAGE_INVALID")
        if forbidden_keys & set(page):
            raise CensusError("RAW_LONG_FORM_TEXT_IN_OUTPUT")
    stable = dict(full)
    stable.pop("retrieval_timestamp", None)
    exclusion_names = sorted(
        row["surface"] for row in registry["hard_exclusions"]
    )
    surface_names = {row["surface"] for row in full["surface_counts"]}
    if any(name in surface_names for name in exclusion_names):
        raise CensusError("HARD_EXCLUSION_RETAINED")
    return {
        "schema": MANIFEST_SCHEMA,
        "status": "G1_SOURCE_DISCOVERY_ONLY_NO_IDENTITY_PROMOTION",
        "collector": {
            "profile": PROFILE,
            "path": "tools/content-census/source_universe.py",
            "sha256": collector_sha256,
        },
        "registry": {
            "path": "tools/content-census/source-surfaces.json",
            "sha256": full["registry_sha256"],
            "surfaces": full["counts"]["registry_surfaces"],
            "roots": full["counts"]["registry_roots"],
        },
        "source": full["source"],
        "retrieval_timestamp": full["retrieval_timestamp"],
        "counts": full["counts"],
        "surface_counts": full["surface_counts"],
        "source_snapshot_sha256": full["source_snapshot_sha256"],
        "full_output": {
            "schema": SCHEMA,
            "sha256": sha256_bytes(canonical_bytes(full)),
            "stable_without_retrieval_timestamp_sha256": sha256_bytes(
                canonical_bytes(stable)
            ),
            "committed_bulk_corpus": False,
        },
        "invariants": {
            "source_universe_first": True,
            "protected_item_census_reused": True,
            "protected_item_full_census_refetched": False,
            "live_page_ids_deduplicated_before_counts": True,
            "live_page_titles_unique": True,
            "exact_revisions_reverified": True,
            "hard_exclusions_absent": True,
            "identity_resolution_performed": False,
            "identity_minting_performed": False,
            "semantic_promotion_performed": False,
            "worldproject_population_performed": False,
            "raw_long_form_prose_collected": False,
            "assets_collected": False,
        },
        "hard_exclusions": exclusion_names,
        "limitations": [
            (
                "The protected wiki-first Item lane is reused from its compact "
                "repository manifest; G1 does not re-fetch or re-parse the "
                "6,918 Item pages."
            ),
            (
                "Live non-Item pages are deduplicated by exact page ID/title "
                "within G1. Cross-lane and cross-source overlap with the sealed "
                "Item lane is intentionally deferred to G2."
            ),
            (
                "Candidate families are source-surface routing candidates, not "
                "canonical identity matches or promoted gameplay semantics."
            ),
            (
                "Only structured MediaWiki identity/revision/category/template "
                "metadata and bounded discovery links are retained. Article, "
                "quest, book and lore prose is not collected."
            ),
        ],
        "next_gate": "GLOBAL_SOURCE_OVERLAP_AND_DEDUPLICATION",
    }


def collect(
    registry_path: Path,
    *,
    client: Any,
    retrieval_timestamp: str,
) -> tuple[dict[str, Any], dict[str, Any]]:
    registry_raw = load_registry(registry_path)
    registry = validate_registry(registry_raw)
    registry_sha256 = sha256_bytes(canonical_bytes(registry_raw))
    budget = RequestBudget(registry["limits"]["max_requests"])
    index = DiscoveryIndex(
        max_pages=registry["limits"]["max_non_item_pages"],
        exclusion_aliases=registry["exclusion_aliases"],
    )
    root_results: list[dict[str, Any]] = []
    sealed_lanes: list[dict[str, Any]] = []

    for root_id in sorted(registry["roots"]):
        root = registry["roots"][root_id]
        kind = root["kind"]
        if kind == "protected_manifest":
            sealed_lanes.append(load_protected_manifest(root))
        elif kind == "category_tree":
            root_results.append(
                discover_category_tree(
                    client,
                    budget,
                    index,
                    root_id=root_id,
                    title=root["title"],
                    limits=registry["limits"],
                )
            )
        elif kind == "page_links":
            root_results.append(
                discover_page_links(
                    client,
                    budget,
                    index,
                    root_id=root_id,
                    title=root["title"],
                    include_root=root["include_root"],
                    max_links=registry["limits"]["max_links_per_root"],
                )
            )
        elif kind == "page_only":
            root_results.append(
                discover_page_only(
                    client,
                    budget,
                    index,
                    root_id=root_id,
                    title=root["title"],
                )
            )
        elif kind == "registry_only":
            root_results.append(
                {
                    "root_id": root_id,
                    "kind": "registry_only",
                    "root_title": root["title"],
                    "page_ids": [],
                }
            )
        else:
            raise CensusError(f"UNSUPPORTED_ROOT_KIND:{kind}")

    page_ids = sorted(index.by_id)
    metadata = fetch_metadata(
        client,
        budget,
        page_ids,
        limits=registry["limits"],
        exclusion_aliases=registry["exclusion_aliases"],
    )
    verify_revisions(
        client,
        budget,
        metadata,
        batch_size=registry["limits"]["metadata_batch_size"],
    )
    full = compile_full_output(
        registry,
        index,
        metadata,
        root_results,
        sealed_lanes,
        request_count=budget.used,
        retrieval_timestamp=retrieval_timestamp,
        registry_sha256=registry_sha256,
    )
    collector_payload = Path(__file__).read_bytes().replace(b"\r\n", b"\n")
    manifest = build_manifest(
        full,
        registry,
        collector_sha256=sha256_bytes(collector_payload),
    )
    return full, manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--registry", type=Path, default=DEFAULT_REGISTRY)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--manifest-output", type=Path, required=True)
    parser.add_argument("--retrieval-timestamp", default=None)
    args = parser.parse_args()

    retrieval_timestamp = args.retrieval_timestamp or utc_now_iso()
    client = predecessor.ApiClient()
    full, manifest = collect(
        args.registry,
        client=client,
        retrieval_timestamp=retrieval_timestamp,
    )
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(canonical_bytes(full))
    args.manifest_output.parent.mkdir(parents=True, exist_ok=True)
    args.manifest_output.write_bytes(canonical_bytes(manifest))
    counts = manifest["counts"]
    print(
        "full-content-source-discovery: PASS "
        f"live_pages={counts['live_unique_pages']} "
        f"protected_items={counts['protected_item_pages']} "
        f"requests={counts['requests']} "
        f"snapshot={manifest['source_snapshot_sha256']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
