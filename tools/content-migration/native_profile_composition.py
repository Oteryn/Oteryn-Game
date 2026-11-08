"""Combine source knowledge without replacing an already known mechanical fact."""
import copy


def merge_fact(existing, source, path):
    if isinstance(source, dict) and source.get('state') == 'UNKNOWN':
        return copy.deepcopy(existing)
    if isinstance(existing, dict) and existing.get('state') == 'UNKNOWN':
        return copy.deepcopy(source)
    if type(existing) is not type(source):
        raise ValueError(f'NATIVE_PROFILE_FACT_CONFLICT:{path}')
    if isinstance(existing, dict):
        if existing.keys() != source.keys():
            raise ValueError(f'NATIVE_PROFILE_FACT_CONFLICT:{path}')
        return {key: merge_fact(existing[key], source[key], f'{path}/{key}')
                for key in existing}
    if isinstance(existing, list):
        if len(existing) != len(source):
            raise ValueError(f'NATIVE_PROFILE_FACT_CONFLICT:{path}')
        return [merge_fact(old, new, f'{path}/{index}')
                for index, (old, new) in enumerate(zip(existing, source))]
    if existing != source:
        raise ValueError(f'NATIVE_PROFILE_FACT_CONFLICT:{path}')
    return copy.deepcopy(existing)
