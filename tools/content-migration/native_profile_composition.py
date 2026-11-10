"""Combine source knowledge without replacing an already known mechanical fact."""
import copy
import json
from contextlib import contextmanager
from contextvars import ContextVar


_compact_native_profiles = ContextVar('compact_native_profiles', default=False)


@contextmanager
def compact_native_profile_scope():
    """Use compact intermediate banks while registration owns final byte encoding."""
    token = _compact_native_profiles.set(True)
    try:
        yield
    finally:
        _compact_native_profiles.reset(token)


def encode_native_profile(document):
    """Keep standalone provider bytes; avoid pretty-printing transient full banks."""
    options = {'separators': (',', ':')} if _compact_native_profiles.get() else {'indent': 2}
    # Preserve key insertion order in both modes. The registration owner performs
    # the final sorted canonical encoding before publishing any provider digest.
    return (json.dumps(document, ensure_ascii=False, **options) + '\n').encode()


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
