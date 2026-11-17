"""Per-qualification reuse of exact source bytes with isolated consumer records."""
import copy
from collections.abc import MutableMapping
from contextlib import contextmanager
from contextvars import ContextVar
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'content-schema/item-authoring'))
import engine_items

_decode_appearance_objects = engine_items.load_appearance_objects

_appearance_inputs = ContextVar("appearance_inputs", default=None)
_deleted_appearance = object()


@contextmanager
def appearance_input_scope():
    """Reuse exact bytes within one qualification; never retain data between passes."""
    if _appearance_inputs.get() is not None:
        yield
        return
    token = _appearance_inputs.set({})
    try:
        yield
    finally:
        _appearance_inputs.reset(token)


class _AppearanceView(MutableMapping):
    """Call-local mutable records over an inaccessible shared decoded bank."""
    def __init__(self, source):
        self._source = source
        self._local = {}

    def __getitem__(self, key):
        if key not in self._local:
            self._local[key] = copy.deepcopy(self._source[key])
        value = self._local[key]
        if value is _deleted_appearance:
            raise KeyError(key)
        return value

    def __setitem__(self, key, value):
        self._local[key] = value

    def __contains__(self, key):
        if key in self._local:
            return self._local[key] is not _deleted_appearance
        return key in self._source

    def get(self, key, default=None):
        if key not in self:
            return default
        return self[key]

    def __delitem__(self, key):
        self[key]
        self._local[key] = _deleted_appearance

    def __iter__(self):
        for key in self._source:
            if self._local.get(key) is not _deleted_appearance:
                yield key
        for key, value in self._local.items():
            if key not in self._source and value is not _deleted_appearance:
                yield key

    def __len__(self):
        return sum(1 for _ in self)

    def copy(self):
        return dict(self.items())

    def __deepcopy__(self, memo):
        return copy.deepcopy(dict(self.items()), memo)


def qualified_input_view(kind, data, decode):
    """Private parsed input, with a separate mutable view for each consumer."""
    cache = _appearance_inputs.get()
    if cache is None:
        return decode()
    key = (kind, bytes(data))
    if key not in cache:
        cache[key] = decode()
    return _AppearanceView(cache[key])


def load_appearance_objects(data):
    return qualified_input_view('appearances', data, lambda: _decode_appearance_objects(data))


# Outside a qualification this remains the original uncached decoder. Imported
# aliases may keep using it directly; they retain the same source checks.
engine_items.load_appearance_objects = load_appearance_objects
