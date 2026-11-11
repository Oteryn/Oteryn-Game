"""Refresh only canonical definition digests after unchanged named Item bindings.

The original evidence bytes and pin are Git-bound. The existing source builder
must reproduce the current evidence. Identities, names, client/wiki evidence,
counts and rules remain exact; no general evidence-pin bypass is provided.
"""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import re
import subprocess

import binding_evidence
import eligibility_evidence

ROOT = Path(__file__).resolve().parents[3]
AUTHOR = 'tools/content-schema/imbuement-authoring/imbuement_authoring.py'
BINDINGS = 'tools/content-schema/imbuement-authoring/samples/imbuement-bindings.json'
ELIGIBILITY = 'tools/content-schema/imbuement-authoring/samples/imbuement-eligibility.json'


def verify_reference_only(previous, current, path=()):
    if isinstance(previous, dict) and isinstance(current, dict):
        if previous.keys() != current.keys():
            raise ValueError('BINDING_EVIDENCE_STRUCTURE')
        for key in previous:
            verify_reference_only(previous[key], current[key], path + (key,))
    elif isinstance(previous, list) and isinstance(current, list):
        if len(previous) != len(current):
            raise ValueError('BINDING_EVIDENCE_STRUCTURE')
        for index, (before, after) in enumerate(zip(previous, current)):
            verify_reference_only(before, after, path + (index,))
    elif path[-2:] in (('canonical', 'definition_sha256'), ('identity_evidence', 'canonical_definition_sha256')):
        if (not isinstance(current, str) or len(current) != 64
                or any(c not in '0123456789abcdef' for c in current)):
            raise ValueError('BINDING_EVIDENCE_DEFINITION_DIGEST')
    elif previous != current:
        raise ValueError('BINDING_EVIDENCE_NONREFERENCE_CHANGE: ' + str(path))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', default='HEAD')
    args = parser.parse_args()
    base = subprocess.check_output(['git', '-C', str(ROOT), 'rev-parse', '--verify',
                                    '--end-of-options', args.base + '^{commit}'], text=True).strip()
    def blob(path):
        return subprocess.check_output(['git', '-C', str(ROOT), 'show', base + ':' + path])
    pins = next(ast.literal_eval(node.value)
                for node in ast.parse(blob(AUTHOR).decode()).body if isinstance(node, ast.Assign)
                and any(isinstance(t, ast.Name) and t.id == 'EVIDENCE_PINS' for t in node.targets))
    current = (ROOT / AUTHOR).read_text()
    planned = current
    outputs = {}
    for path in (BINDINGS, ELIGIBILITY):
        old = blob(path)
        filename = Path(path).name
        pin = pins[filename]
        if hashlib.sha256(old).hexdigest() != pin:
            raise ValueError('BINDING_EVIDENCE_BASE_PIN')
        existing = (ROOT / path).read_bytes()
        existing_document = json.loads(existing)
        verify_reference_only(json.loads(old), existing_document)
        fresh = (binding_evidence.build() if path == BINDINGS else
                 eligibility_evidence.build(existing_document))
        verify_reference_only(json.loads(old), fresh)
        raw = (json.dumps(fresh, ensure_ascii=False, sort_keys=path == BINDINGS, indent=2) + '\n').encode()
        digest = hashlib.sha256(raw).hexdigest()
        pattern = r'("' + re.escape(filename) + r'": ")([a-f0-9]{64})(")'
        matches = list(re.finditer(pattern, planned))
        if len(matches) != 1 or matches[0].group(2) not in (pin, digest, hashlib.sha256(existing).hexdigest()):
            raise ValueError('BINDING_EVIDENCE_CURRENT_PIN')
        planned = re.sub(pattern, lambda m: m[1] + digest + m[3], planned)
        outputs[path] = raw
    # Every source, reference-only and original-pin proof precedes any write.
    for path, raw in outputs.items():
        (ROOT / path).write_bytes(raw)
    (ROOT / AUTHOR).write_text(planned)
    print('PASS unchanged named bindings; canonical definition digest pin refreshed from Git/source proof')


if __name__ == '__main__':
    main()
