"""Immutable cohort and private DATA packaging helpers; no runtime admission."""
import gzip
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
PINS = {'canary': '04b83b512114bfd888000d6e1433ed8ecaec7c5b',
        'crystal': '00ce02a57ca5a12e48f32a3476e37471167e4c3f'}
INPUTS = {
    'imports/spells/r51/import-manifest.json': 'df51aa01e0796ac7deea08dd57a621a0653e23d614028da8a9ed00e410599bfe',
    'imports/spells/r51/source-programs/source-monster-slot-semantics.json.gz': 'ff5256a1d359cfb7744be4fdadb8ece0edc2c906fd8d35a57d21f39cb5df50e8',
    'imports/spells/r54/import-manifest.json': '9bec95b4e71d64b740f69f37f7865083fc0dccb79a921d0712bd7ad926050fc1',
    'imports/spells/r54/target-projections/projected-monster-slot-candidates.json.gz': '08e7f3852bf54e1b9b0084a907d0bd949f6b7a92c785eb5902fd509cdd422187'}
R65_STEMS = frozenset('lisa_skill_reducer lisa_heal spider_queen_wrap walker_skill_reducer '
    'minotaur_cult_prophet_mass_healing rotthing_wave plagirath_bog icicle_heal '
    'glooth_fairy_healing omrafir_healing_2 the_welter_heal tyrn_heal ignite '
    'candy_horror_wave rootkraken_deathholy rootkraken_rootearth foam_splash '
    'ratmiral_fire_wave ratmiral_ball smelly_cheese_berserk '
    'angry_orc_ancestor_spirit_rooted angry_orc_ancestor_spirit_fear'.split())
FAMILY = 'private_monster_slot_v2'
PRODUCER_FAMILIES = {63: 'private_source_complete_monster_v2',
                    64: 'private_source_complete_monster_slot_v2', 65: FAMILY, 66: FAMILY}
FLAGS = {'target_schema_family': FAMILY, 'authoring_contract_extension_pending': True,
         'source_consumer_implemented': False, 'input_provider_equivalence': False,
         'runtime_activation': False, 'native_execution_qualified': False,
         'native_identity_allocation': False, 'canonical_selection_changed': False}
OPTIONAL_FALSE_FLAGS = ('native_admission', 'native_provider_qualified', 'source_helpers_execution_qualified')


def sha(body):
    return hashlib.sha256(body).hexdigest()


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'), ensure_ascii=False).encode()


def encoded(value):
    return (json.dumps(value, sort_keys=True, indent=2, ensure_ascii=False) + '\n').encode()


def key(identity):
    return json.dumps(identity, sort_keys=True)


def load_inputs(root=ROOT):
    for name, expected in INPUTS.items():
        if sha((root / name).read_bytes()) != expected:
            raise ValueError('immutable monster input changed: ' + name)
    paths = list(INPUTS)
    source = json.loads(gzip.decompress((root / paths[1]).read_bytes()))
    previous = json.loads(gzip.decompress((root / paths[3]).read_bytes()))
    originals = {key(row['slot_identity']): row for row in source['slots']}
    earlier = {key(row['slot_identity']): row for row in previous['slots']}
    if len(originals) != 175 or set(originals) != set(earlier):
        raise ValueError('monster source population differs')
    cohorts = {revision: {} for revision in range(63, 67)}
    for identity, row in originals.items():
        if earlier[identity]['full_slot_projection_complete']:
            continue
        if row['source_program_index'] is None:
            revision = 63
        elif row['custom_category'] is not None:
            revision = 64
        else:
            syntax = source['source_programs'][row['source_program_index']]['source_syntax']
            revision = 65 if Path(syntax['path']).stem in R65_STEMS else 66
        cohorts[revision][identity] = row
    if [len(cohorts[r]) for r in range(63, 67)] != [10, 21, 51, 83]:
        raise ValueError('source cohorts changed')
    return source, previous, cohorts


def verify_rows(revision, rows, root=ROOT):
    _, _, cohorts = load_inputs(root)
    expected = cohorts[revision]
    actual = {key(row['slot_identity']): row for row in rows}
    if len(actual) != len(rows) or set(actual) != set(expected):
        raise ValueError('producer cohort differs')
    for identity, row in actual.items():
        for field in ('slot_identity', 'source', 'monster', 'original_slot_sha256', 'source_parameters'):
            if canonical(row[field]) != canonical(expected[identity][field]):
                raise ValueError('producer source identity changed: ' + field)
        if row.get('required_operations_unrepresented', []) != []:
            raise ValueError('producer declares missing mechanics')
        if row.get('source_alias_to_existing_native_profile', False) is not False:
            raise ValueError('producer aliases an existing profile')
        if row.get('full_slot_projection_complete', True) is not True or row.get('status', 'SOURCE_SCHEMA_VALID') not in ('SOURCE_SCHEMA_VALID', 'CANDIDATE_SCHEMA_VALID'):
            raise ValueError('producer still declares a partial or blocked model')
        if not row.get('controller') or not row.get('source_proofs') or not row.get('target_schedule'):
            raise ValueError('producer missing target model or source proofs')
        for field, expected_value in FLAGS.items():
            if field == 'target_schema_family' and row.get(field) == PRODUCER_FAMILIES[revision]:
                continue
            if field in row and row[field] != expected_value:
                raise ValueError('producer claims runtime or contract admission')
        if any(row.get(field, False) is not False for field in OPTIONAL_FALSE_FLAGS):
            raise ValueError('producer claims native helper admission')
    return actual
