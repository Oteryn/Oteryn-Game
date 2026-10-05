"""Reproduce concrete donor condition, component association and NPC context refinements."""
import gzip
import hashlib
import json
from pathlib import Path
import subprocess
import sys

import jsonschema

OUTPUTS = ('conditions.json', 'conditions-qualification.json', 'guard-specs.json', 'mission-progress.json', 'mission-progress-config.json', 'joins.json',
           'dialogue.json.gz', 'summary.json')


def read(path):
    raw = path.read_bytes()
    return json.loads(gzip.decompress(raw) if path.suffix == '.gz' else raw)


def assemble(root, manifest, ast_root, output):
    here = Path(__file__).parent
    modules = here / 'donor_sources/refinements'
    output.mkdir(parents=True, exist_ok=True)
    commands = [
        [modules / 'conditions/builder.py', '--authoring', here, '--corpus-manifest', manifest,
         '--ast-root', ast_root, '--product-root', root, '--out', output / 'conditions.json'],
        [modules / 'conditions/qualify.py', '--packet', output / 'conditions.json',
         '--authoring', here, '--corpus-manifest', manifest, '--ast-root', ast_root,
         '--out', output / 'conditions-qualification.json'],
        [modules / 'composition/builder.py', '--conditions', output / 'conditions.json',
         '--corpus-manifest', manifest, '--ast-root', ast_root,
         '--out', output / 'guard-specs.json'],
        [modules / 'progress/builder.py', '--repo-root', root,
         '--corpus-manifest', manifest, '--ast-root', ast_root,
         '--out', output / 'mission-progress.json'],
        [modules / 'progress_config/builder.py', '--repo-root', root,
         '--corpus-manifest', manifest, '--ast-root', ast_root,
         '--out', output / 'mission-progress-config.json'],
        [modules / 'joins_next2/builder.py', '--repo-root', root, '--assignment',
         here / 'samples/donor-source/components248/assignments/all.json',
         '--corpus-manifest', manifest, '--out', output / 'joins.json'],
        [modules / 'dialogue/quest_dialogue_links_all.py', '--repo-root', root,
         '--corpus-manifest', manifest, '--ast-root', ast_root, '--all-source-quests',
         '--out', output / 'dialogue.json.gz'],
    ]
    for command in commands:
        subprocess.run([sys.executable, *map(str, command)], check=True)
    return summarize(output)


def summarize(output):
    modules = Path(__file__).parent / 'donor_sources/refinements'
    packets = {}
    for lane, filename, schema, module in [('conditions', 'conditions.json', 'schema.json', 'conditions'),
                                   ('guard_specs', 'guard-specs.json', 'schema.json', 'composition'),
                                   ('mission_progress', 'mission-progress.json', 'schema.json', 'progress'),
                                   ('mission_progress_config', 'mission-progress-config.json', 'schema.json', 'progress_config'),
                                   ('joins', 'joins.json', 'schema.json', 'joins_next2'),
                                   ('dialogue', 'dialogue.json.gz', 'quest_dialogue_links_all.schema.json', 'dialogue')]:
        packet = read(output / filename)
        jsonschema.Draft202012Validator(read(modules / module / schema)).validate(packet)
        if packet['native_admission'] is not False:
            raise ValueError('Source refinement cannot admit runtime')
        packets[lane] = packet
    summary = {'schema': 'OTERYN_QUEST_DONOR_REFINEMENT_COVERAGE/v1',
               'conditions': packets['conditions']['counts'],
               'condition_quests': packets['conditions']['quest_count'],
               'guard_specs': packets['guard_specs']['summary'],
               'mission_progress': packets['mission_progress']['summary'],
               'mission_progress_config': packets['mission_progress_config']['summary'],
               'joins': packets['joins']['summary'], 'dialogue': packets['dialogue']['summary'],
               'canonical_opaque_replacements': 0, 'whole_quests_completed': 0,
               'native_admission': False, 'external_wiki_read': False,
               'outputs': [{'path': name, 'sha256': hashlib.sha256((output / name).read_bytes()).hexdigest()}
                           for name in OUTPUTS[:-1]]}
    (output / 'summary.json').write_text(json.dumps(summary, sort_keys=True, indent=2) + '\n')
    return summary
