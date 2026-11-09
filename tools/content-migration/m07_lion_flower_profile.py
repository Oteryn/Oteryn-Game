"""Finite Lion flower annotation from Crystal 201189f8 items.xml:42586-42590.

The donor duration is 3600 seconds. The accepted runtime represents that lifetime
with an absolute deadline; no replacement item is declared by the donor.
The shared XML/client pins remain mandatory; no other timed item is admitted.
"""
import copy


def admitted(item_id, attributes):
    return (item_id == 21389
            and attributes.get('duration') == '3600'
            and attributes.get('weight') == '100'
            and attributes.keys() <= {'description', 'duration', 'weight'})


def temporal(record, canonical, known_conflict):
    value = record['semantics']['temporal']['value']
    value['consumption_mode'] = {'state': 'KNOWN', 'value': 'DURABLE_ABSOLUTE_DEADLINE'}
    value['decay_target'] = {'state': 'NOT_APPLICABLE'}
    if known_conflict(canonical['semantics']['temporal'], record['semantics']['temporal']):
        raise ValueError('M07_LION_FLOWER_TEMPORAL_CONFLICT')
    return copy.deepcopy(record)
