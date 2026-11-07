"""SPDX-License-Identifier: MPL-2.0; generate closed shapes for the finite capture."""
import json
from pathlib import Path
from quest_enrichment_authoring import FLAGS, INPUT_SHA, PROFILE, read

ROOT = Path(__file__).resolve().parent

def shape(values, key=''):
    if key in FLAGS:
        return {'const': FLAGS[key]}
    groups = {}
    for value in values:
        kind = ('null' if value is None else 'boolean' if isinstance(value, bool) else
                'integer' if isinstance(value, int) else 'number' if isinstance(value, float) else
                'object' if isinstance(value, dict) else 'array' if isinstance(value, list) else 'string')
        groups.setdefault(kind, []).append(value)
    alternatives = []
    for kind, values in sorted(groups.items()):
        spec = {'type': kind}
        if kind == 'object':
            keys = sorted({k for v in values for k in v})
            spec.update(additionalProperties=False, required=sorted(set.intersection(*(set(v) for v in values))),
                        properties={k: shape([v[k] for v in values if k in v], k) for k in keys})
        elif kind == 'array':
            children = [c for v in values for c in v]
            spec['items'] = shape(children) if children else False
        elif kind == 'string' and (key.endswith('sha256') or key == 'span_sha256'):
            spec['pattern'] = '^[a-f0-9]{64}$'
        elif kind == 'string' and key in ('baseline_head',):
            spec['pattern'] = '^[a-f0-9]{40}$'
        elif kind == 'string' and key in ('profile', 'novelty', 'provider', 'kind', 'classification', 'disposition'):
            spec['enum'] = sorted(set(values))
        elif kind == 'integer' and key in ('line_start', 'line_end'):
            spec['minimum'] = 1
        alternatives.append(spec)
    return alternatives[0] if len(alternatives) == 1 else {'anyOf': alternatives}


def build_schema():
    capture = read(ROOT / 'samples/enrichment242/capture.json')
    packet = {**capture, 'schema': 'OTERYN_QUEST_SOURCE_ENRICHMENT/v1'}
    receipt = read(ROOT / 'samples/enrichment242/receipt.json')
    schema = shape([packet]); schema['properties']['schema'] = {'const': packet['schema']}
    schema['properties']['records']['minItems'] = schema['properties']['records']['maxItems'] = 242
    receipt_schema = shape([receipt]); receipt_schema['properties']['schema'] = {'const': receipt['schema']}
    receipt_schema['properties']['original_input_sha256'] = {'const': INPUT_SHA}
    metadata = {'profile': PROFILE, 'classification': 'SCOPED_EVIDENCE_ONLY_NOT_SOURCE_CLOSURE',
                'path': 'tools/content-schema/quest-authoring/samples/enrichment242/enrichment.json',
                'packet_sha256': '0'*64, 'receipt_sha256': '0'*64, 'record_sha256': '0'*64,
                'facts_count': 1, 'candidate_count': 0, 'confirmation_count': 0, 'reduced_text_count': 0, **FLAGS}
    metadata_schema = shape([metadata])
    for name in ('profile', 'classification', 'path'):
        metadata_schema['properties'][name] = {'const': metadata[name]}
    schema.update({'$schema': 'https://json-schema.org/draft/2020-12/schema',
                   '$id': 'oteryn:schema/quest-enrichment/v1',
                   'description': 'Finite scoped historical evidence; no Source hold closure or Native admission.',
                   '$defs': {'receipt': receipt_schema, 'metadata': metadata_schema}})
    return schema

if __name__ == '__main__':
    (ROOT / 'quest_enrichment.schema.json').write_text(json.dumps(build_schema(), indent=2, sort_keys=True)+'\n')
