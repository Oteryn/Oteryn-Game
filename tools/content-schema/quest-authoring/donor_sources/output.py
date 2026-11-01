"""Generated donor output identity is the exact payload, independent of gzip encoding."""
import gzip
import hashlib
import json


def payload(path, data):
    return gzip.decompress(data) if path.suffix == '.gz' else data


def payload_sha256(path):
    return hashlib.sha256(payload(path, path.read_bytes())).hexdigest()


def matches_output(path, data):
    return path.is_file() and payload(path, path.read_bytes()) == payload(path, data)


def write_output(path, data):
    if not matches_output(path, data):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def quest_source_sha256(path):
    """Fingerprint Quest inputs without the generated donor evidence that consumes them."""
    value = json.loads(path.read_bytes())
    if path.name == 'index.json':
        value.pop('authoring_sources', None)
    else:
        for record in value['records']:
            record['definition'].pop('oteryn_recipe', None)
    raw = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()
    return hashlib.sha256(raw).hexdigest()
