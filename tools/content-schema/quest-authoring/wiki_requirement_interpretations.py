"""Bounded SOURCE requirement syntax; historical specifications stay immutable."""
import argparse
import copy
import hashlib
import json
import re
from pathlib import Path


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()


def partial_requirement(raw, witness, section):
    """Parse literal categories/scopes while retaining missing condition fields."""
    text = re.sub(r"'{2,}", '', raw).strip()
    text = re.sub(r'^\*\s*', '', text)
    generic = {'Dinheiro para viagem': ('travel funds', 'currency_and_amount'),
        'Dinheiro para viagens': ('travel funds', 'currency_and_amount'),
        'Poções de cura.': ('healing supplies', 'item_identities_and_quantities'),
        'Hunting supplies': ('hunting supplies', 'item_identities_and_quantities'),
        'Team work': ('team work', 'team_members_and_size'),
        'Other participants in the event': ('event participants', 'participant_identities_and_count'),
        'Suprimentos.': ('supplies', 'item_identities_and_quantities'),
        'Suprimentos;': ('supplies', 'item_identities_and_quantities'),
        'Arma melee': ('melee weapon', 'weapon_identity')}
    facts, fields, recommendation = None, [], False
    if text in generic:
        label, missing = generic[text]
        facts = {'requirement_category': label, 'required_quantity': None}; fields = [missing]
    if re.fullmatch(r'\[\[Arquivo:[^\]\n]+\]\] uma arma física', text):
        facts = {'requirement_category': 'physical weapon', 'required_quantity': None}; fields = ['weapon_identity']
    outfit = re.fullmatch(r'Ter o (outfit base(?: e primeiro addon)?)\.', text)
    if outfit:
        facts = {'required_component_literal': outfit[1]}; fields = ['outfit_identity_and_state_binding']
    if text == 'All previous bosses defeated':
        facts = {'required_defeated_group_literal': 'previous bosses', 'group_quantifier': 'all'}; fields = ['boss_identities_and_order']
    places = re.fullmatch(r'Alguns locais só são acessíveis por personagens level ([0-9]+) ou superior\.', text)
    if places:
        facts = {'minimum_level_literal': int(places[1]), 'scope_literal': 'some_locations'}; fields = ['location_identities']
    if text == 'Por ser necessário executar uma grande exploração por todo o mapa tibiano, várias quests são necessárias para se ter acesso a determinados locais.':
        facts = {'requirement_category': 'quests for location access', 'required_quantity': None}; fields = ['quest_and_location_identities']
    caution = re.fullmatch(r"\*\s*'''(Carved Shrine [0-9]+):''' Shrine em área perigosa, leve proteção!", raw)
    if caution:
        facts = {'recommendation_category': 'protection', 'location_label': caution[1]}; fields = ['protection_items_or_effects']; recommendation = True
    if facts is None: return None
    return {'prerequisite_expressions': [], 'source_fact_entries': [{'kind': 'curated_source_fact',
        'source_order': witness['line'], 'execution_semantics': 'UNKNOWN', 'binding_status': 'UNKNOWN',
        'classification': 'DERIVED', 'scope': section, 'facts': facts, 'evidence': [copy.deepcopy(witness)]}],
        'unresolved_semantics': [{'field': f, 'classification': 'UNKNOWN'} for f in fields],
        'source_scope_kind': 'recommendation_only' if recommendation else 'source_requirement_reference'}


def parse_requirement(raw, witness, section):
    """Return source references/literal facts only; no canonical or execution choice."""
    text = re.sub(r"'{2,}", '', raw).strip()
    text = re.sub(r'^\*\s*', '', text)
    expr = None
    facts = None
    level = re.fullmatch(r'(?:Level|Nível|Nivel) ([0-9]+) (?:or more|or higher|ou superior)\.?', text, re.I)
    if level:
        expr = {'line': witness['line'], 'line_sha256': witness['line_sha256'], 'section': section,
            'scope': 'source_requirement_list', 'scope_anchor': None, 'operator': 'references',
            'execution_semantics': 'UNKNOWN', 'declaration_strength': 'reference_only',
            'nodes': [{'kind': 'level_at_least', 'value': int(level[1]),
                       'binding_status': 'SOURCE_DECLARED', 'applies_to': 'source_requirement_list'}]}
    templates = re.findall(r'\{\{Achievement\|([^{}|]+)\}\}', text)
    remainder = re.sub(r'\{\{Achievement\|[^{}|]+\}\}', '', text)
    if templates and all(name.strip() for name in templates) and re.fullmatch(r'Ter os achievements[ ,e.]*', remainder):
        expr = {'line': witness['line'], 'line_sha256': witness['line_sha256'], 'section': section,
            'scope': 'source_requirement_list', 'scope_anchor': None, 'operator': 'references',
            'execution_semantics': 'UNKNOWN', 'declaration_strength': 'reference_only',
            'nodes': [{'kind': 'entity_reference', 'entity': name.strip(), 'quantity': None,
                       'binding_status': 'UNKNOWN', 'source_namespace': 'mediawiki/page_title'} for name in templates]}
    money = re.fullmatch(r'([0-9]{1,3}(?:,[0-9]{3})+|[0-9]+) gold coins for donations\.', text)
    travel = re.fullmatch(r'Dinheiro para as viagens\. \(([0-9]+) gps\)', text)
    if money or travel:
        facts = {'required_amount_literal': int((money or travel)[1].replace(',', '')),
                 'unit_literal': 'gold coins' if money else 'gps', 'scope_literal': 'donations' if money else 'viagens'}
    cumulative = re.fullmatch(r'Os pontos de War Exp são acumulativos\. Isso quer dizer que,? para conseguir o (primeiro|segundo) addon,? (?:você precisa de|são necessários) ([0-9]+) pontos \(não ([0-9]+(?: \+ [0-9]+)+)\)\.', text)
    if cumulative:
        facts = {'required_amount_literal': int(cumulative[2]), 'unit_literal': 'War Exp pontos',
                 'scope_literal': cumulative[1] + ' addon', 'cumulative_declared': True,
                 'excluded_additive_terms': [int(x) for x in cumulative[3].split(' + ')]}
    title = re.fullmatch(r"\*\s*Ter o título '''([^'{}\[\]\n]+)'''\.", raw)
    if title:
        facts = {'required_title_literal': title[1]}
    scope = re.fullmatch(r"\*\s*(?:For the ''optional mission''|Para missão ''opcional'') '''((?:(?!''')[^{}\[\]\n])+)''':?\s*", raw)
    if scope:
        facts = {'optional_mission_heading': scope[1].strip().strip('"')}
    if not expr and not facts:
        return partial_requirement(raw, witness, section)
    fact = [] if not facts else [{'kind': 'curated_source_fact', 'source_order': witness['line'],
        'execution_semantics': 'UNKNOWN', 'binding_status': 'UNKNOWN', 'classification': 'DERIVED',
        'scope': section, 'facts': facts, 'evidence': [copy.deepcopy(witness)]}]
    return {'prerequisite_expressions': [] if not expr else [expr], 'source_fact_entries': fact}


def schema():
    def obj(props): return {'type': 'object', 'properties': props, 'required': list(props), 'additionalProperties': False}
    string = {'type': 'string', 'minLength': 1}
    hashed = {'type': 'string', 'pattern': '^[0-9a-f]{64}$'}
    unknown = {'const': 'UNKNOWN'}
    integer = {'type': 'integer', 'minimum': 0}
    witness = obj({'line': {'type': 'integer', 'minimum': 1}, 'line_sha256': hashed,
                   'byte_offset': integer, 'byte_length': integer, 'span_sha256': hashed})
    facts = {'oneOf': [obj({'required_amount_literal': integer, 'unit_literal': string, 'scope_literal': string}),
        obj({'required_amount_literal': integer, 'unit_literal': {'const': 'War Exp pontos'}, 'scope_literal': string,
             'cumulative_declared': {'const': True}, 'excluded_additive_terms': {'type': 'array', 'minItems': 2, 'items': integer}}),
        obj({'required_title_literal': string}), obj({'optional_mission_heading': string}),
        obj({'requirement_category': string, 'required_quantity': {'const': None}}),
        obj({'required_component_literal': string}),
        obj({'required_defeated_group_literal': string, 'group_quantifier': {'const': 'all'}}),
        obj({'minimum_level_literal': integer, 'scope_literal': {'const': 'some_locations'}}),
        obj({'recommendation_category': {'const': 'protection'}, 'location_label': string})]}
    fact = obj({'kind': {'const': 'curated_source_fact'}, 'source_order': integer, 'execution_semantics': unknown,
        'binding_status': unknown, 'classification': {'const': 'DERIVED'}, 'scope': {'type': 'string'},
        'facts': facts, 'evidence': {'type': 'array', 'minItems': 1, 'maxItems': 1, 'items': witness}})
    return fact


def build(specifications, authored, receipt, authored_schema):
    from jsonschema import Draft202012Validator
    if receipt['qualification'] != 'RECORDED_LOCAL_PINNED_BODY_AND_LINE_WITNESSES' or receipt['historical_entries_rewritten'] is not False or digest(authored_schema) != receipt['schema_sha256']:
        raise ValueError('Interpretation qualification, schema or historical preservation differs')
    if digest(authored) != receipt['authored_interpretations_sha256'] or digest(specifications) != receipt['specifications_semantic_sha256']:
        raise ValueError('Authored interpretations or frozen specification anchor differ')
    Draft202012Validator(authored_schema).validate(authored)
    baseline = {}
    for entry in specifications['entries']:
        for hold in entry['unresolved']:
            if hold['field_group'] == 'Source requirement not parsed':
                baseline[(entry['wiki_title'], digest(hold))] = hold
    seen, rows = set(), []
    for row in authored:
        key = row['wiki_title'], row['historical_hold_sha256']
        if key in seen or key not in baseline or row['historical_evidence'] != baseline[key]['evidence']:
            raise ValueError('Duplicate, missing or mismatched historical requirement identity')
        if row['execution_semantics'] != 'UNKNOWN' or row['canonical_binding_status'] != 'UNKNOWN':
            raise ValueError('SOURCE interpretation cannot promote execution/binding')
        for fact in row['source_fact_entries']: Draft202012Validator(schema()).validate(fact)
        for expr in row['prerequisite_expressions']:
            if expr['execution_semantics'] != 'UNKNOWN' or expr['declaration_strength'] != 'reference_only':
                raise ValueError('SOURCE expression authority differs')
            if expr['line'] != baseline[key]['evidence']['line'] or expr['line_sha256'] != baseline[key]['evidence']['line_sha256']:
                raise ValueError('Source expression witness differs')
        partial = any(set(f['facts']) & {'requirement_category', 'required_component_literal', 'required_defeated_group_literal', 'minimum_level_literal', 'recommendation_category'} for f in row['source_fact_entries'])
        if partial and (not row.get('unresolved_semantics') or row.get('prerequisite_expressions')):
            raise ValueError('Partial SOURCE condition cannot lose unknown fields or become a gate')
        if partial and row.get('source_scope_kind') != ('recommendation_only' if any('recommendation_category' in f['facts'] for f in row['source_fact_entries']) else 'source_requirement_reference'):
            raise ValueError('Recommendation and required source scope differ')
        if not row['prerequisite_expressions'] and not row['source_fact_entries']:
            raise ValueError('Empty interpretation cannot resolve requirement syntax')
        for fact in row['source_fact_entries']:
            for proof in fact['evidence']:
                if proof['line'] != row['historical_evidence']['line'] or proof['line_sha256'] != row['historical_evidence']['line_sha256']:
                    raise ValueError('Literal fact witness differs')
        rows.append(copy.deepcopy(row)); seen.add(key)
    preserved = receipt.get('preserved_interpretation_digests', [])
    if preserved and [digest(r) for r in rows[:len(preserved)]] != preserved:
        raise ValueError('Prior interpretations cannot change or reorder')
    if sorted(digest(r) for r in rows) != receipt['interpretation_record_digests']:
        raise ValueError('Interpretation receipt membership differs')
    return {'schema': 'OTERYN_WIKI_REQUIREMENT_INTERPRETATIONS/v1', 'scope': 'SOURCE syntax only; historical holds preserved',
        'historical_specifications_sha256': receipt['specifications_file_sha256'], 'raw_body_rechecked': False,
        'proof_mode': 'AUTHORED_FACTS_AND_RECORDED_OFFLINE_BODY_WITNESSES', 'runtime_readiness': 'UNKNOWN',
        'definition_complete': False, 'historical_unparsed_holds': len(baseline), 'interpreted_holds': len(rows),
        'current_unparsed_holds': len(baseline)-len(rows),
        'partial_interpretations_with_unknown_fields': sum(bool(r.get('unresolved_semantics')) for r in rows),
        'structured_unknown_fields': sum(len(r.get('unresolved_semantics', [])) for r in rows),
        'full_syntax_interpretations': sum(not bool(r.get('unresolved_semantics')) for r in rows),
        'partial_syntax_interpretations': sum(bool(r.get('unresolved_semantics')) for r in rows),
        'unparsed': len(baseline)-len(rows),
        'effective_unparsed_or_partial': len(baseline)-len(rows)+sum(bool(r.get('unresolved_semantics')) for r in rows), 'interpretations': rows,
        'remaining_holds': [{'wiki_title': title, 'hold': hold} for (title, marker), hold in baseline.items() if (title, marker) not in seen]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--samples', type=Path, default=Path(__file__).parent / 'samples/wiki-requirements')
    parser.add_argument('--specifications', type=Path, default=Path(__file__).parent / 'samples/wiki-source-all373/source-specs-373.json')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args(); read = lambda p: json.loads(p.read_text())
    receipt = read(args.samples/'interpretation-receipt.json')
    if hashlib.sha256(args.specifications.read_bytes()).hexdigest() != receipt['specifications_file_sha256']:
        raise ValueError('Frozen SOURCE specification file digest differs')
    content = json.dumps(build(read(args.specifications), read(args.samples/'interpretations-input.json'), receipt, read(args.samples/'interpretation-input.schema.json')), ensure_ascii=False, indent=2)+'\n'
    output = args.samples/'current-interpretations.json'
    if args.check:
        if not output.is_file() or output.read_text() != content: raise ValueError('SOURCE interpretation output stale')
    else: output.write_text(content)
    print(json.dumps({k:json.loads(content)[k] for k in ('historical_unparsed_holds', 'full_syntax_interpretations', 'partial_syntax_interpretations', 'unparsed', 'effective_unparsed_or_partial')}))


if __name__ == '__main__': main()
