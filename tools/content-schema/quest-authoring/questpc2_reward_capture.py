"""Distinguish applied door evidence from reward-selector captures."""
import hashlib
import json

GATES = 'tools/content-schema/quest-authoring/samples/doors/gates.json'
GATE_MANIFEST = 'tools/content-schema/quest-authoring/samples/doors/manifest.json'
SHA = 'dfcd8f8424d01c393fc6e0a25dbf26aeab902ec5cc691bd21ddbe336ddad7e87'


def reward_rows(path, raw):
    document = json.loads(raw)
    if 'bindings' in document:
        return document['bindings']
    if (path == GATES and hashlib.sha256(raw).hexdigest() == SHA
            and set(document) == {'gates'}):
        return []  # Existing door access evidence grants no reward selector.
    if (path == GATE_MANIFEST
            and set(document) == {'classification', 'join', 'sources', 'counts', 'entries', 'source_checks'}):
        return []  # The applied Paradox door cites the pinned gate manifest.
    raise ValueError('unrecognized applied reward capture format')
