"""Load the contract registries that are stored as one file per entry (REGISTRY-SPLIT-1).

`PROTOCOL_OTERYN_V1_REGISTRY.json` and `RESOURCE_LIMITS_REGISTRY.json` are no longer
committed. Each is a base file plus one fragment file per array entry. The aggregate text
is the base with each placeholder line `  "<key>": [],` replaced by the fragments in order,
every fragment line indented by four spaces. `crates/protocol-oteryn/build.rs` assembles
the same text for Rust; keep the two in step.

Order: a protocol fragment is named `<id>-<slug>.json` and sorts by `id`; a resource-limit
fragment is named `<ordinal>-<id>.json` and sorts by ordinal, then file name.
"""

from __future__ import annotations

import json
import re
import subprocess
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
_NAME = re.compile(r"^([0-9]+)-[A-Za-z0-9._-]+\.json$")


@dataclass(frozen=True)
class Registry:
    legacy_path: str
    base_path: str
    collections: tuple[str, ...]
    order: str  # "id": the name prefix is the numeric `id`; "ordinal": it only orders
    required: dict[str, tuple[str, ...]] = field(default_factory=dict)


PROTOCOL = Registry(
    legacy_path="docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json",
    base_path="docs/contracts/protocol-oteryn/v1/registry/registry.json",
    collections=("capabilities", "command_types", "state_domains"),
    order="id",
    required={
        "capabilities": (
            "id", "name", "owner_decision", "owner", "command_types", "state_domains", "offered",
        ),
        "command_types": (
            "id", "name", "owner_decision", "payload_schema", "result_schema",
            "max_payload_bytes", "max_result_payload_bytes",
        ),
        "state_domains": (
            "id", "name", "owner_decision", "owner", "delta_types", "snapshot_types",
        ),
    },
)
# Required entry fields come from the base's `required_entry_fields`.
RESOURCE_LIMITS = Registry(
    legacy_path="docs/contracts/RESOURCE_LIMITS_REGISTRY.json",
    base_path="docs/contracts/resource-limits/registry.json",
    collections=("entries",),
    order="ordinal",
)


def collection_dir(registry: Registry, key: str) -> str:
    return f"{registry.base_path.rsplit('/', 1)[0]}/{key}"


def _sort_key(name: str) -> tuple[int, str]:
    match = _NAME.match(name)
    if match is None:
        raise ValueError(f"fragment {name} is not named <number>-<slug>.json")
    return int(match.group(1)), name


def assemble(base_text: str, fragments: dict[str, list[tuple[str, str]]]) -> str:
    """The aggregate text from the base and `{key: [(file name, text)]}` (any order)."""
    lines = base_text.replace("\r\n", "\n").split("\n")
    for key, items in fragments.items():
        hits = [i for i, line in enumerate(lines) if line in (f'  "{key}": [],', f'  "{key}": []')]
        if len(hits) != 1:
            raise ValueError(f'base needs exactly one `  "{key}": [],` line, found {len(hits)}')
        index = hits[0]
        if not items:
            continue
        comma = lines[index][len(f'  "{key}": []') :]
        bodies = []
        for name, text in sorted(items, key=lambda item: _sort_key(item[0])):
            text = text.replace("\r\n", "\n")
            if not text.endswith("\n") or text.endswith("\n\n"):
                raise ValueError(f"fragment {name} must end with exactly one newline")
            bodies.append("\n".join("    " + line for line in text[:-1].split("\n")))
        lines[index : index + 1] = [f'  "{key}": [', ",\n".join(bodies), f"  ]{comma}"]
    return "\n".join(lines)


def _read_tree(registry: Registry, read, listdir) -> str:
    fragments = {}
    for key in registry.collections:
        directory = collection_dir(registry, key)
        fragments[key] = [(name, read(f"{directory}/{name}")) for name in listdir(directory)]
    return assemble(read(registry.base_path), fragments)


def _listdir(root: Path, directory: str) -> list[str]:
    path = root / directory
    if not path.is_dir():
        return []
    return sorted(child.name for child in path.iterdir() if child.suffix == ".json")


def load_text(registry: Registry, root: Path = ROOT) -> str:
    """The aggregate JSON text, byte for byte the file the split replaced."""
    return _read_tree(
        registry,
        lambda rel: (root / rel).read_text(encoding="utf-8"),
        lambda rel: _listdir(root, rel),
    )


def load(registry: Registry, root: Path = ROOT) -> dict:
    return json.loads(load_text(registry, root))


def source_files(registry: Registry, root: Path = ROOT) -> list[str]:
    """Every committed file the aggregate is built from, relative to `root`."""
    files = [registry.base_path]
    for key in registry.collections:
        directory = collection_dir(registry, key)
        files.extend(f"{directory}/{name}" for name in _listdir(root, directory))
    return files


def load_at_ref(registry: Registry, root: Path, ref: str):
    """The aggregate at git `ref` as a dict, or `None` when it does not exist there.

    A ref from before the split has only `legacy_path`; that file is read instead.
    """

    def git(*arguments: str) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["git", "-C", str(root), *arguments], capture_output=True, text=True, encoding="utf-8"
        )

    if git("cat-file", "-e", f"{ref}:{registry.base_path}").returncode != 0:
        legacy = git("show", f"{ref}:{registry.legacy_path}")
        return json.loads(legacy.stdout) if legacy.returncode == 0 else None

    def read(rel: str) -> str:
        shown = git("show", f"{ref}:{rel}")
        if shown.returncode != 0:
            raise ValueError(f"{ref}:{rel} is not readable")
        return shown.stdout

    def listdir(rel: str) -> list[str]:
        listed = git("ls-tree", "--name-only", f"{ref}:{rel}")
        if listed.returncode != 0:
            return []
        return sorted(name for name in listed.stdout.splitlines() if name.endswith(".json"))

    return json.loads(_read_tree(registry, read, listdir))


def validate(registry: Registry, root: Path = ROOT) -> list[str]:
    """Problems with the fragment tree: names, ids, order and required fields."""
    try:
        base = json.loads((root / registry.base_path).read_text(encoding="utf-8"))
        aggregate = load(registry, root)
    except (OSError, ValueError) as error:
        return [f"{registry.base_path}: {error}"]
    errors: list[str] = []
    for key in registry.collections:
        if base.get(key) != []:
            errors.append(f"{registry.base_path}: `{key}` must be [] (its entries are fragments)")
        required = registry.required.get(key) or tuple(base.get("required_entry_fields", ()))
        directory = collection_dir(registry, key)
        names = sorted(_listdir(root, directory), key=_sort_key)
        seen: set[str] = set()
        for name, entry in zip(names, aggregate.get(key, [])):
            where = f"{directory}/{name}"
            if not isinstance(entry, dict):
                errors.append(f"{where}: must be a JSON object")
                continue
            missing = [field for field in required if field not in entry]
            if missing:
                errors.append(f"{where}: missing {missing}")
            identifier = entry.get("id")
            prefix = name.split("-", 1)[0]
            if registry.order == "id":
                if type(identifier) is not int:
                    errors.append(f"{where}: `id` must be an integer")
                elif identifier != int(prefix):
                    errors.append(f"{where}: name prefix {prefix} is not id {identifier}")
            elif type(identifier) is not str or name != f"{prefix}-{identifier}.json":
                errors.append(f"{where}: must be named <ordinal>-<id>.json")
            folded = str(identifier).lower()
            if folded in seen:
                errors.append(f"{where}: duplicate id {identifier}")
            seen.add(folded)
    return errors


def split(registry: Registry, aggregate_text: str):
    """(base text, {key: [(file name, text)]}) for an aggregate: the inverse of `assemble`."""
    data = json.loads(aggregate_text)
    lines = aggregate_text.split("\n")
    fragments: dict[str, list[tuple[str, str]]] = {}
    for key in registry.collections:
        start = lines.index(f'  "{key}": [')
        end = next(i for i in range(start + 1, len(lines)) if lines[i] in ("  ]", "  ],"))
        bodies: list[list[str]] = []
        for line in lines[start + 1 : end]:
            if not line.startswith("    "):
                raise ValueError(f"{key}: line not indented by four spaces: {line[:60]}")
            if line.startswith("    {"):
                bodies.append([])
            bodies[-1].append(line[4:])
        if len(bodies) != len(data[key]):
            raise ValueError(f"{key}: {len(bodies)} bodies for {len(data[key])} entries")
        items = []
        for ordinal, (entry, body) in enumerate(zip(data[key], bodies), start=1):
            if body[-1].endswith(","):
                body[-1] = body[-1][:-1]
            if registry.order == "id":
                slug = entry["name"].lower().replace("_", "-")
                name = f"{entry['id']:04d}-{slug}.json"
            else:
                name = f"{ordinal:04d}-{entry['id']}.json"
            items.append((name, "\n".join(body) + "\n"))
        fragments[key] = items
        lines[start : end + 1] = [f'  "{key}": []{lines[end][3:]}']
    return "\n".join(lines), fragments
