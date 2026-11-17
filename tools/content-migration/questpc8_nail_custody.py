"""Reuse the already admitted harvest nail stack at the callback boundary."""
import copy
import hashlib
import json

PATH = 'tools/content-schema/native-gameplay/harvest-item-profiles.json'
SHA256 = '75d32d5a109da7c9c5e51c3d977d8e02dc7682bab01216892d600881fbe11b42'


def callback_definition(root, native, definition):
    if definition['identity']['key'] != 'oteryn:item.tibia.i953':
        return definition
    raw = (root / PATH).read_bytes()
    if hashlib.sha256(raw).hexdigest() != SHA256:
        raise ValueError('QUESTPC8_NAIL_SOURCE_PIN')
    source = next(row for row in json.loads(raw)['records']
                  if row['production_definition']['production_key'] == definition['identity']['key'])
    if (native is None or any(native[field] != source[field] for field in
                             ('production_definition', 'production_binding', 'admission'))
            or native['semantics']['stack'] != source['semantics']['stack']
            or definition['stack_class'] != 'NonStackable'
            or definition['semantics']['stack'] != {'state': 'UNKNOWN'}):
        raise ValueError('QUESTPC8_NAIL_SOURCE_CONFLICT')
    effective = copy.deepcopy(definition)
    effective['stack_class'] = source['admission']['stack_class']
    effective['semantics']['stack'] = copy.deepcopy(source['semantics']['stack'])
    return effective
