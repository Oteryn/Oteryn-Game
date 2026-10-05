"""Closed registry of source-qualified native spell authoring families.

These definitions describe complete candidate mechanics. Runtime admission is
separate: registering a schema never installs a domain mutation executor.
"""
import importlib
import inspect

from jsonschema import Draft202012Validator

MODULE_NAMES = (
    'native_actor_states', 'native_combat', 'native_companions',
    'native_delayed', 'native_house_movement', 'native_world_items',
)


def modules():
    return [importlib.import_module(name) for name in MODULE_NAMES]


def schemas():
    result = {}
    for module in modules():
        for key, schema in module.schemas().items():
            if key in result:
                raise ValueError(f'duplicate native behavior schema: {key}')
            # The shared builder supplies the key/parameters envelope.
            if set(schema.get('properties', {})) == {'key', 'parameters'}:
                raise ValueError(f'{module.__name__}: expected parameter schema for {key}')
            result[key] = schema
    return result


def build(name, spell_type, records, texts):
    results = []
    for module in modules():
        result = module.build(name, spell_type, records, texts)
        recognized = spell_type == 'instant' and str(name).casefold() in {
            'native_actor_states': getattr(module, 'MODELS', {}),
            'native_combat': getattr(module, 'TEMPLATES', {}),
            'native_delayed': getattr(module, 'FILES', {}),
        }.get(module.__name__, {})
        if result is None and recognized:
            raise ValueError(f'{module.__name__}: recognized spell source qualification failed: {name}')
        if result is not None:
            results.append((module, result))
    if not results:
        return None
    if len(results) != 1:
        raise ValueError(f'ambiguous native behavior ownership: {spell_type} {name}')
    module, result = results[0]
    if set(result) != {'key', 'parameters'}:
        raise ValueError(f'{module.__name__}: invalid native behavior envelope')
    schema = module.schemas().get(result['key'])
    if schema is None:
        raise ValueError(f'{module.__name__}: missing schema for {result["key"]}')
    Draft202012Validator(schema).validate(result['parameters'])
    evidence_fn = getattr(module, 'evidence', None)
    if evidence_fn is None:
        raise ValueError(f'{module.__name__}: missing source qualification evidence')
    parameters = inspect.signature(evidence_fn).parameters
    if len(parameters) == 4:
        evidence = evidence_fn(name, spell_type, records, texts)
    elif len(parameters) == 3:
        evidence = evidence_fn(name, records, texts)
    elif len(parameters) == 1:
        evidence = evidence_fn(name)
    else:
        raise ValueError(f'{module.__name__}: unsupported evidence interface')
    return result, {'family_module': module.__name__, 'observations': evidence}


def barrier(name, spell_type, records, texts):
    if spell_type != 'rune':
        return None
    module = importlib.import_module('native_world_items')
    return module.barrier_effect(name, records, texts)


def barrier_evidence(name, spell_type, records, texts):
    module = importlib.import_module('native_world_items')
    return {'family_module': module.__name__,
            'observations': module.evidence(name, spell_type, records, texts)}
