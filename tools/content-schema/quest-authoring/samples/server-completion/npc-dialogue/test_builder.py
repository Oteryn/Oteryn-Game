import base64
import gzip
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import builder


def name(value):
    return {'node_type': 'Name', 'fields': {'id': value}, 'span': None}


def literal(value, escaped=False):
    return {'node_type': 'String', 'fields': {
        'raw': value if not escaped else value + '\\n',
        's': {'bytes_base64': base64.b64encode(value.encode()).decode()},
        'delimiter': {'name': 'DOUBLE_QUOTE'}}, 'span': None}


def invoke(method, arguments):
    return {'node_type': 'Invoke', 'fields': {
        'func': name(method), 'source': name('npcHandler' if method == 'say' else 'keywordHandler'), 'args': arguments}, 'span': None}


def table(value, key=None):
    return {'node_type': 'Table', 'fields': {'fields': [
        {'node_type': 'Field', 'fields': {'key': name(key) if key else None,
                                       'value': value}, 'span': None}]}, 'span': None}


class StageCandidatesTests(unittest.TestCase):
    def source(self, ast, mutate_container=False):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        chunks = []
        for pointer, node in builder.walk(ast):
            if node['node_type'] == 'String':
                text = chr(34) + node['fields']['raw'] + chr(34)
                start = len(' '.join(chunks)) + (1 if chunks else 0)
                node['span'] = {'start_char': start, 'end_char_exclusive': start + len(text)}
                chunks.append(text)
        raw = ' '.join(chunks).encode()
        source_sha = builder.sha(raw)
        blob_sha = hashlib.sha1(b'blob ' + str(len(raw)).encode() + b'\0' + raw).hexdigest()
        (root / 'npc.lua').write_bytes(raw)
        identity = {'source': 'canary', 'revision': 'pin', 'path': 'npc.lua',
                    'source_sha256': source_sha, 'git_blob_sha1': blob_sha}
        row = dict(identity, sha256=source_sha, cache_path='npc.lua')
        (root / 'manifest.json').write_text(json.dumps({'files': [row]}))
        (root / 'captures').mkdir()
        capture = gzip.compress(json.dumps({'ast': ast, 'raw_bytes_base64': base64.b64encode(raw).decode()}).encode())
        (root / 'captures' / (source_sha + '.json.gz')).write_bytes(capture)
        (root / 'index.json').write_text(json.dumps({'captures': [{
            'sha256': source_sha, 'container_sha256': 'bad' if mutate_container else builder.sha(capture)}]}))
        cache = builder.SourceCache(root / 'manifest.json', root)
        return cache, identity

    def registration(self, trigger, reply, callback=None):
        callback = callback or {'node_type': 'Index', 'fields': {'value': name('StdModule'), 'idx': name('say')}, 'span': None}
        return invoke('addKeyword', [table(literal(trigger)), callback, table(literal(reply), 'text')])

    def test_trigger_literal_is_not_reply_witness(self):
        cache, identity = self.source(self.registration('quest', 'Actual reply'))
        self.assertEqual([r['text'] for r in cache.strings(identity)], ['Actual reply'])

    def test_custom_callback_not_assumed_to_speak_text(self):
        cache, identity = self.source(self.registration('quest', 'Hidden', name('custom')))
        self.assertEqual(cache.strings(identity), [])

    def test_speech_only_first_argument(self):
        cache, identity = self.source(invoke('say', [literal('Reply'), literal('recipient-name')]))
        self.assertEqual([r['text'] for r in cache.strings(identity)], ['Reply'])

    def test_player_speech_not_npc_reply(self):
        call = invoke('say', [literal('Player emote')])
        call['fields']['source'] = name('player')
        cache, identity = self.source(call)
        self.assertEqual(cache.strings(identity), [])

    def test_escape_literal_held(self):
        cache, identity = self.source(invoke('say', [literal('Reply', escaped=True)]))
        self.assertEqual(cache.strings(identity), [])

    def test_wrong_container_rejected(self):
        cache, identity = self.source(invoke('say', [literal('Reply')]), True)
        with self.assertRaises(ValueError):
            cache.strings(identity)

    def test_wrong_identity_no_filename_guess(self):
        cache, identity = self.source(invoke('say', [literal('Reply')]))
        identity['source_sha256'] = 'unknown'
        self.assertEqual(cache.strings(identity), [])

    def test_source_dispatch_not_promoted(self):
        cache, identity = self.source(invoke('say', [literal('Reply')]))
        row = cache.strings(identity)[0]
        self.assertFalse(row['dispatch_proven'])
        self.assertFalse(row['source_spec_complete'])

    def test_duplicate_parent_prevents_child_selection(self):
        keyword = {'key': 'quest', 'triggers': ['quest'], 'reply': ['Reply'],
                   'children': [{'key': 'yes', 'triggers': ['yes'], 'reply': ['Reply']}]}
        rows = list(builder.branches([keyword, keyword], '/keywords'))
        self.assertTrue(all(not r[-1] for r in rows))

    def test_branch_hint_stays_derived(self):
        stage = {'objective': 'Ask about mission', 'targets': ['Named NPC']}
        quest = {'display_name': 'Example Quest'}
        rows = builder.choose_branches(stage, quest, 'Named NPC', {'keywords': [
            {'key': 'mission', 'triggers': ['mission'], 'reply': ['An adventure awaits.']}]}, '/keywords')
        self.assertEqual(rows[0]['scope'], 'DERIVED_GENERIC_QUEST_TRIGGER')
        self.assertFalse(rows[0]['native_bound'])

    def test_named_target_does_not_select_name_branch(self):
        rows = builder.choose_branches({'objective': 'Talk with Named NPC'},
            {'display_name': 'Example Quest'}, 'Named NPC', {'keywords': [
            {'key': 'name', 'triggers': ['name'], 'reply': ['I am Named NPC.']}]}, '/keywords')
        self.assertEqual(rows, [])


if __name__ == '__main__':
    unittest.main()
