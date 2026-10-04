"""Compose private source-complete authoring snapshots; never alter v1 or a consumer."""
import copy
import json
from pathlib import Path
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

HERE = Path(__file__).resolve().parent
SPELL_URI = 'urn:oteryn:spell-authoring:source-complete:candidate:2'


def compose(extension_paths):
    """Keep all v1 fields and add closed, versioned native parameter alternatives."""
    spell = json.loads((HERE / 'spell.schema.json').read_bytes())
    original_native = copy.deepcopy(spell['$defs']['nativeBehavior'])
    spell['$id'] = SPELL_URI
    spell['title'] = 'Private source-complete Spell authoring candidate v2'
    spell['description'] = ('Local target DATA proposal. New authoring fields require contract and consumer '
                            'qualification; this schema does not admit execution or change v1.')
    alternatives = [original_native]
    resources, seen = [], set()
    for path in extension_paths:
        extension = json.loads(Path(path).read_bytes())
        uri = extension['$id']
        if uri in seen or uri == SPELL_URI:
            raise ValueError('duplicate private authoring schema URI')
        if 'nativeBehavior' not in extension.get('$defs', {}):
            raise ValueError('private extension must expose closed nativeBehavior definition')
        Draft202012Validator.check_schema(extension)
        seen.add(uri)
        alternatives.append({'$ref': uri + '#/$defs/nativeBehavior'})
        if 'sourceStateContract' in extension['$defs']:
            if uri != 'urn:oteryn:source-state-native-extensions:1':
                raise ValueError('source state contract must use its private owning schema')
            spell['$defs']['spell']['properties']['source_state_contract'] = {
                '$ref': uri + '#/$defs/sourceStateContract'}
        resources.append(extension)
    if not seen:
        raise ValueError('source-complete composition requires actual typed extensions')
    spell['$defs']['nativeBehavior'] = {'anyOf': alternatives}
    # Source party helpers remove only the computed delta during the callback;
    # default post-cast payment still charges their unchanged registrar mana.
    # Keep the v1 mana=0 rule for every existing party profile. Only the closed
    # private r61 party model carries its own complete payment timeline.
    source_party = {'properties': {'execution': {'properties': {
        'native_behavior': {'properties': {
            'key': {'const': 'party_buff'},
            'parameters': {'properties': {'source_model': {'const': 'r61-party_buff'}},
                           'required': ['source_model']}},
            'required': ['key', 'parameters']}}, 'required': ['native_behavior']}},
        'required': ['execution']}
    for rule in spell['$defs']['spell'].get('allOf', []):
        if ('party_buff' in json.dumps(rule.get('if', {}))
                and rule.get('then', {}).get('properties', {}).get('costs', {})
                .get('properties', {}).get('mana') == {'const': 0}):
            rule['if'] = {'allOf': [rule['if'], {'not': source_party}]}
    Draft202012Validator.check_schema(spell)
    return spell, resources


def validator(extension_paths, other_schemas):
    spell, extensions = compose(extension_paths)
    documents = [*other_schemas, *extensions, spell]
    seen = {}
    for document in documents:
        uri = document['$id']
        if uri in seen and seen[uri] != document:
            raise ValueError('conflicting schema resource identity')
        seen[uri] = document
    registry = Registry().with_resources((uri, Resource.from_contents(document))
                                        for uri, document in seen.items())
    return Draft202012Validator(spell, registry=registry), spell, registry
