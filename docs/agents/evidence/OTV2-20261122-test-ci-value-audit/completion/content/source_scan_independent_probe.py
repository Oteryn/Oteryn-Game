"""Replay the independent scanner probes; keep the original failed receipt unchanged."""
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCANNER = HERE.parent / "ci" / "source_value_scan.py"
spec = importlib.util.spec_from_file_location("shared_scan", SCANNER)
scan = importlib.util.module_from_spec(spec)
spec.loader.exec_module(scan)
original = json.loads((HERE / "source_scan_independent_probe.json").read_text())
parser = scan.Parser(scan.Language(scan.tree_sitter_rust.language()))
results = {}
for name, text in original["examples"].items():
    tests, context, errors = scan.rust_records(text.encode(), parser)
    results[name] = {"tests": tests, "context": context, "errors": errors}
python_tests, python_context, errors = scan.python_records(original["python_example"].encode())
results["python_class_skip_and_nested_return"] = {"tests": python_tests, "context": python_context, "errors": errors}
checks = {
    "rust_comment_formatting_equal": results["rust_comments"]["tests"][0]["body_token_sha256"] == results["rust_comments"]["tests"][1]["body_token_sha256"],
    "rust_string_whitespace_distinct": results["rust_literal_whitespace"]["tests"][0]["body_token_sha256"] != results["rust_literal_whitespace"]["tests"][1]["body_token_sha256"],
    "rust_return_scope_separated": sorted(r["scope"] for r in results["rust_return_scope"]["tests"][0]["returns"]) == ["nested_callable", "test"],
    "rust_macro_body_not_claimed": len(results["rust_macro_opaque"]["tests"]) == 1,
    "python_class_skip_not_detected": all("skip_path" not in t["risk_candidates"] for t in python_tests),
    "python_environ_subscript_not_detected": "environment_dependency" not in python_tests[1]["risk_candidates"],
    "python_nested_return_conservative_flag": "return_path_requires_review" in python_tests[0]["risk_candidates"],
}
supplement = {
    "python_class_decorator_is_visible": all(t.get("class_context", [{}])[0].get("decorators") for t in python_tests),
    "python_environment_access_is_visible": any(c["kind"] == "environment_access" for c in python_context),
    "python_nested_return_scope_is_explicit": python_tests[0]["returns"][0].get("scope") == "nested_callable",
}
result = {"scanner_sha256": hashlib.sha256(SCANNER.read_bytes()).hexdigest(),
          "original_expectations_unchanged": checks,
          "new_metadata_checks": supplement,
          "interpretation": "The original class/environment absence observations were method limits, not desired invariants. Their changed truth values are retained, not rewritten. Comment equivalence is the repaired correctness check.",
          "results": results}
(HERE / "source_scan_independent_probe_after.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({"original_expectations_unchanged": checks, "new_metadata_checks": supplement}, indent=2))
assert all(checks[k] for k in ("rust_comment_formatting_equal", "rust_string_whitespace_distinct", "rust_return_scope_separated", "rust_macro_body_not_claimed"))
assert all(supplement.values())
