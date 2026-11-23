"""Display bounded source records for human family review; never declare completion.

Uses the reviewed shared scanner's Rust parser helpers. This is a read-only
presentation utility. Macro arguments remain lexical, and displayed excerpts
are explicitly shortened; open the original source for any ambiguous oracle.
"""
import argparse
import importlib.util
import json
import pathlib
import re


def compact(value, limit):
    value = re.sub(r"\s+", " ", value)
    return value if len(value) <= limit else value[:limit] + " [EXCERPT]"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=pathlib.Path, required=True)
    parser.add_argument("--file", required=True)
    parser.add_argument("--function", action="append", default=[])
    parser.add_argument("--limit", type=int, default=220)
    args = parser.parse_args()
    scanner_path = pathlib.Path(__file__).parent.parent / "ci/source_value_scan.py"
    spec = importlib.util.spec_from_file_location("shared_source_scanner", scanner_path)
    scanner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(scanner)
    source = (args.repo / args.file).read_bytes()
    parser = scanner.Parser(scanner.Language(scanner.tree_sitter_rust.language()))
    tree = parser.parse(source)
    print("SOURCE", args.file, scanner.digest(source))
    for node in scanner.walk(tree.root_node):
        if node.type != "function_item":
            continue
        name = node.child_by_field_name("name").text.decode()
        if args.function and name not in args.function:
            continue
        body = node.child_by_field_name("body")
        if body is None:
            continue
        calls, assertions, returns = set(), [], []
        for child in scanner.walk(body):
            if child.type == "call_expression":
                function = child.child_by_field_name("function")
                if function is not None:
                    calls.add(compact(function.text.decode(), args.limit))
            elif child.type == "macro_invocation":
                macro = child.child_by_field_name("macro")
                if macro and re.search(r"(?:^|::)(?:debug_)?assert(?:_eq|_ne)?$", macro.text.decode()):
                    assertions.append((child.start_point.row + 1, macro.text.decode(),
                                       [compact(a, args.limit) for a in scanner.macro_arguments(child, source)]))
            elif child.type == "return_expression":
                returns.append((child.start_point.row + 1, compact(child.text.decode(), args.limit)))
        print("FUNCTION", name, node.start_point.row + 1, node.end_point.row + 1,
              "body_sha256", scanner.digest(body.text))
        print("CALLS", json.dumps(sorted(calls)))
        for assertion in assertions:
            print("ASSERT", json.dumps(assertion))
        for returned in returns:
            print("RETURN", json.dumps(returned))


if __name__ == "__main__":
    main()
