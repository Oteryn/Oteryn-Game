"""Evidence-only callable companion to source_value_scan.py.

Record declared functions, main/run entrypoint candidates, fixture methods and
helpers without claiming they are selected tests. Calls are syntactic candidates,
not resolved call edges. Rust macro arguments stay opaque. No source is imported
or executed. Uses the shared scanner's parser and token/scope helpers.
"""
from __future__ import annotations

import argparse
import ast
import collections
import gzip
import json
import pathlib
import re

import source_value_scan as shared


def rust_scope(node, owner):
    scope = shared.rust_callable_scope(node, owner)
    return "owner" if scope == "test" else scope


def rust_context(node, source):
    context, parent = [], node.parent
    while parent is not None:
        if parent.type in {"mod_item", "impl_item", "trait_item", "function_item"}:
            name = parent.child_by_field_name("name")
            if name is None and parent.type == "impl_item":
                name = parent.child_by_field_name("type")
            context.append({
                "kind": parent.type,
                "name": name.text.decode() if name is not None else None,
                "line": parent.start_point.row + 1,
                "attributes": shared.preceding_attributes(parent, source),
            })
        parent = parent.parent
    return list(reversed(context))


def rust_callables(source, parser):
    root = parser.parse(source).root_node
    errors = [{"line": n.start_point.row + 1, "type": n.type}
              for n in shared.walk(root) if n.type == "ERROR" or n.is_missing]
    contexts = []
    records = []
    for node in shared.walk(root):
        if node.type in {"use_declaration", "macro_definition", "mod_item"}:
            contexts.append({
                "kind": node.type, "line": node.start_point.row + 1,
                "end_line": node.end_point.row + 1,
                "attributes": shared.preceding_attributes(node, source),
                "source_sha256": shared.digest(source[node.start_byte:node.end_byte]),
                "text": source[node.start_byte:node.end_byte].decode()
                if node.type == "use_declaration" else None,
            })
        if node.type != "function_item":
            continue
        body = node.child_by_field_name("body")
        if body is None:
            continue
        attrs = shared.preceding_attributes(node, source)
        name = node.child_by_field_name("name").text.decode()
        assertions, calls, returns, raises, propagation, macros = [], [], [], [], [], []
        for child in shared.walk(body):
            scope = rust_scope(child, node)
            line = child.start_point.row + 1
            expression = source[child.start_byte:child.end_byte].decode()
            if child.type == "call_expression":
                function = child.child_by_field_name("function")
                if function is not None:
                    calls.append({"callee": function.text.decode(), "line": line,
                                  "scope": scope})
            elif child.type == "macro_invocation":
                macro = child.child_by_field_name("macro")
                macro_name = macro.text.decode() if macro is not None else ""
                macros.append({"name": macro_name, "line": line, "scope": scope})
                if re.search(r"(?:^|::)(?:debug_)?assert(?:_eq|_ne)?$", macro_name):
                    assertions.append({"kind": macro_name, "line": line, "scope": scope,
                                       "arguments": shared.macro_arguments(child, source)})
                if re.search(r"(?:^|::)(?:panic|unreachable|todo)$", macro_name):
                    raises.append({"kind": macro_name, "line": line, "scope": scope})
            elif child.type == "return_expression":
                returns.append({"line": line, "scope": scope, "expression": expression,
                                "successful": bool(re.fullmatch(
                                    r"return\s*(?:Ok\s*\(\s*\(\s*\)\s*\))?",
                                    expression.strip()))})
            elif child.type == "try_expression":
                propagation.append({"line": line, "scope": scope,
                                    "expression": expression})
        parameters = node.child_by_field_name("parameters")
        return_type = node.child_by_field_name("return_type")
        records.append({
            "name": name, "line": node.start_point.row + 1,
            "end_line": node.end_point.row + 1,
            "start_byte": node.start_byte, "end_byte": node.end_byte,
            "source_sha256": shared.digest(source[node.start_byte:node.end_byte]),
            "body_sha256": shared.digest(source[body.start_byte:body.end_byte]),
            "body_token_sha256": shared.token_signature(body, source),
            "parameters": parameters.text.decode() if parameters is not None else None,
            "return_type": return_type.text.decode() if return_type is not None else None,
            "attributes": attrs, "ownership_context": rust_context(node, source),
            "entrypoint_candidate": "RUST_TEST_ATTRIBUTE" if any(
                re.search(r"#\[\s*(?:[\w]+::)*test\s*(?:\(|\])", a) for a in attrs)
                else "MAIN_NAME" if name == "main" else "ORDINARY_CALLABLE",
            "calls": calls, "assertions": assertions, "returns": returns,
            "explicit_failure": raises, "error_propagation": propagation,
            "macros": macros,
        })
    return records, contexts, errors


def python_callables(source):
    text = source.decode()
    try:
        root = ast.parse(text)
    except SyntaxError as error:
        return [], [], [{"line": error.lineno, "type": str(error)}]
    parents = {child: parent for parent in ast.walk(root)
               for child in ast.iter_child_nodes(parent)}
    lines = source.splitlines(keepends=True)
    offsets = [0]
    for line in lines:
        offsets.append(offsets[-1] + len(line))

    def span(node):
        return (offsets[node.lineno - 1] + node.col_offset,
                offsets[node.end_lineno - 1] + node.end_col_offset)

    def scope(child, owner):
        parent = parents.get(child)
        while parent is not None and parent is not owner:
            if isinstance(parent, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
                return "nested_callable"
            parent = parents.get(parent)
        return "owner"

    def context(node):
        result, parent = [], parents.get(node)
        while parent is not None:
            if isinstance(parent, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
                result.append({"kind": type(parent).__name__, "name": parent.name,
                               "line": parent.lineno,
                               "decorators": [ast.unparse(d) for d in parent.decorator_list],
                               "bases": [ast.unparse(b) for b in parent.bases]
                               if isinstance(parent, ast.ClassDef) else []})
            parent = parents.get(parent)
        return list(reversed(result))

    contexts = []
    for node in ast.walk(root):
        if isinstance(node, (ast.Import, ast.ImportFrom, ast.ClassDef)):
            contexts.append({"kind": type(node).__name__, "line": node.lineno,
                             "end_line": node.end_lineno,
                             "text": ast.unparse(node) if not isinstance(node, ast.ClassDef)
                             else None,
                             "name": node.name if isinstance(node, ast.ClassDef) else None,
                             "decorators": [ast.unparse(d) for d in node.decorator_list]
                             if isinstance(node, ast.ClassDef) else [],
                             "bases": [ast.unparse(b) for b in node.bases]
                             if isinstance(node, ast.ClassDef) else []})
        if isinstance(node, ast.If) and parents.get(node) is root:
            contexts.append({"kind": "module_if", "line": node.lineno,
                             "condition": ast.unparse(node.test),
                             "calls": [ast.unparse(c.func) for c in ast.walk(node)
                                       if isinstance(c, ast.Call)]})
    records = []
    for node in ast.walk(root):
        if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            continue
        calls, assertions, returns, raises = [], [], [], []
        for child in ast.walk(node):
            current_scope = scope(child, node)
            if isinstance(child, ast.Call):
                function = ast.unparse(child.func)
                calls.append({"callee": function, "line": child.lineno,
                              "scope": current_scope})
                if re.search(r"(?:^|\.)(?:assert\w*|raises|fail)$", function):
                    assertions.append({"kind": function, "line": child.lineno,
                                       "scope": current_scope,
                                       "arguments": [ast.unparse(a) for a in child.args]})
            elif isinstance(child, ast.Assert):
                assertions.append({"kind": "assert", "line": child.lineno,
                                   "scope": current_scope,
                                   "arguments": [ast.unparse(child.test)]})
            elif isinstance(child, ast.Return):
                returns.append({"line": child.lineno, "scope": current_scope,
                                "expression": ast.unparse(child),
                                "successful": child.value is None or isinstance(child.value, ast.Constant)
                                and child.value.value is None})
            elif isinstance(child, ast.Raise):
                raises.append({"line": child.lineno, "scope": current_scope,
                               "expression": ast.unparse(child)})
        start, end = span(node)
        ownership = context(node)
        records.append({
            "name": node.name, "line": node.lineno, "end_line": node.end_lineno,
            "start_byte": start, "end_byte": end,
            "source_sha256": shared.digest(source[start:end]),
            "body_token_sha256": shared.digest(ast.dump(
                ast.Module(body=node.body, type_ignores=[]), include_attributes=False).encode()),
            "parameters": ast.unparse(node.args),
            "return_type": ast.unparse(node.returns) if node.returns else None,
            "attributes": [ast.unparse(d) for d in node.decorator_list],
            "ownership_context": ownership,
            "entrypoint_candidate": "PYTHON_TEST_NAME" if node.name.startswith("test")
                else "FIXTURE_METHOD" if node.name in {"setUp", "setUpClass", "tearDown", "tearDownClass"}
                and ownership and ownership[-1]["kind"] == "ClassDef"
                else "MAIN_RUN_NAME" if node.name in {"main", "run"} else "ORDINARY_CALLABLE",
            "calls": calls, "assertions": assertions, "returns": returns,
            "explicit_failure": raises,
        })
    return sorted(records, key=lambda r: (r["line"], r["end_line"])), contexts, []


def self_test():
    python = b'''import unittest
@unittest.skipUnless(False, "fixture")
class C(unittest.TestCase):
    def setUp(self):
        self.skipTest("no input")
    def test_entry(self):
        def helper():
            assert False
            return
        helper()
def main():
    assert 2 == 2
    raise ValueError("probe")
if __name__ == "__main__":
    main()
'''
    py, context, errors = python_callables(python)
    rust = b'''mod tests { #[test] fn entry() { let future = async { assert!(false); }; helper(); }
fn helper() -> Result<(), ()> { if disabled() { return Ok(()); } fallible()?; Ok(()) }
impl Thing { fn method(&self) { assert_eq!(1, 2); } }
macro_rules! opaque { () => { #[test] fn hidden() {} } }
}'''
    parser = shared.Parser(shared.Language(shared.tree_sitter_rust.language()))
    rs, rust_contexts, rust_errors = rust_callables(rust, parser)
    pmap, rmap = {r["name"]: r for r in py}, {r["name"]: r for r in rs}
    checks = {
        "python_all_declared_callables": set(pmap) == {"setUp", "test_entry", "helper", "main"},
        "python_fixture_owner": pmap["setUp"]["entrypoint_candidate"] == "FIXTURE_METHOD"
            and pmap["setUp"]["ownership_context"][-1]["name"] == "C",
        "python_nested_assert_not_direct": pmap["test_entry"]["assertions"][0]["scope"] == "nested_callable",
        "python_main_guard": any(c.get("kind") == "module_if" and c["calls"] == ["main"] for c in context),
        "python_exact_byte_spans": all(shared.digest(python[r["start_byte"]:r["end_byte"]]) == r["source_sha256"] for r in py),
        "rust_only_declared_functions_not_macro_tokens": set(rmap) == {"entry", "helper", "method"},
        "rust_nested_future_assert": rmap["entry"]["assertions"][0]["scope"] == "nested_future",
        "rust_result_early_return_and_propagation": rmap["helper"]["returns"][0]["successful"] and bool(rmap["helper"]["error_propagation"]),
        "rust_impl_owner": rmap["method"]["ownership_context"][-1]["name"] == "Thing",
        "rust_exact_byte_spans": all(shared.digest(rust[r["start_byte"]:r["end_byte"]]) == r["source_sha256"] for r in rs),
        "valid_probe_syntax": not errors and not rust_errors,
    }
    print(json.dumps(checks, indent=2))
    if not all(checks.values()):
        raise SystemExit(1)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=pathlib.Path)
    parser.add_argument("--assignment", type=pathlib.Path)
    parser.add_argument("--path", action="append", default=[])
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return
    if not args.repo or not args.output or not (args.assignment or args.path):
        parser.error("--repo --output and either --assignment or --path are required")
    repo = args.repo.resolve()
    assigned = json.loads(args.assignment.read_text())["files"] if args.assignment else []
    paths = sorted({r["path"] for r in assigned} | set(args.path))
    rust_parser = shared.Parser(shared.Language(shared.tree_sitter_rust.language()))
    rows = []
    for relative in paths:
        path = repo / relative
        if not path.is_file():
            rows.append({"path": relative, "status": "MISSING_CURRENT_SOURCE"})
            continue
        source = path.read_bytes()
        if path.suffix == ".py":
            records, contexts, errors = python_callables(source)
        elif path.suffix == ".rs":
            records, contexts, errors = rust_callables(source, rust_parser)
        else:
            records, contexts, errors = [], [], [{"type": "UNSUPPORTED_SYNTAX"}]
        rows.append({"path": relative, "sha256": shared.digest(source),
                     "bytes": len(source), "callables": records,
                     "source_context": contexts, "parse_errors": errors})
    result = {"schema": "test-callable-source-evidence-v1",
              "scanner_sha256": shared.digest(pathlib.Path(__file__).read_bytes()),
              "shared_scanner_sha256": shared.digest(pathlib.Path(shared.__file__).read_bytes()),
              "limits": [__doc__, "Entrypoint markers are syntax/name candidates, not actual selection or execution.",
                         "Assertions in nested functions/futures may be uncalled/unpolled. Caller evidence is required.",
                         "Function calls inside Rust macro token trees are not resolved. Parse errors stay explicit.",
                         "Python imports/attributes can be dynamic. Syntactic call names do not establish target identity.",
                         "Fixture methods and ordinary helpers are not counted as executed tests."],
              "summary": {"paths": len(rows), "callables": sum(len(r.get("callables", [])) for r in rows),
                          "paths_with_parse_errors": sum(bool(r.get("parse_errors")) for r in rows),
                          "entrypoint_candidates": dict(collections.Counter(
                              c["entrypoint_candidate"] for r in rows for c in r.get("callables", [])))},
              "files": rows}
    raw = (json.dumps(result, ensure_ascii=False, indent=2) + "\n").encode()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_bytes(gzip.compress(raw, mtime=0) if args.output.suffix == ".gz" else raw)
    print(json.dumps(result["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
