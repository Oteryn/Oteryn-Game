"""Reproducible source evidence for the test-value audit, not execution/coverage.

Requires tree-sitter==0.25.2 and tree-sitter-rust==0.24.0 for Rust syntax trees.
Python uses the standard ast module. Every potential risk remains a candidate
until a human reviews its body, fixtures, helpers and selection. A parsed body
or a clean risk scan never establishes that a test is necessary or sufficient.
"""
from __future__ import annotations

import argparse
import ast
import collections
import hashlib
import json
import pathlib
import re
import subprocess
import tomllib

from tree_sitter import Language, Parser
import tree_sitter_rust


def digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def walk(node):
    yield node
    for child in node.children:
        yield from walk(child)


def preceding_attributes(node, source):
    result = []
    sibling = node.prev_named_sibling
    while sibling is not None and sibling.type in {
        "attribute_item", "line_comment", "block_comment"
    }:
        if sibling.type == "attribute_item":
            result.append(source[sibling.start_byte:sibling.end_byte].decode())
        sibling = sibling.prev_named_sibling
    return list(reversed(result))


def token_signature(node, source):
    # Preserve literal contents, punctuation and token boundaries. In particular,
    # never remove whitespace inside strings when looking for duplicate bodies.
    parts = []
    def visit(child):
        if child.type in {"line_comment", "block_comment"}:
            return
        if child.children:
            for descendant in child.children:
                visit(descendant)
        else:
            parts.append(source[child.start_byte:child.end_byte])
    visit(node)
    return digest(b"\0".join(parts))


def macro_arguments(node, source):
    tree = next((c for c in node.children if c.type == "token_tree"), None)
    if tree is None:
        return []
    arguments, current = [], []
    for child in tree.children[1:-1]:
        value = source[child.start_byte:child.end_byte]
        if child.type == ",":
            arguments.append(b"".join(current).strip().decode())
            current = []
        elif child.type not in {"line_comment", "block_comment"}:
            current.append(value)
    if current:
        arguments.append(b"".join(current).strip().decode())
    return arguments


def rust_callable_scope(node, owner):
    ancestor = node.parent
    while ancestor is not None and ancestor != owner:
        if ancestor.type == "async_block":
            return "nested_future"
        if ancestor.type in {"closure_expression", "function_item"}:
            return "nested_callable"
        ancestor = ancestor.parent
    return "test"


def rust_records(source, parser):
    tree = parser.parse(source)
    root = tree.root_node
    tests, contexts, errors = [], [], []
    for node in walk(root):
        if node.type == "ERROR" or node.is_missing:
            errors.append({"line": node.start_point.row + 1, "type": node.type})
        if node.type in {"mod_item", "macro_definition", "macro_invocation"}:
            # Context index only; macro-defined tests are not silently credited
            # as ordinary parsed function items.
            if node.type != "macro_invocation" or b"include" in source[node.start_byte:node.start_byte+30]:
                contexts.append({"kind": node.type, "line": node.start_point.row + 1,
                                 "name": node.child_by_field_name("name").text.decode()
                                 if node.child_by_field_name("name") else None,
                                 "attributes": preceding_attributes(node, source),
                                 "has_body": node.child_by_field_name("body") is not None,
                                 "include_expression": source[node.start_byte:node.end_byte].decode()
                                 if node.type == "macro_invocation" else None})
        if node.type != "function_item":
            continue
        attrs = preceding_attributes(node, source)
        if not any(re.search(r"#\[\s*(?:[\w]+::)*test\s*(?:\(|\])", a) for a in attrs):
            continue
        body = node.child_by_field_name("body")
        name = node.child_by_field_name("name").text.decode()
        ancestors, parent = [], node.parent
        while parent is not None:
            if parent.type == "mod_item":
                ancestors.append({"name": parent.child_by_field_name("name").text.decode(),
                                  "attributes": preceding_attributes(parent, source)})
            parent = parent.parent
        raw = source[body.start_byte:body.end_byte]
        assertions, calls, returns, macros = [], set(), [], []
        for child in walk(body):
            text = source[child.start_byte:child.end_byte].decode()
            if child.type == "call_expression":
                function = child.child_by_field_name("function")
                if function:
                    calls.add(function.text.decode())
            if child.type == "macro_invocation":
                macro = child.child_by_field_name("macro")
                macro_name = macro.text.decode() if macro else ""
                macros.append(macro_name)
                if re.search(r"(?:^|::)(?:debug_)?assert(?:_eq|_ne)?$", macro_name):
                    arguments = macro_arguments(child, source)
                    assertions.append({"macro": macro_name, "line": child.start_point.row + 1,
                                       "arguments": arguments,
                                       "callable_scope": rust_callable_scope(child, node),
                                       "self_equality": macro_name.endswith("assert_eq")
                                       and len(arguments) >= 2 and arguments[0] == arguments[1]})
                # Macro token trees are opaque to expression AST parsing. Keep
                # these calls explicitly lexical, not resolved callee claims.
            if child.type == "return_expression":
                scope, context, ancestor = "test", "", child.parent
                while ancestor is not None and ancestor != node:
                    if ancestor.type == "async_block":
                        scope = "nested_future"
                        break
                    if ancestor.type in {"closure_expression", "function_item"}:
                        scope = "nested_callable"
                        break
                    if not context and ancestor.type in {"if_expression", "let_declaration", "match_arm"}:
                        context = source[ancestor.start_byte:min(ancestor.end_byte, ancestor.start_byte+700)].decode()
                    ancestor = ancestor.parent
                returns.append({"line": child.start_point.row + 1, "expression": text,
                                "scope": scope, "context": context,
                                "successful": bool(re.fullmatch(r"return\s*(?:Ok\s*\(\s*\(\s*\)\s*\))?", text.strip()))})
        assertion_text = "\n".join(" ".join(a["arguments"]) for a in assertions)
        raw_text = raw.decode()
        flags = []
        if not any(a["callable_scope"] == "test" for a in assertions):
            flags.append("no_direct_assertion")
        if assertions and all(a["callable_scope"] != "test" for a in assertions):
            flags.append("assertions_only_in_nested_callable")
        if any(a["self_equality"] for a in assertions):
            flags.append("self_equality")
        if any(r["successful"] and r["scope"] == "test" for r in returns):
            flags.append("successful_early_return")
        if any(re.search(r"\benv\s*::\s*(?:var|var_os)\b", c) for c in calls):
            flags.append("environment_dependency")
        if any("ignore" in a for a in attrs):
            flags.append("ignored_attribute")
        if re.search(r"\.find\s*\([^;\n]*\)\s*[<>]\s*\w+\.find\s*\(", assertion_text):
            flags.append("option_find_ordering")
        if re.search(r"\.all\s*\(", assertion_text):
            flags.append("all_quantifier_needs_nonempty_fixture")
        if "include_str" in macros or any("read_to_string" in c for c in calls):
            flags.append("text_input_or_source_oracle")
        if any(re.fullmatch(r"(?:true|false|[0-9_]+)", a["arguments"][0] if a["arguments"] else "")
               for a in assertions):
            flags.append("literal_first_assertion_argument")
        tests.append({"name": name, "line": node.start_point.row + 1,
                      "end_line": node.end_point.row + 1, "attributes": attrs,
                      "module_context": list(reversed(ancestors)),
                      "body_sha256": digest(raw), "body_token_sha256": token_signature(body, source),
                      "body_bytes": len(raw), "assertions": assertions,
                      "macro_argument_limit": "Argument token trees are not expression ASTs; generic commas may split argument candidates. Review source before accepting equality candidates.",
                      "ast_calls": sorted(calls), "macros": sorted(set(macros)),
                      "returns": returns, "risk_candidates": flags})
    return tests, contexts, errors


def python_records(source):
    text = source.decode()
    try:
        root = ast.parse(text)
    except SyntaxError as error:
        return [], [], [{"line": error.lineno, "type": str(error)}]
    tests, contexts = [], []
    all_parents = {child: parent for parent in ast.walk(root) for child in ast.iter_child_nodes(parent)}
    for item in ast.walk(root):
        if isinstance(item, ast.ClassDef):
            contexts.append({"kind": "ClassDef", "name": item.name, "line": item.lineno,
                             "decorators": [ast.unparse(d) for d in item.decorator_list],
                             "bases": [ast.unparse(b) for b in item.bases],
                             "fixture_methods": [{"name": f.name, "line": f.lineno,
                                 "end_line": f.end_lineno,
                                 "source_sha256": digest(ast.get_source_segment(text, f).encode())}
                                 for f in item.body if isinstance(f, (ast.FunctionDef, ast.AsyncFunctionDef))
                                 and f.name in {"setUp", "setUpClass", "tearDown", "tearDownClass"}]})
        if isinstance(item, ast.Attribute) and item.attr == "environ":
            contexts.append({"kind": "environment_access", "line": item.lineno,
                             "text": ast.unparse(item)})
    for node in ast.walk(root):
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            contexts.append({"kind": type(node).__name__, "line": node.lineno,
                             "text": ast.get_source_segment(text, node)})
        if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) or not node.name.startswith("test"):
            continue
        calls, assertions, returns = set(), [], []
        class_context, ancestor = [], all_parents.get(node)
        while ancestor is not None:
            if isinstance(ancestor, ast.ClassDef):
                fixtures = [f for f in ancestor.body if isinstance(f, (ast.FunctionDef, ast.AsyncFunctionDef))
                            and f.name in {"setUp", "setUpClass"}]
                class_context.append({"name": ancestor.name,
                                      "decorators": [ast.unparse(d) for d in ancestor.decorator_list],
                                      "bases": [ast.unparse(b) for b in ancestor.bases],
                                      "fixture_skip_candidate": any("skipTest" in ast.unparse(f)
                                                                    or "SkipTest" in ast.unparse(f) for f in fixtures)})
            ancestor = all_parents.get(ancestor)
        parents = {child: parent for parent in ast.walk(node) for child in ast.iter_child_nodes(parent)}
        def scope(child):
            parent = parents.get(child)
            while parent is not None and parent is not node:
                if isinstance(parent, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
                    return "nested_callable"
                parent = parents.get(parent)
            return "test"
        for child in ast.walk(node):
            if isinstance(child, ast.Call):
                function = ast.unparse(child.func)
                calls.add(function)
                if re.search(r"(?:^|\.)(?:assert\w*|raises|fail)$", function):
                    arguments = [ast.unparse(a) for a in child.args]
                    assertions.append({"macro": function, "line": child.lineno, "arguments": arguments,
                                       "callable_scope": scope(child),
                                       "self_equality": function.endswith("assertEqual") and len(arguments) >= 2
                                       and ast.dump(child.args[0]) == ast.dump(child.args[1])})
            if isinstance(child, ast.Assert):
                assertions.append({"macro": "assert", "line": child.lineno,
                                   "arguments": [ast.unparse(child.test)],
                                   "callable_scope": scope(child),
                                   "self_equality": isinstance(child.test, ast.Compare)
                                   and len(child.test.ops) == 1 and isinstance(child.test.ops[0], ast.Eq)
                                   and ast.dump(child.test.left) == ast.dump(child.test.comparators[0])})
            if isinstance(child, ast.Return):
                returns.append({"line": child.lineno, "expression": ast.unparse(child),
                                "scope": scope(child),
                                "successful": child.value is None or isinstance(child.value, ast.Constant)
                                and child.value.value is None})
        raw = ast.get_source_segment(text, node).encode()
        body_dump = ast.dump(ast.Module(body=node.body, type_ignores=[]), include_attributes=False)
        flags = []
        if not any(a["callable_scope"] == "test" for a in assertions): flags.append("no_direct_assertion")
        if assertions and all(a["callable_scope"] != "test" for a in assertions):
            flags.append("assertions_only_in_nested_callable")
        if any(a["self_equality"] for a in assertions): flags.append("self_equality")
        if returns: flags.append("return_path_requires_review")
        if any("environ" in c or c.endswith("getenv") for c in calls) or any(
            isinstance(c, ast.Attribute) and c.attr == "environ" for c in ast.walk(node)
        ): flags.append("environment_dependency")
        if any("skip" in c for c in calls) or any("skip" in ast.unparse(a) for a in node.decorator_list):
            flags.append("skip_path")
        if any(c["fixture_skip_candidate"] or any("skip" in d for d in c["decorators"]) for c in class_context):
            flags.append("class_or_fixture_skip_candidate")
        if any("read_text" in c or c.endswith("open") for c in calls): flags.append("text_input_or_source_oracle")
        tests.append({"name": node.name, "line": node.lineno, "end_line": node.end_lineno,
                      "attributes": [ast.unparse(d) for d in node.decorator_list],
                      "class_context": class_context,
                      "body_sha256": digest(raw), "body_token_sha256": digest(body_dump.encode()),
                      "body_bytes": len(raw), "assertions": assertions, "ast_calls": sorted(calls),
                      "returns": returns, "risk_candidates": flags})
    return tests, contexts, []


def package_context(repo, path):
    directory = path.parent
    while directory.is_relative_to(repo):
        manifest = directory / "Cargo.toml"
        if manifest.is_file():
            try:
                value = tomllib.loads(manifest.read_text())
            except tomllib.TOMLDecodeError as error:
                return {"manifest": str(manifest.relative_to(repo)),
                        "manifest_sha256": digest(manifest.read_bytes()),
                        "parse_error": str(error),
                        "selection_limit": "Python tomllib could not parse this manifest; inspect the real Cargo owner separately"}
            return {"manifest": str(manifest.relative_to(repo)),
                    "name": value.get("package", {}).get("name"),
                    "features": value.get("features", {}),
                    "explicit_tests": value.get("test", []),
                    "selection_limit": "Manifest/source context only; no compiled-ID or execution claim"}
        directory = directory.parent
    return None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=pathlib.Path, required=True)
    parser.add_argument("--assignment", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    args = parser.parse_args()
    repo = args.repo.resolve()
    assignment = json.loads(args.assignment.read_text())
    rust = Parser(Language(tree_sitter_rust.language()))
    files, duplicates, risks, packages = [], collections.defaultdict(list), [], {}
    for assigned in assignment["files"]:
        rel = assigned["path"]
        path = repo / rel
        if not path.is_file():
            files.append({**assigned, "status": "MISSING_CURRENT_SOURCE"})
            continue
        source = path.read_bytes()
        if path.suffix == ".rs":
            tests, context, errors = rust_records(source, rust)
            package = package_context(repo, path)
            if package: packages[package["manifest"]] = package
            language = "rust_tree_sitter"
        elif path.suffix == ".py":
            tests, context, errors = python_records(source)
            package = None
            language = "python_ast"
        else:
            tests, context, errors, package, language = [], [], [], None, "UNSUPPORTED_SYNTAX"
        row = {**assigned, "sha256": digest(source), "bytes": len(source),
               "status": "PARSED_SOURCE_EVIDENCE" if not errors and language != "UNSUPPORTED_SYNTAX"
                         else "PARSE_OR_LANGUAGE_REVIEW_REQUIRED",
               "method": language, "package_manifest": package["manifest"] if package else None,
               "parse_errors": errors, "tests": tests, "selection_context": context}
        if not tests: row["unresolved_selection"] = "No ordinary parsed test function: inspect module/macro/dynamic/runner ownership"
        files.append(row)
        for test in tests:
            ref = {"path": rel, "test": test["name"], "line": test["line"]}
            duplicates[test["body_token_sha256"]].append(ref)
            if test["risk_candidates"]:
                risks.append({**ref, "risk_candidates": test["risk_candidates"]})
    duplicate_groups = [{"body_token_sha256": k, "cases": v} for k,v in duplicates.items() if len(v)>1]
    result = {"schema": "test-source-value-evidence-v1", "owner": assignment["owner"],
              "baseline": assignment["baseline"],
              "observed_worktree_head": subprocess.check_output(["git","rev-parse","HEAD"],cwd=repo,text=True).strip(),
              "scanner_sha256": digest(pathlib.Path(__file__).read_bytes()),
              "method_limits": [__doc__, "AST macro token trees are not expanded; unknown/dynamic selectors remain explicit.",
                  "Source matches, assertions, calls, defaults, manifests and risk candidates are evidence for review, not proof of necessity, execution or coverage.",
                  "Duplicate token bodies require independent fixture/helper/module/cfg reconciliation before consolidation.",
                  "Python nested return paths are conservative candidates; all() and text input flags are not defects by themselves."],
              "summary": {"assigned_paths":len(files),"parsed_test_functions":sum(len(f.get("tests",[])) for f in files),
                  "paths_with_parse_errors":sum(bool(f.get("parse_errors")) for f in files),
                  "paths_without_ordinary_tests":sum(not f.get("tests") for f in files),
                  "risk_counts":dict(collections.Counter(k for r in risks for k in r["risk_candidates"])),
                  "duplicate_groups":len(duplicate_groups)},
              "packages":list(packages.values()),"duplicate_groups":duplicate_groups,"risk_index":risks,"files":files}
    args.output.parent.mkdir(parents=True,exist_ok=True)
    args.output.write_text(json.dumps(result,ensure_ascii=False,indent=2)+"\n")
    print(json.dumps(result["summary"],sort_keys=True))


if __name__ == "__main__":
    main()
