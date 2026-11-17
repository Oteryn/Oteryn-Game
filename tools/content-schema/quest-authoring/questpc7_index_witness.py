"""Bind unchanged native-palette evidence to this generation's planned index."""
import hashlib
import json

INDEX = 'content/world/placements/index.json'


def rebind(root, callbacks, generated):
    before = (root / INDEX).read_bytes()
    after = generated[INDEX]
    old, new = json.loads(before), json.loads(after)
    old.pop('authored_props', None)
    new.pop('authored_props', None)
    if old != new:
        raise ValueError('questpc7 planned index changes native palette or regions')
    prefix = INDEX + ':1:sha256:'
    expected = prefix + hashlib.sha256(before).hexdigest()
    replacement = prefix + hashlib.sha256(after).hexdigest()
    seen = set()
    for callback in callbacks:
        sources = callback['sources']
        if id(sources) in seen:
            continue
        seen.add(id(sources))
        for index, source in enumerate(sources):
            if source.startswith(prefix):
                if source != expected:
                    raise ValueError('questpc7 stale native index witness')
                sources[index] = replacement
