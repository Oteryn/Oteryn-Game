"""Negative controls for audit extraction; this does not run product tests."""
import json
from tree_sitter import Language, Parser
import tree_sitter_rust
import source_value_scan as scan


def main():
    parser = Parser(Language(tree_sitter_rust.language()))
    def cases(source):
        tests, _, errors = scan.rust_records(source.encode(), parser)
        assert not errors, errors
        return tests

    a = cases('#[test] fn a(){ assert_eq!("a b", "c"); }')[0]
    b = cases('#[test] fn b(){ /* ignored { } */ assert_eq!( "a b" , "c" ); }')[0]
    different = cases('#[test] fn c(){ assert_eq!("ab", "c"); }')[0]
    assert a['body_token_sha256'] == b['body_token_sha256']
    assert a['body_token_sha256'] != different['body_token_sha256']
    nested = cases('#[test] fn a(){ let unused = || { assert!(false); return; }; }')[0]
    assert 'no_direct_assertion' in nested['risk_candidates']
    assert 'assertions_only_in_nested_callable' in nested['risk_candidates']
    assert 'successful_early_return' not in nested['risk_candidates']
    future = cases('#[test] fn a(){ let unused = async { assert!(false); return; }; }')[0]
    assert future['assertions'][0]['callable_scope'] == 'nested_future'
    assert future['returns'][0]['scope'] == 'nested_future'
    assert 'successful_early_return' not in future['risk_candidates']
    early = cases('#[test] fn a(){ if disabled() { return; } assert!(observed()); }')[0]
    assert 'successful_early_return' in early['risk_candidates']
    assert 'no_direct_assertion' not in early['risk_candidates']
    macro = cases('#[test] fn a(){ assert_eq!(observed(), expected()); }')[0]
    assert macro['ast_calls'] == [], 'calls inside token-tree macros must not be claimed as expression AST calls'
    assert macro['assertions'][0]['arguments'] == ['observed()', 'expected()']
    python, context, errors = scan.python_records(b'''import os
import unittest
@unittest.skip("missing fixture")
class Cases(unittest.TestCase):
    def setUpClass():
        raise unittest.SkipTest("fixture")
    def test_value(self):
        def unused():
            assert False
            return
        value = os.environ["INPUT"]
''')
    assert not errors and len(python) == 1
    flags = python[0]['risk_candidates']
    for flag in ['no_direct_assertion','assertions_only_in_nested_callable',
                 'class_or_fixture_skip_candidate','environment_dependency']:
        assert flag in flags, (flag, flags)
    assert python[0]['returns'][0]['scope'] == 'nested_callable'
    assert any(c['kind'] == 'ClassDef' and c['fixture_methods'] for c in context)
    print(json.dumps({'result':'PASS','controls':[
        'Rust comments/formatting preserve token identity',
        'Rust whitespace inside string literals changes token identity',
        'Uninvoked nested assertions are not direct assertions',
        'Nested return is not a successful outer-test early return',
        'Unpolled async Future assertions and returns remain separate',
        'Outer successful early return is a review candidate',
        'Macro argument calls remain explicitly opaque',
        'Python class/fixture skip and environ subscript are visible',
        'Python nested assertion/return scope remains separate'
    ]},indent=2))


if __name__ == '__main__':
    main()
