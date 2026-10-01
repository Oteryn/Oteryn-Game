"""Qualify successor declarations; native staged migration stays with GEM-R."""
import hashlib
import json
import re
from wheel_authoring import ROOT, read


def gem_contract(candidate):
    def scrub(value):
        if isinstance(value, dict):
            return {k: scrub(v) for k, v in value.items() if k not in
                    ('icon', 'name', 'summary_name', 'reference_text', 'reference_summary')}
        if isinstance(value, list):
            return [scrub(v) for v in value]
        return value
    return scrub({'gems': {k: v for k, v in candidate['gems'].items()
                           if k not in ('reference_corrections', 'loot_reference', 'parameter_state')},
                  'vocations': {v: {k: data[k] for k in ('gem_family', 'basic_mods_position_1',
                    'basic_mods_position_2', 'supreme_mods', 'resonance_slots')}
                    for v, data in candidate['vocations'].items()}})


def contract_digest(contract):
    return hashlib.sha256(json.dumps(contract, sort_keys=True, allow_nan=False).encode()).hexdigest()


def validate_gem_revision(candidate, previous):
    def require(condition, code):
        if not condition:
            raise ValueError(code)
    declaration = candidate['release'].get('gem_revision')
    source, destination = gem_contract(previous), gem_contract(candidate)
    if declaration is None:
        require(source == destination, 'GEM_REVISION_DECLARATION_REQUIRED')
        return
    reference = declaration['reference']
    require(re.fullmatch(r'samples/gem-revisions/[a-z0-9][a-z0-9_-]*\.json', reference), 'GEM_REVISION_PATH')
    path = (ROOT / reference).resolve()
    require(path.is_relative_to(ROOT.resolve()) and path.is_file(), 'GEM_REVISION_PATH')
    require(hashlib.sha256(path.read_bytes()).hexdigest() == declaration['sha256'], 'GEM_REVISION_DIGEST')
    record = read(path)
    require(record.get('schema') == 'OTERYN_WHEEL_GEM_REVISION_REFERENCE/v1', 'GEM_REVISION_SCHEMA')
    require(record.get('source_revision') == previous['revision'] and
            record.get('destination_revision') == candidate['revision'], 'GEM_REVISION_CHAIN')
    require(record.get('kind') == declaration['kind'], 'GEM_REVISION_KIND')
    require(record.get('source_gem_sha256') == contract_digest(source) and
            record.get('destination_gem_sha256') == contract_digest(destination), 'GEM_REVISION_CONTRACT')
    require(record.get('runtime_admitted') is False and
            record.get('runtime_validation') == 'PENDING_GEM_R_ADMISSION', 'GEM_REVISION_ADMISSION')
    if declaration['kind'] == 'declared_compatible':
        # Fees/yields can differ while both revisions decode stored gem/grade rows identically.
        def row_contract(contract):
            gems = contract['gems']
            return {'vocations': contract['vocations'], 'mods': [gems['basic_mods'], gems['supreme_mods']],
                    'qualities': gems['qualities'], 'grade_chain': {
                        k: gems['atelier'][k] for k in ('basic_pair_compatibility',
                            'effective_grade_order', 'effective_grade_rule', 'grade_scope')}}
        require(row_contract(source) == row_contract(destination), 'GEM_COMPATIBLE_ROW_CHANGED')
    else:
        require(declaration['kind'] == 'staged_migration', 'GEM_REVISION_KIND')
        require(isinstance(record.get('native_migration_reference'), str) and
                bool(record['native_migration_reference'].strip()), 'GEM_NATIVE_MIGRATION_REFERENCE')

