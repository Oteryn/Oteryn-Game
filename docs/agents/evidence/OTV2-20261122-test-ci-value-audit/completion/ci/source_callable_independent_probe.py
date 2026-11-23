"""Independent root controls for the evidence extractor; no product source edits."""
import hashlib
import json
from pathlib import Path

import source_callable_scan as scan


def run():
    py_source = '''# Zażółć: byte offsets must not be character offsets.
class Fixture:
    def setUp(self):
        self.skipTest("unavailable external input")
    async def test_async(self):
        def nested():
            assert "łódź" == "łódź"
        await producer()
        nested()
def main():
    if condition():
        raise RuntimeError("failed contract")
if __name__ == "__main__":
    main()
'''.encode()
    py, py_context, py_errors = scan.python_callables(py_source)
    pmap = {row["name"]: row for row in py}
    rs_source = b'''#[cfg(test)] mod tests {
#[test] fn outer() -> Result<(), ()> {
    fn never_called() { assert!(false); }
    let never_polled = async { assert_eq!(1, 2); return; };
    fallible()?;
    Ok(())
}
fn helper() { assert!(condition()); }
macro_rules! opaque { () => { #[test] fn not_expanded() {} } }
}'''
    parser = scan.shared.Parser(scan.shared.Language(scan.shared.tree_sitter_rust.language()))
    rs, _, rs_errors = scan.rust_callables(rs_source, parser)
    rmap = {row["name"]: row for row in rs}
    checks = {
        "python_utf8_byte_spans": all(
            hashlib.sha256(py_source[r["start_byte"]:r["end_byte"]]).hexdigest()
            == r["source_sha256"] for r in py),
        "python_nested_assert_is_not_direct":
            pmap["test_async"]["assertions"][0]["scope"] == "nested_callable",
        "python_fixture_is_not_test":
            pmap["setUp"]["entrypoint_candidate"] == "FIXTURE_METHOD",
        "python_fixture_skip_is_visible_call": any(
            c["callee"] == "self.skipTest" for c in pmap["setUp"]["calls"]),
        "python_main_failure_visible": bool(pmap["main"]["explicit_failure"]),
        "python_main_guard_visible": any(c.get("condition") == "__name__ == '__main__'"
            for c in py_context),
        "rust_nested_function_and_future_are_not_direct":
            {a["scope"] for a in rmap["outer"]["assertions"]}
            == {"nested_callable", "nested_future"},
        "rust_error_propagation_is_visible": bool(rmap["outer"]["error_propagation"]),
        "rust_macro_defined_function_is_not_claimed": "not_expanded" not in rmap,
        "rust_helper_is_not_test":
            rmap["helper"]["entrypoint_candidate"] == "ORDINARY_CALLABLE",
        "rust_ownership_context_visible":
            rmap["outer"]["ownership_context"][-1]["name"] == "tests",
        "fixtures_parse_without_errors": not py_errors and not rs_errors,
    }
    result = {
        "kind": "independent-extractor-controls-not-product-tests",
        "callable_scanner_sha256": hashlib.sha256(Path(scan.__file__).read_bytes()).hexdigest(),
        "shared_scanner_sha256": hashlib.sha256(Path(scan.shared.__file__).read_bytes()).hexdigest(),
        "checks": checks, "passed": sum(checks.values()), "total": len(checks),
        "limitations": [
            "These controls qualify bounded extraction behavior, not all Rust/Python grammar.",
            "Names and candidate call edges do not prove selected tests, dynamic call targets or execution.",
        ],
    }
    Path(__file__).with_suffix(".json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    if not all(checks.values()):
        raise SystemExit(1)


if __name__ == "__main__":
    run()
