"""Error code registry loader and validator (ARCH-ERROR-CODES-0 §1.3)."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GAME_PATH = "docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json"
PROTOCOL_PATH = "docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json"
BASE_REF = "origin/main"

CATEGORIES = frozenset(
    {
        "INVALID_INPUT",
        "UNSUPPORTED_REVISION",
        "AUTHENTICATION_FAILED",
        "SESSION_REJECTED",
        "STALE_GENERATION",
        "CONFLICT",
        "CAPACITY_EXCEEDED",
        "DEPENDENCY_UNAVAILABLE",
        "TIMEOUT",
        "CANCELLED",
        "INTERNAL_UNAVAILABLE",
    }
)
PROGRESSIONS = frozenset({"RETRYABLE", "TERMINAL", "SECURITY_TERMINAL"})
STATUSES = frozenset({"ACTIVE", "RETIRED"})
# §1.3 table: default_disposition -> (progression, retry requires).
DISPOSITION_PROGRESSION = {
    "TRANSPORT_FATAL": ("TERMINAL", "a new connection and session"),
    "SESSION_FATAL": ("TERMINAL", "a new connection and session"),
    "OPERATION_TERMINAL": ("TERMINAL", "a new command in the same session"),
    "RESYNC_REQUIRED": ("RETRYABLE", "the resync, then the same session"),
}
GAME_FIELDS = {"code", "name", "category", "progression", "owner", "contract", "hint", "status"}
GAME_OPTIONAL = {"public_class"}
NAME_RE = re.compile(r"^[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)*$")
HINT_MAX = 160
GAME_BLOCK_FIRST, GAME_BLOCK_LAST = 2000, 9999
WIRE_FIRST, WIRE_LAST = 1000, 1999


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _ref_readable(root: Path, ref: str) -> bool:
    verify = subprocess.run(
        ["git", "-C", str(root), "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}"],
        capture_output=True,
        text=True,
    )
    return verify.returncode == 0


def _fetch_base(root: Path, ref: str) -> None:
    """Best effort: a checkout of the exact head has no `origin/main`."""
    remote, _, branch = ref.partition("/")
    subprocess.run(
        ["git", "-C", str(root), "fetch", "--quiet", "--depth=1", remote,
         f"+refs/heads/{branch}:refs/remotes/{ref}"],
        capture_output=True,
        text=True,
    )


def load_base_json(rel_path: str, root: Path = ROOT, ref: str = BASE_REF):
    """The file at `ref`: a dict, `None` when it does not exist there, or
    `False` when the ref cannot be read even after a fetch."""
    if not _ref_readable(root, ref):
        _fetch_base(root, ref)
    if not _ref_readable(root, ref):
        return False
    shown = subprocess.run(
        ["git", "-C", str(root), "show", f"{ref}:{rel_path}"], capture_output=True, text=True
    )
    if shown.returncode != 0:
        return None
    return json.loads(shown.stdout)


def derived_progression(entry: dict):
    """(progression, retry_requires, derived) for a protocol entry, or None
    when its disposition is outside the §1.3 table and it has no explicit
    progression."""
    if "progression" in entry:
        return entry["progression"], None, False
    row = DISPOSITION_PROGRESSION.get(entry.get("default_disposition"))
    if row is None:
        return None
    return row[0], row[1], True


def _block_of(number: int, blocks: list[dict]):
    for block in blocks:
        if block["first"] <= number <= block["last"]:
            return block
    return None


def _is_u32(value) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and 0 < value <= 0xFFFFFFFF


def validate_blocks(game: dict) -> list[str]:
    errors: list[str] = []
    blocks = game.get("blocks")
    if not isinstance(blocks, list) or not blocks:
        return ["game registry: blocks must be a non-empty list"]
    previous_last = 0
    for block in blocks:
        if not (
            isinstance(block, dict)
            and _is_u32(block.get("first"))
            and _is_u32(block.get("last"))
            and block["first"] <= block["last"]
            and block.get("registry") in {"protocol", "game", "none"}
            and isinstance(block.get("owner"), str)
            and isinstance(block.get("contents"), str)
        ):
            errors.append(f"game registry: malformed block {block!r}")
            continue
        if block["first"] <= previous_last:
            errors.append(f"game registry: block {block['first']} overlaps or is out of order")
        previous_last = block["last"]
    return errors


def validate_game(game: dict, protocol_names: dict[str, int]) -> list[str]:
    errors = validate_blocks(game)
    if game.get("schema_version") != 1:
        errors.append("game registry: schema_version must be 1")
    codes = game.get("codes")
    if not isinstance(codes, list):
        return errors + ["game registry: codes must be a list"]
    blocks = game.get("blocks") if isinstance(game.get("blocks"), list) else []
    seen_numbers: set[int] = set()
    seen_names: dict[str, int] = dict(protocol_names)
    for entry in codes:
        if not isinstance(entry, dict):
            errors.append(f"game registry: entry is not an object: {entry!r}")
            continue
        label = f"game code {entry.get('code')!r}"
        keys = set(entry)
        if not GAME_FIELDS <= keys or keys - GAME_FIELDS - GAME_OPTIONAL:
            errors.append(f"{label}: fields must be {sorted(GAME_FIELDS)} plus optional public_class")
            continue
        number, name = entry["code"], entry["name"]
        if not _is_u32(number):
            errors.append(f"{label}: code must be a positive u32")
            continue
        block = _block_of(number, [b for b in blocks if isinstance(b, dict) and "first" in b])
        if block is None or block.get("registry") != "game":
            errors.append(f"{label}: number is not in a Game-registry block")
        if number in seen_numbers:
            errors.append(f"{label}: number is duplicated")
        seen_numbers.add(number)
        if not isinstance(name, str) or not NAME_RE.match(name):
            errors.append(f"{label}: name must be SCREAMING_SNAKE_CASE")
        elif name in seen_names:
            errors.append(f"{label}: name {name} is already registered as {seen_names[name]}")
        else:
            seen_names[name] = number
        if entry["category"] not in CATEGORIES:
            errors.append(f"{label}: unknown category {entry['category']!r}")
        if entry["progression"] not in PROGRESSIONS:
            errors.append(f"{label}: unknown progression {entry['progression']!r}")
        if entry["status"] not in STATUSES:
            errors.append(f"{label}: status must be ACTIVE or RETIRED")
        for key in ("owner", "hint"):
            if not isinstance(entry[key], str) or not entry[key]:
                errors.append(f"{label}: {key} must be a non-empty string")
        if entry["contract"] is not None and not isinstance(entry["contract"], str):
            errors.append(f"{label}: contract must be a string or null")
        hint = entry["hint"]
        if isinstance(hint, str) and (len(hint.encode("utf-8")) > HINT_MAX or "\n" in hint):
            errors.append(f"{label}: hint must be one line of at most {HINT_MAX} bytes")
        if "public_class" in entry and (
            not isinstance(entry["public_class"], str) or not entry["public_class"]
        ):
            errors.append(f"{label}: public_class must be a non-empty string")
    return errors


def validate_protocol(protocol: dict, game_numbers: set[int]) -> tuple[list[str], dict[str, int]]:
    errors: list[str] = []
    names: dict[str, int] = {}
    seen: set[int] = set()
    entries = protocol.get("error_codes")
    if not isinstance(entries, list):
        return ["protocol registry: error_codes must be a list"], names
    for entry in entries:
        label = f"protocol code {entry.get('code')!r}" if isinstance(entry, dict) else "protocol entry"
        if not isinstance(entry, dict):
            errors.append(f"{label}: not an object")
            continue
        number, name = entry.get("code"), entry.get("name")
        if not _is_u32(number) or not WIRE_FIRST <= number <= WIRE_LAST:
            errors.append(f"{label}: number must lie in 1000-1999")
            continue
        if number in seen or number in game_numbers:
            errors.append(f"{label}: number is duplicated")
        seen.add(number)
        if not isinstance(name, str) or not NAME_RE.match(name):
            errors.append(f"{label}: name must be SCREAMING_SNAKE_CASE")
        elif name in names:
            errors.append(f"{label}: name {name} is duplicated")
        else:
            names[name] = number
        if entry.get("category") not in CATEGORIES:
            errors.append(f"{label}: unknown category {entry.get('category')!r}")
        if "progression" in entry and entry["progression"] not in PROGRESSIONS:
            errors.append(f"{label}: unknown progression {entry['progression']!r}")
        if derived_progression(entry) is None:
            errors.append(
                f"{label}: default_disposition {entry.get('default_disposition')!r} is outside "
                "the progression table and the entry has no explicit progression"
            )
    return errors, names


def _by_code(entries: list[dict]) -> dict[int, dict]:
    return {e["code"]: e for e in entries if isinstance(e, dict) and "code" in e}


def compare_to_base(
    current: list[dict], base: list[dict], immutable: tuple[str, ...], label: str
) -> list[str]:
    """Append-only and no-semantic-edit check of one registry's entries."""
    errors: list[str] = []
    cur, old = _by_code(current), _by_code(base)
    base_names = {e["name"]: n for n, e in old.items() if "name" in e}
    for number, was in old.items():
        now = cur.get(number)
        if now is None:
            if base_names.get(was.get("name")) in cur or any(
                e.get("name") == was.get("name") for e in cur.values()
            ):
                errors.append(f"{label} {number}: renumbered ({was.get('name')} moved)")
            else:
                errors.append(f"{label} {number}: removed (a retired code stays with status RETIRED)")
            continue
        for key in immutable:
            if now.get(key) != was.get(key):
                errors.append(f"{label} {number}: {key} changed {was.get(key)!r} -> {now.get(key)!r}")
        if "public_class" in was:
            if now.get("public_class") != was["public_class"]:
                errors.append(f"{label} {number}: public_class changed or removed")
        if "status" in was and now.get("status") != was["status"]:
            if not (was["status"] == "ACTIVE" and now.get("status") == "RETIRED"):
                errors.append(f"{label} {number}: status may only change ACTIVE -> RETIRED")
    return errors


def compare_blocks(current: list, base: list) -> list[str]:
    """Blocks are never split, renumbered or reassigned; new ones go above 9999."""
    errors: list[str] = []
    cur = {b["first"]: b for b in current if isinstance(b, dict) and "first" in b}
    for was in base:
        now = cur.get(was["first"])
        if now is None:
            errors.append(f"block {was['first']}-{was['last']}: removed, split or renumbered")
            continue
        for key in ("last", "registry"):
            if now.get(key) != was[key]:
                errors.append(f"block {was['first']}: {key} changed {was[key]!r} -> {now.get(key)!r}")
    old_firsts = {b["first"] for b in base}
    for first, block in cur.items():
        if first not in old_firsts and first <= 9999:
            errors.append(f"block {first}: new blocks must start above 9999")
    return errors


def validate_files(
    game: dict, protocol: dict, base_game=None, base_protocol=None
) -> list[str]:
    """Validate both registries; the bases are dicts, or None for a new file."""
    game_codes = game.get("codes") if isinstance(game.get("codes"), list) else []
    game_numbers = {e["code"] for e in game_codes if isinstance(e, dict) and "code" in e}
    errors, protocol_names = validate_protocol(protocol, game_numbers)
    errors += validate_game(game, protocol_names)
    if isinstance(base_game, dict):
        errors += compare_blocks(game.get("blocks", []), base_game.get("blocks", []))
        errors += compare_to_base(
            game_codes, base_game.get("codes", []), ("name", "category", "progression"), "game code"
        )
    if isinstance(base_protocol, dict):
        errors += compare_to_base(
            protocol.get("error_codes", []),
            base_protocol.get("error_codes", []),
            ("name", "category", "default_disposition", "progression"),
            "protocol code",
        )
    return errors


def validate(root: Path = ROOT, base_ref: str = BASE_REF) -> list[str]:
    game = load_json(root / GAME_PATH)
    protocol = load_json(root / PROTOCOL_PATH)
    base_game = load_base_json(GAME_PATH, root, base_ref)
    base_protocol = load_base_json(PROTOCOL_PATH, root, base_ref)
    errors: list[str] = []
    if base_game is False or base_protocol is False:
        errors.append(
            f"{base_ref} is not readable, so the append-only check cannot run; "
            f"fetch it (git fetch origin main) and retry"
        )
        base_game = base_protocol = None
    return errors + validate_files(game, protocol, base_game, base_protocol)


def main() -> int:
    errors = validate()
    for error in errors:
        print(f"- {error}")
    return 1 if errors else 0


if __name__ == "__main__":
    raise SystemExit(main())
