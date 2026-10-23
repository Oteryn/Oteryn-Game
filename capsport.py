#!/usr/bin/env python3
"""Port additive old-layout capabilities during a merge; never commit or publish.

Usage: python3 capsport.py <worktree> <merge-base-sha-or-auto>
Reads HEAD (ours), MERGE_HEAD (theirs), and the base, never conflict-marker text.
Unsupported Rust expressions, unrelated shared-file edits, or uncertain facts fail
with exit 3 before writing. Successful runs stage only the files they resolve.
"""

import functools
import json
from pathlib import Path
import re
import subprocess
import sys

PROTOCOL = "crates/protocol-oteryn/src/"
DISPATCH = "crates/gameplay/dispatch/src/"
CONTRACTS = "docs/contracts/protocol-oteryn/v1/registry/capabilities/"
FRAGMENTS = "crates/protocol-oteryn/capabilities/"
GOLDEN = PROTOCOL + "tests/capabilities_golden.rs"
SHARED = [PROTOCOL + p for p in (
    "lib.rs", "registered_capabilities.rs", "capability_tables.rs")]
SHARED += [DISPATCH + "gameplay_transport/capabilities.rs"]
SETS = {"production": "PRODUCTION", "item_view": "ITEM_VIEW", "bundle_world": "BUNDLE_WORLD"}


class Refusal(Exception):
    pass


def git(root, *args, optional=False):
    result = subprocess.run(["git", "-C", str(root), *args], capture_output=True)
    if result.returncode and not optional:
        raise Refusal(f"git {' '.join(args)}: {result.stderr.decode().strip()}")
    return None if result.returncode else result.stdout


def clean(text):
    return re.sub(r"/\*.*?\*/|//[^\n]*", "", text, flags=re.S)


def parts(text):
    """Split only commas outside balanced Rust brackets; reject unknown syntax later."""
    result, start, stack = [], 0, []
    pairs = {"[": "]", "(": ")", "{": "}"}
    for index, char in enumerate(text):
        if char in pairs:
            stack.append(pairs[char])
        elif char in "])}":
            if not stack or stack.pop() != char:
                raise Refusal("unbalanced Rust expression")
        elif char == "," and not stack:
            result.append(text[start:index].strip())
            start = index + 1
    if stack:
        raise Refusal("unbalanced Rust expression")
    result.append(text[start:].strip())
    return [part for part in result if part]


class Sources:
    def __init__(self, root, revision):
        self.root, self.revision = root, revision

    @functools.lru_cache(maxsize=None)
    def read(self, path):
        data = git(self.root, "show", f"{self.revision}:{path}", optional=True)
        return None if data is None else data.decode()

    def files(self, prefix):
        return git(self.root, "ls-tree", "-r", "--name-only", self.revision, "--", prefix).decode().splitlines()

    def module(self, scope):
        prefix = PROTOCOL if scope[0] == "protocol" else DISPATCH
        relative = "/".join(scope[1:])
        candidates = [prefix + relative + ".rs", prefix + relative + "/mod.rs"] if relative else [prefix + "lib.rs"]
        for path in candidates:
            text = self.read(path)
            if text is not None:
                return clean(text)
        raise Refusal(f"cannot resolve Rust module {'::'.join(scope)} at {self.revision}")

    @staticmethod
    def imports(text):
        imports = {}

        def expand(expression, prefix=""):
            expression = expression.strip()
            if "{" in expression:
                stem, body = expression.split("{", 1)
                for item in parts(body.rsplit("}", 1)[0]):
                    expand(item, prefix + stem.strip())
                return
            path, *alias = re.split(r"\s+as\s+", expression)
            if path == "*":
                return
            if path == "self":
                path = prefix.rstrip(":")
            else:
                path = prefix + path
            imports[alias[0] if alias else path.split("::")[-1]] = path

        for match in re.finditer(r"\buse\s+([^;]+);", text):
            expand(match[1])
        return imports

    @functools.lru_cache(maxsize=None)
    def constant(self, scope, name):
        text = self.module(scope)
        match = re.search(r"\bconst\s+" + re.escape(name) + r"\s*:\s*[^=]+?=\s*([^;]+);", text)
        if match:
            return self.expression(scope, match[1])
        imported = self.imports(text).get(name)
        if imported:
            return self.reference(scope, imported)
        raise Refusal(f"cannot resolve {'::'.join(scope)}::{name}")

    def reference(self, scope, path):
        terms = path.split("::")
        imported = self.imports(self.module(scope)).get(terms[0])
        if imported and imported != terms[0]:
            terms = imported.split("::") + terms[1:]
        if terms[0] == "oteryn_protocol_oteryn":
            target, terms = ["protocol"], terms[1:]
        elif terms[0] == "crate":
            target, terms = [scope[0]], terms[1:]
        elif terms[0] == "super":
            target = list(scope)
            while terms and terms[0] == "super":
                target.pop()
                terms = terms[1:]
        else:
            target = list(scope)
            if terms[0] == "self":
                terms = terms[1:]
        return self.constant(tuple(target + terms[:-1]), terms[-1])

    def expression(self, scope, text):
        text = text.strip().lstrip("&").strip()
        if re.fullmatch(r"[0-9][0-9_]*", text):
            return int(text.replace("_", ""))
        if text.startswith("[") and text.endswith("]"):
            return [self.expression(scope, part) for part in parts(text[1:-1])]
        if text.startswith("(") and text.endswith(")"):
            return tuple(self.expression(scope, part) for part in parts(text[1:-1]))
        if re.match(r"(?:[\w]+::)*OfferedCapability\s*\{", text) and text.endswith("}"):
            values = {}
            for item in parts(text.split("{", 1)[1][:-1]):
                field, value = item.split(":", 1)
                values[field.strip()] = self.expression(scope, value)
            if set(values) != {"id", "requires"}:
                raise Refusal("unsupported OfferedCapability fields")
            return values["id"], values["requires"]
        if re.fullmatch(r"\w+(?:::\w+)*", text):
            return self.reference(scope, text)
        raise Refusal(f"unsupported Rust capability expression: {text[:100]}")

    def registered(self):
        for scope in [("protocol",), ("protocol", "registered_capabilities")]:
            try:
                return self.constant(scope, "REGISTERED_CAPABILITY_IDS_V1")
            except Refusal:
                continue
        raise Refusal("old registered capability list not found")

    def facts(self):
        contracts = {}
        for path in self.files(CONTRACTS):
            if path.endswith(".json"):
                entry = json.loads(self.read(path))
                contracts[entry["id"]] = entry
        registered = self.registered()
        if set(registered) != set(contracts):
            raise Refusal("registered ids and contract fragments differ")
        requirements = dict(self.constant(("protocol",), "CAPABILITY_REQUIREMENTS_V1"))
        gates = {row[0]: row[1:] for row in self.constant(("protocol", "capability_tables"), "GATED")}
        supersedes = dict(self.constant(("protocol", "capability_tables"), "SUPERSEDED"))
        offers = {name: dict(self.constant(("dispatch", "gameplay_transport", "capabilities"),
                                         prefix + "_OFFERED_CAPABILITIES")) for name, prefix in SETS.items()}
        constants = {}
        for path in self.files(PROTOCOL):
            if not path.endswith(".rs"):
                continue
            for name, number in re.findall(r"\bconst\s+(CAPABILITY_[A-Z0-9_]+)\s*:\s*u32\s*=\s*([0-9_]+)\s*;", clean(self.read(path))):
                module = path[len(PROTOCOL):].removesuffix(".rs").replace("/", "::")
                constants.setdefault(int(number.replace("_", "")), []).append(module + "::" + name)
        result = []
        for ident in registered:
            contract = contracts[ident]
            candidates = [p for p in constants.get(ident, []) if p.endswith("::CAPABILITY_" + contract["name"])]
            if len(candidates) != 1:
                raise Refusal(f"capability {ident}: cannot determine unique codec constant: {candidates}")
            commands, domains = gates.get(ident, ([], None))
            requires = contract.get("requires", [])
            if commands != contract["command_types"] or (domains != contract["state_domains"] and ident != 6):
                raise Refusal(f"capability {ident}: gate and contract facts differ")
            if ident in requirements and requirements[ident] != requires:
                raise Refusal(f"capability {ident}: acceptance requirements differ")
            if any(table[ident] != requires for table in offers.values() if ident in table):
                raise Refusal(f"capability {ident}: offered requirements differ")
            result.append(dict(id=ident, name=contract["name"], constant=candidates[0], requires=requires,
                               acceptance_requires=ident in requirements, commands=commands, domains=domains,
                               supersedes=supersedes.get(ident, []), offers={k: ident in v for k, v in offers.items()}))
        return result


def golden_text(entries):
    return ("// Explicit accepted facts. Update only for accepted capability-set changes.\n[\n"
            + "".join('    r#"' + json.dumps(entry, indent=2) + '"#,\n' for entry in entries) + "]\n")


def normalize_tables(text):
    for name in ("REGISTERED_CAPABILITY_IDS_V1", "CAPABILITY_REQUIREMENTS_V1", "GATED",
                 *(value + "_OFFERED_CAPABILITIES" for value in SETS.values())):
        text = re.sub(r"(?m)^(?:pub(?:\([^)]*\))?\s+)?const\s+" + name + r"\s*:[^=;]+=\s*[^;]+;", "", text)
    # Stale prose is also removed in the layout migration, never retained as runtime facts.
    return re.sub(r"(?m)^\s*//[^\n]*\n", "", text)


def without_added_list_ids(text, added):
    """Recognize only literal numeric snapshot lists, not arbitrary test edits."""
    def replace(match):
        values = [int(part.strip()) for part in match[1].split(",") if part.strip()]
        return "[" + ",".join(str(value) for value in values if value not in added) + "]"
    return re.sub(r"\[([0-9,\s]+)\]", replace, clean(text))


def port(root, base_arg):
    root = root.resolve()
    if Path(git(root, "rev-parse", "--show-toplevel").decode().strip()).resolve() != root:
        raise Refusal("argument must name the worktree root")
    ours = git(root, "rev-parse", "HEAD").decode().strip()
    merge_path = Path(git(root, "rev-parse", "--git-path", "MERGE_HEAD").decode().strip())
    if not merge_path.is_absolute():
        merge_path = root / merge_path
    heads = merge_path.read_text().splitlines()
    if len(heads) != 1:
        raise Refusal("exactly one MERGE_HEAD is required")
    theirs = heads[0]
    base = git(root, "merge-base", ours, theirs).decode().strip() if base_arg == "auto" else git(root, "rev-parse", "--verify", base_arg + "^{commit}").decode().strip()
    for revision in (ours, theirs):
        git(root, "merge-base", "--is-ancestor", base, revision)
    old, ancestor, main = (Sources(root, revision) for revision in (ours, base, theirs))
    before = {entry["id"]: entry for entry in ancestor.facts()}
    after = {entry["id"]: entry for entry in old.facts()}
    if not before.keys() <= after.keys() or any(after[i] != entry for i, entry in before.items()):
        raise Refusal("branch changed existing capability facts; only additive ports are supported")
    added = [entry for ident, entry in after.items() if ident not in before]
    if not added:
        raise Refusal("branch adds no old-layout capabilities relative to merge base")
    snapshot = main.read(GOLDEN)
    if snapshot is None:
        raise Refusal("MERGE_HEAD has no CAPS-SPLIT-1 golden fixture")
    expected = [json.loads(text) for text in re.findall(r'r#"(.*?)"#', snapshot, re.S)]
    main_entries = sorted((json.loads(main.read(path)) for path in main.files(FRAGMENTS)
                           if path.endswith(".json")), key=lambda entry: entry["id"])
    if expected != main_entries or not expected:
        raise Refusal("main golden fixture and runtime fragments differ")
    if any(entry["id"] in {item["id"] for item in expected} for entry in added):
        raise Refusal("a branch capability id is already allocated on main")
    changes = {}
    unresolved = git(root, "diff", "--name-only", "--diff-filter=U", "-z").decode().split("\0")
    unresolved = set(filter(None, unresolved))
    for path in sorted(unresolved - set(SHARED)):
        branch, original, target = old.read(path), ancestor.read(path), main.read(path)
        if (not path.startswith((PROTOCOL, DISPATCH + "gameplay_transport/"))
                or "test" not in Path(path).stem or not all((branch, original, target))):
            raise Refusal(f"unhandled conflict (resolve separately first): {path}")
        ident_set = {entry["id"] for entry in added}
        left = without_added_list_ids(branch, ident_set)
        right = without_added_list_ids(original, ident_set)
        if re.sub(r"\s+", "", left) != re.sub(r"\s+", "", right):
            raise Refusal(f"{path}: conflict includes edits beyond additive pinned id lists")
        changes[path] = target.encode()
    for path in SHARED:
        target = main.read(path)
        if target is None:
            raise Refusal(f"main is missing {path}")
        branch, original = old.read(path), ancestor.read(path)
        if branch != original and branch is not None:
            # Tables go to theirs; preserve independent declarations, e.g. pub mod guild_bank.
            left, right = normalize_tables(branch), normalize_tables(original or "")
            additions = set(re.findall(r"(?m)^(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+;", left)) - set(re.findall(r"(?m)^(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+;", right))
            for declaration in additions:
                left = left.replace(declaration, "")
            if re.sub(r"\s+", "", left) != re.sub(r"\s+", "", right):
                raise Refusal(f"{path}: unrelated edits outside capability tables; cannot safely choose theirs")
            for declaration in sorted(additions):
                if declaration not in target:
                    target += "\n" + declaration + "\n"
        changes[path] = target.encode()
    for entry in added:
        filename = f'{entry["id"]:04}-{entry["name"].lower().replace("_", "-")}.json'
        data = (json.dumps(entry, indent=2) + "\n").encode()
        destination = root / FRAGMENTS / filename
        if destination.exists() and destination.read_bytes() != data:
            raise Refusal(f"{destination}: existing fragment differs; preserve it before porting")
        changes[FRAGMENTS + filename] = data
        contract_path = CONTRACTS + filename
        if (root / contract_path).read_text() != old.read(contract_path):
            raise Refusal(f"{contract_path}: merged contract differs from branch; resolve first")
    changes[GOLDEN] = golden_text(sorted(expected + added, key=lambda entry: entry["id"])).encode()
    # Do not overwrite independent unstaged edits; conflict markers are the only expected edits.
    dirty = set(filter(None, git(root, "diff", "--name-only", "-z").decode().split("\0")))
    if (dirty & set(changes)) - unresolved:
        raise Refusal("unstaged edits in port destinations; preserve or stage them first")
    changed = []
    for path, data in changes.items():
        destination = root / path
        if not destination.exists() or destination.read_bytes() != data or path in unresolved:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
            changed.append(path)
    if changed:
        git(root, "add", "--", *changed)
    for path in changed:
        print(path)
    print("ported capability ids: " + ", ".join(str(entry["id"]) for entry in added))


def main():
    try:
        if len(sys.argv) != 3:
            raise Refusal("usage: python3 capsport.py <worktree> <merge-base-sha-or-auto>")
        port(Path(sys.argv[1]), sys.argv[2])
    except (Refusal, OSError, ValueError, KeyError, TypeError, RecursionError) as error:
        print(f"capsport: cannot determine a safe port: {error}", file=sys.stderr)
        return 3
    return 0


if __name__ == "__main__":
    sys.exit(main())
