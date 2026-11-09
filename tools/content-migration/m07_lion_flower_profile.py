"""Finite Lion flower annotation; preserve source duration without inventing decay.

Crystal201189 items.hpp:302 defaults decayTo=-1; item_parse.cpp:370-375
sets only decayTime for duration; item.cpp:3377-3394 refuses decayTo<0.
The chosen Lion programme explicitly leaves original schedules uncertified.
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
    value['consumption_mode'] = {'state': 'NOT_APPLICABLE'}
    value['decay_target'] = {'state': 'NOT_APPLICABLE'}
    if known_conflict(canonical['semantics']['temporal'], record['semantics']['temporal']):
        raise ValueError('M07_LION_FLOWER_TEMPORAL_CONFLICT')
    return copy.deepcopy(record)
