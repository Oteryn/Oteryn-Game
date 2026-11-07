"""Portable strict structure for derived facts; baseline payload remains digest-bound."""
import json
from pathlib import Path


def obj(properties, required=()):
    return {'type': 'object', 'properties': properties, 'required': list(required), 'additionalProperties': False}


def array(items):
    return {'type': 'array', 'items': items}


def ref(name):
    return {'$ref': '#/$defs/' + name}


def build():
    string = {'type': 'string'}
    integer = {'type': 'integer', 'minimum': 0}
    unknown = {'const': 'UNKNOWN'}
    false = {'const': False}
    digest = {'type': 'string', 'pattern': '^[0-9a-f]{64}$'}
    witness = {'line': {'type': 'integer', 'minimum': 1}, 'line_sha256': digest,
               'byte_offset': integer, 'byte_length': integer, 'span_sha256': digest}
    identity = {'provider': {'enum': ['tibia_fandom', 'tibiawiki_br']},
        'relation': {'enum': ['base', 'spoiler', 'crosscheck']},
        'pageid': {'type': 'integer', 'minimum': 1}, 'revid': {'type': 'integer', 'minimum': 1},
        'content_sha256': digest, 'url': {'type': 'string', 'pattern': '^https://(?:tibia\\.fandom\\.com|www\\.tibiawiki\\.com\\.br)/'}}
    defs = {'witness': obj(witness, witness), 'source_ref': obj(identity, identity)}
    entity = {'entity': string, 'quantity': {'type': ['integer', 'null'], 'minimum': 0},
              'binding_status': unknown, 'source_namespace': {'const': 'mediawiki/page_title'}}
    defs['entity'] = obj(entity, entity)
    defs['requirement_node'] = obj({**entity, 'kind': {'const': 'entity_reference'}}, (*entity, 'kind'))
    defs['reward_group'] = obj({'scope_anchor': {'type': ['string', 'null']},
                              'entities': array(ref('entity')), 'evidence': ref('witness')},
                             ('scope_anchor', 'entities', 'evidence'))
    defs['quantitative_reward'] = obj({'kind': {'enum': ['experience', 'gold']}, 'amount': integer,
                                      'scope': string, 'evidence': ref('witness')},
                                     ('kind', 'amount', 'scope', 'evidence'))
    defs['prerequisite'] = obj({**witness, 'section': string, 'scope': string,
        'scope_anchor': {'type': ['string', 'null']}, 'execution_semantics': unknown,
        'operator': {'enum': ['references', 'any_of_references']},
        'nodes': array(ref('requirement_node')), 'declaration_strength': {'const': 'reference_only'}},
        (*witness, 'section', 'scope', 'scope_anchor', 'execution_semantics', 'operator', 'nodes', 'declaration_strength'))
    defs['unparsed'] = obj({**witness, 'reason': string}, (*witness, 'reason'))
    delta = {'reward_entity_references': array(ref('entity')), 'reward_groups': array(ref('reward_group')),
        'reward_quantitative_facts': array(ref('quantitative_reward')), 'prerequisite_expressions': array(ref('prerequisite')),
        'unparsed_requirements': array(ref('unparsed')), 'reward_declared_none': {'type': 'boolean'}}
    defs['delta'] = obj(delta, delta)
    defs['span'] = obj({**witness, 'field': string, 'section': string, 'source_order': integer,
                       'classification': {'const': 'DERIVED'}}, (*witness, 'field', 'section', 'source_order', 'classification'))
    defs['heading'] = obj({**witness, 'heading': string, 'level': {'type': 'integer', 'minimum': 1, 'maximum': 6},
                           'source_order': integer, 'execution_order': unknown},
                         (*witness, 'heading', 'level', 'source_order', 'execution_order'))
    defs['journal'] = obj({**witness, 'value': {'const': 'REFERENCE_ONLY'}, 'section': string,
                          'source_order': integer, 'journal_text_ref': ref('witness')},
                         (*witness, 'value', 'section', 'source_order', 'journal_text_ref'))
    defs['counter'] = obj({'amount': integer, 'unit': string, 'scope': string,
                           'interpretation': {'const': 'LITERAL_ONLY'}}, ('amount', 'unit', 'scope', 'interpretation'))
    defs['table_cell'] = obj({'entity_references': array(ref('entity')), 'numeric_literals': array(string),
                             'evidence': ref('witness'), 'header': string},
                            ('entity_references', 'numeric_literals', 'evidence', 'header'))
    defs['json_value'] = {'anyOf': [{'type': ['string', 'number', 'boolean', 'null']},
        array(ref('json_value')), {'type': 'object', 'additionalProperties': ref('json_value')}]}
    fact_props = {**witness, 'kind': {'enum': ['source_reference', 'reward_reference', 'recommendation',
        'source_requirement_list', 'exchange_reference', 'source_table', 'curated_source_fact']},
        'section': string, 'source_order': integer, 'entity_references': array(ref('entity')),
        'counters': array(ref('counter')), 'execution_semantics': unknown,
        'headers': array(string), 'header_witnesses': array(ref('witness')),
        'rows': array(array(ref('table_cell'))), 'scope': string,
        'facts': {'type': 'object', 'additionalProperties': ref('json_value')},
        'field': string, 'value': ref('json_value'),
        'evidence': {'oneOf': [array(ref('witness')), ref('witness')]},
        'classification': {'const': 'DERIVED'}, 'binding_status': unknown}
    defs['fact'] = obj(fact_props, ('kind', 'source_order', 'execution_semantics'))
    defs['fact']['allOf'] = [
        {'if': {'properties': {'kind': {'const': 'curated_source_fact'}}},
         'then': {'required': ['evidence', 'binding_status'], 'oneOf': [
             {'required': ['scope', 'facts', 'classification'],
              'properties': {'evidence': {**array(ref('witness')), 'minItems': 1}}},
             {'required': ['field', 'value'], 'properties': {'evidence': ref('witness')}}]}},
        {'if': {'properties': {'kind': {'const': 'source_table'}}},
         'then': {'required': ['section', 'headers', 'header_witnesses', 'rows']}},
        {'if': {'properties': {'kind': {'not': {'enum': ['curated_source_fact', 'source_table']}}}},
         'then': {'required': [*witness, 'section', 'entity_references', 'counters']}}]
    field = obj({**witness, 'value': {'type': ['string', 'null']},
                 'meaning': {'enum': ['recommendation', 'source_field_only']}}, (*witness, 'value', 'meaning'))
    fields = {name: field for name in ['lvl', 'lvlrec', 'lvlreq', 'lvlmax', 'premium', 'questlog', 'quest_log',
        'log', 'team', 'levelnote', 'lvlnote', 'time', 'timealloc', 'duration', 'removed', 'requirements']}
    source = {**identity, 'semantic_enrichment_delta': ref('delta'), 'source_spans': array(ref('span')),
        'heading_witnesses': array(ref('heading')), 'journal_field_witnesses': array(ref('journal')),
        'source_fact_entries': array(ref('fact')), 'source_fields': obj(fields), 'source_field_listing_complete': false,
        'access_method': {'const': 'remote_desktop_browser'}, 'processing_method': {'const': 'offline_pinned_capture'},
        'target_cut': {'enum': ['2026-09-27', '2026-10-01']},
        'revision_timestamp': {'type': 'string', 'pattern': '^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$'},
        'raw_capture': obj({'body_path': string, 'bytes': integer, 'sha256': digest,
                            'publication': {'const': 'REFERENCE_ONLY_SCRATCH'}},
                           ('body_path', 'bytes', 'sha256', 'publication')),
        'source_revision_verified': false, 'historical_acquisition_verified': {'const': True}, 'proof_verification': {'const': 'RECORDED_ACQUISITION_ONLY'}, 'classification': {'const': 'DERIVED'},
        'runtime_promotion': false, 'unclosed_table': {'const': True}, 'bounded_scope_corrections': array(string)}
    defs['source'] = obj(source, tuple(k for k in source if k not in ('unclosed_table', 'bounded_scope_corrections')))
    for definition in defs.values():
        if 'line_sha256' in definition.get('properties', {}):
            definition['properties']['end_line'] = {'type': 'integer', 'minimum': 1}
    # Legacy source data is opaque here and checked against the mandatory baseline
    # input by the CLI. Derived facts above remain closed, typed and span-verified.
    entry = {name: {'type': 'object'} for name in ['historical_donor_status', 'source_identity_overlap']}
    entry.update({name: string for name in ['source_field_completeness', 'canonical_binding_readiness']})
    entry.update({name: {'type': 'array'} for name in ['source_specification', 'unresolved',
        'item_name_review_candidates', 'source_fields_with_provenance', 'source_identity_links',
        'source_item_identity_bindings', 'verified_source_conditions']})
    entry.update({'wiki_title': string, 'source_refs': array(ref('source_ref')),
        'migration_status': {'const': 'BLOCKED_UNKNOWN_BINDINGS'}, 'definition_complete': false,
        'runtime_readiness': unknown, 'available_model_projection': string, 'baseline_entry_sha256': digest,
        'source_fact_supplements': array(ref('source')), 'source_capture_complete_for_selected_revisions': {'const': True},
        'source_field_listing_complete': false})
    defs['entry'] = obj(entry, ('wiki_title', 'source_refs', 'source_specification', 'unresolved',
        'migration_status', 'definition_complete', 'runtime_readiness', 'baseline_entry_sha256',
        'source_fact_supplements', 'source_capture_complete_for_selected_revisions', 'source_field_listing_complete'))
    root = {'schema': {'const': 'OTERYN_WIKI_ONLY_SOURCE_SPEC_ENRICHMENT/v2'}, 'titles': integer,
        'entries': array(ref('entry')), 'no_runtime_promotion': {'const': True},
        'supplement_summary': obj({name: integer for name in ['source_revisions', 'source_spans',
            'heading_witnesses', 'journal_field_witnesses', 'source_fact_entries', 'unparsed_requirements',
            'curated_source_fact_groups', 'titles', 'provider:tibia_fandom', 'provider:tibiawiki_br']}),
        'source_field_listing_complete': false, 'selected_source_capture_complete': {'const': True},
        'source_definition_complete': false, 'baseline_inventory_sha256': digest,
        'source_access': obj({key: {'const': value} for key, value in {
            'repositories': 'normal_github_and_local_pinned_read',
            'tibia_fandom': 'remote_desktop_browser_after_normal_http_402',
            'tibiawiki_br': 'remote_desktop_browser_after_normal_http_403', 'tibiopedia': 'UNAVAILABLE'}.items()},
            ('repositories', 'tibia_fandom', 'tibiawiki_br', 'tibiopedia')),
        'supplement_inputs': array(obj({'artifact': string, 'sha256': digest}, ('artifact', 'sha256'))),
        'inventory_scope': obj({'repository': string, 'revision': {'type': 'string', 'pattern': '^[0-9a-f]{40}$'},
            'path': string, 'content_sha256': digest, 'selection': {'const': 'unbound_at_pinned_catalogue_baseline'},
            'later_partial_bindings_do_not_remove_snapshot_specifications': {'const': True}, 'runtime_promotion': false,
            'selected_title_count': integer, 'current_binding_observation': {'oneOf': [{'type': 'null'},
                obj({'catalogue_sha256': digest, 'snapshot_titles_with_current_source_binding': array(string),
                     'snapshot_titles_currently_unbound': integer,
                     'current_source_binding_does_not_assert_native_definition_ready': {'const': True}},
                    ('catalogue_sha256', 'snapshot_titles_with_current_source_binding',
                     'snapshot_titles_currently_unbound', 'current_source_binding_does_not_assert_native_definition_ready'))]}},
            ('repository', 'revision', 'path', 'content_sha256', 'selection',
             'later_partial_bindings_do_not_remove_snapshot_specifications', 'runtime_promotion',
             'selected_title_count', 'current_binding_observation'))}
    root['offline_verification'] = obj({'mode': {'const': 'AUTHORED_FACTS_AND_RECORDED_ACQUISITION_ONLY'}, 'raw_body_rechecked': false, 'line_spans_rechecked': false, 'source_membership_checked': {'const': True}, 'authored_fact_digests_checked': {'const': True}, 'acquisition_receipt_sha256': digest, 'frozen_acquisition_handoff_sha256': digest}, ('mode', 'raw_body_rechecked', 'line_spans_rechecked', 'source_membership_checked', 'authored_fact_digests_checked', 'acquisition_receipt_sha256', 'frozen_acquisition_handoff_sha256'))
    defs['authored_supplement'] = obj({'schema': {'const': 'OTERYN_BOUNDED_WIKI_FACT_SUPPLEMENT/v1'}, 'chunk': {'type': 'integer', 'minimum': 1, 'maximum': 5}, 'entries': array(obj({'wiki_title': string, 'sources': array(ref('source')), 'definition_complete': false, 'runtime_promotion': false}, ('wiki_title', 'sources', 'definition_complete', 'runtime_promotion'))), 'definition_complete': false, 'runtime_promotion': false, 'source_field_listing_complete': false}, ('schema', 'chunk', 'entries', 'definition_complete', 'runtime_promotion', 'source_field_listing_complete'))
    receipt = {'schema': {'const': 'OTERYN_RECORDED_WIKI_ACQUISITION/v1'}, 'qualification': {'const': 'RAW_BODIES_VERIFIED_IN_PRIOR_ACQUISITION_ONLY'}, 'frozen_handoff_sha256': digest, 'frozen_output_sha256': digest, 'raw_capture_manifest_sha256': digest, 'baseline_inventory_sha256': digest, 'selection_receipt_sha256': digest, 'source_records': array(obj({**identity, 'wiki_title': string, 'source_fact_sha256': digest, 'frozen_source_fact_sha256': digest}, (*identity, 'wiki_title', 'source_fact_sha256', 'frozen_source_fact_sha256'))), 'runtime_promotion': false}
    defs['acquisition_receipt'] = obj(receipt, receipt)
    defs['input_manifest'] = obj({'schema': {'const': 'OTERYN_PORTABLE_SOURCE_INPUT_MANIFEST/v1'}, 'inputs': {'type': 'object', 'patternProperties': {'^[a-z0-9-]+\\.json$': digest}, 'additionalProperties': False}}, ('schema', 'inputs'))
    return {'$schema': 'https://json-schema.org/draft/2020-12/schema',
            '$id': 'https://schemas.oteryn.invalid/source/wiki-spec-supplement-v1',
            '$defs': defs, **obj(root, root)}


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    output = Path(__file__).with_name('wiki_source_specs.schema.json')
    content = json.dumps(build(), indent=2) + '\n'
    if args.check:
        if output.read_text() != content:
            raise SystemExit('SOURCE schema differs from generator')
    else:
        output.write_text(content)
