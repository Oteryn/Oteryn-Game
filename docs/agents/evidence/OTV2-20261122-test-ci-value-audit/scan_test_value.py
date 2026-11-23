from __future__ import annotations
import ast
from collections import Counter, defaultdict
import csv
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

import argparse
parser = argparse.ArgumentParser(description='Inventory source tests; this does not execute or semantically review them.')
parser.add_argument('--repo', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
ROOT = args.repo.resolve()
OUT = args.output.resolve()
OUT.mkdir(parents=True, exist_ok=True)
HEAD = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
tracked_records = subprocess.check_output(['git', 'ls-tree', '-r', 'HEAD'], cwd=ROOT, text=True).splitlines()
blobs = {row.split('\t', 1)[1]: row.split('\t', 1)[0].split()[2] for row in tracked_records}
tracked = list(blobs)
members = tomllib.loads((ROOT / 'Cargo.toml').read_text().split('[workspace.package]',1)[0])['workspace']['members']
test_attr = re.compile(r'^\s*#\[\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*test\s*(?:\]|\()', re.M)
ignore_attr = re.compile(r'^\s*#\[ignore(?:\s*=\s*"([^"\n]*)")?\]', re.M)
def group(path):
    for member in sorted(members, key=len, reverse=True):
        if path.startswith(member + '/'):
            return member
    bits = path.split('/')
    return '/'.join(bits[:3] if path.startswith('tools/content-schema/') else bits[:2])

rows = []
same_asserts = []
python_no_discovery = []
ignored = []
python_duplicates = defaultdict(list)
missing = []
syntax_errors = []
source_files = Counter()
source_groups = defaultdict(Counter)
modified = []
for rel in tracked:
    p = Path(rel)
    if p.suffix not in {'.rs', '.py'} or p.parts[0] not in {'apps','crates','tools','tests','deploy'}:
        continue
    file = ROOT / rel
    if not file.is_file():
        missing.append(rel)
        continue
    raw = file.read_bytes()
    if hashlib.sha1(f'blob {len(raw)}\0'.encode()+raw).hexdigest() != blobs[rel]:
        modified.append(rel)
    text = raw.decode()
    source_files[p.suffix] += 1
    source_groups[group(rel)][p.suffix] += 1
    row = dict(path=rel, group=group(rel), language=p.suffix[1:], bytes=file.stat().st_size,
               rust_test_attributes=0, python_test_definitions=0, python_test_classes=0,
               ignored_attributes=0, source_text_reference_hint=False, env_gate_hint=False,
               historical_name_hint=False, semantic_review_status='UNREVIEWED_STATIC_SCAN')
    row['historical_name_hint'] = bool(re.search(r'legacy|pre-native|spike|prototype|evidence', rel))
    row['source_text_reference_hint'] = bool(re.search(r'read_to_string|read_text\(|include_str!|read_bytes\(', text))
    row['env_gate_hint'] = bool(re.search(r'env::var|os\.environ|getenv|skipUnless|skipIf|skipTest', text))
    if p.suffix == '.rs':
        row['rust_test_attributes'] = len(test_attr.findall(text))
        matches = list(ignore_attr.finditer(text))
        row['ignored_attributes'] = len(matches)
        for match in matches:
            name = re.search(r'(?:async\s+)?fn\s+(\w+)', text[match.end():])
            ignored.append(dict(path=rel, line=text.count('\n',0,match.start())+1,
                                test=name[1] if name else None, reason=match[1]))
        candidate = bool(row['rust_test_attributes'])
    else:
        try:
            tree = ast.parse(text, filename=rel)
        except SyntaxError as err:
            syntax_errors.append(dict(path=rel, line=err.lineno, error=err.msg))
            continue
        tests = [n for n in ast.walk(tree) if isinstance(n,(ast.FunctionDef,ast.AsyncFunctionDef)) and n.name.startswith('test_')]
        classes = [n for n in ast.walk(tree) if isinstance(n,ast.ClassDef) and any((isinstance(b,ast.Attribute) and b.attr=='TestCase') or (isinstance(b,ast.Name) and b.id=='TestCase') for b in n.bases)]
        row['python_test_definitions'] = len(tests)
        row['python_test_classes'] = len(classes)
        candidate = bool(tests or classes or p.name.startswith('test_') or p.name.endswith('_test.py') or p.name=='self_test.py')
        if candidate and not tests and not classes:
            python_no_discovery.append(dict(path=rel, has_main_guard='__main__' in text,
                                            functions=[n.name for n in tree.body if isinstance(n,(ast.FunctionDef,ast.AsyncFunctionDef))]))
        for node in tests:
            body = node.body[1:] if node.body and isinstance(node.body[0], ast.Expr) and isinstance(node.body[0].value, ast.Constant) and isinstance(node.body[0].value.value,str) else node.body
            normalized = '\n'.join(ast.dump(n, include_attributes=False) for n in body)
            if len(normalized)>120:
                python_duplicates[hashlib.sha256(normalized.encode()).hexdigest()].append(dict(path=rel,line=node.lineno,test=node.name,bytes=len(normalized)))
            for n in ast.walk(node):
                if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr in {'assertEqual','assertIs','assertSequenceEqual','assertListEqual','assertDictEqual','assertSetEqual','assertTupleEqual'} and len(n.args)>=2 and ast.dump(n.args[0])==ast.dump(n.args[1]):
                    same_asserts.append(dict(path=rel,line=n.lineno,test=node.name,expression=ast.unparse(n)))
    if candidate:
        rows.append(row)

summary_groups = defaultdict(Counter)
for r in rows:
    counter = summary_groups[r['group']]
    counter['files']+=1
    counter[r['language']+'_files']+=1
    for key in ('rust_test_attributes','python_test_definitions','ignored_attributes','source_text_reference_hint','env_gate_hint','historical_name_hint'):
        counter[key]+=r[key]
summary = dict(head=HEAD, method='Static source inventory, not Cargo/unittest runtime discovery or semantic review. Hints are candidates, not defects.',
               tracked_paths=len(tracked), scoped_source_files=dict(source_files), test_bearing_files=len(rows),
               rust_test_attributes=sum(r['rust_test_attributes'] for r in rows),
               python_test_definitions=sum(r['python_test_definitions'] for r in rows),
               groups={k:dict(v) for k,v in sorted(summary_groups.items())},
               workspace_member_count=len(members), workspace_members=[dict(path=m,
                   package=re.search(r'^name\s*=\s*"([^"]+)"', (ROOT/m/'Cargo.toml').read_text(), re.M)[1],
                   source_files=dict(source_groups[m]), test_inventory=dict(summary_groups[m]),
                   semantic_review_status='UNREVIEWED_STATIC_SCAN') for m in members],
               missing_sources=missing, modified_sources=modified, syntax_errors=syntax_errors, python_identical_argument_assertions=same_asserts,
               python_files_without_unittest_style_definitions=python_no_discovery,
               python_identical_test_bodies=[v for v in python_duplicates.values() if len(v)>1], ignored_rust_tests=ignored)
(OUT/'test_value_inventory_summary.json').write_text(json.dumps(summary,indent=2)+'\n')
with (OUT/'test_value_inventory.tsv').open('w',newline='') as f:
    writer=csv.DictWriter(f,fieldnames=list(rows[0]),delimiter='\t', quoting=csv.QUOTE_ALL, lineterminator='\n')
    writer.writeheader();writer.writerows(rows)
with (OUT/'test_value_workspace_members.tsv').open('w',newline='') as f:
    writer=csv.writer(f,delimiter='\t', quoting=csv.QUOTE_ALL, lineterminator='\n'); writer.writerow(['path','package','rust_source_files','test_bearing_files','literal_test_attributes','semantic_review_status'])
    for r in summary['workspace_members']:
        writer.writerow([r['path'],r['package'],r['source_files'].get('.rs',0),r['test_inventory'].get('files',0),r['test_inventory'].get('rust_test_attributes',0),r['semantic_review_status']])
print(json.dumps({k:summary[k] for k in ('head','method','tracked_paths','scoped_source_files','test_bearing_files','rust_test_attributes','python_test_definitions','workspace_member_count','missing_sources','modified_sources','syntax_errors')},indent=2))
print('duplicate_python_body_groups',len(summary['python_identical_test_bodies']))
print('ignored_rust_test_attributes',len(ignored))
