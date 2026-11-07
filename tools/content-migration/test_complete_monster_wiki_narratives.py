import copy
import unittest
import complete_monster_wiki_narratives as m


class NarrativeTests(unittest.TestCase):
    def page(self):
        return {'page_title': 'Rat', 'revision_id': 12, 'content_sha256': 'a'*64,
                'url': 'https://tibiawiki.com.br/wiki/Rat', 'method': 'actual_capture',
                'fields': {'behavior': "'''Luta''' até a morte.<br>[[Rat|Rato]].",
                           'notes': '<spoiler>{{Achievement|Rat Slayer}}</spoiler>'},
                'field_lines': {'behavior': 8, 'notes': 9}}

    def test_rendering_preserves_language_links_and_map_coordinates(self):
        self.assertEqual(m.paragraphs("[[NPC|Pessoa]] {{mapa|1,2,3:1|aqui}}"), ['Pessoa aqui (1,2,3:1)'])
        self.assertEqual(m.paragraphs('<spoiler>Não foge.</spoiler>'), ['Não foge.'])

    def test_news_and_dash_require_captured_helpers(self):
        with self.assertRaises(m.UnsupportedMarkup):
            m.paragraphs('{{OfficialNewsArchive|1234}}')
        helpers = {'Predefinição:OfficialNewsArchive': {}, 'Predefinição:DASH': {}}
        self.assertEqual(m.paragraphs('{{OfficialNewsArchive|1234|Artigo}} {{DASH|Aviso}}', helpers),
                         ['Artigo (https://www.tibia.com/news/?subtopic=newsarchive&id=1234) Aviso'])
        self.assertEqual(m.paragraphs('[[Page|A [[Nested]]]]'), ['A Nested'])
        self.assertEqual(m.paragraphs('{{DASH}}'), [])
        self.assertEqual(m.paragraphs('<gallery>Arquivo:Image.png</gallery>Fato.'), ['Fato.'])

    def test_unknown_template_fails_entire_field_not_silent_deletion(self):
        with self.assertRaises(m.UnsupportedMarkup):
            m.paragraphs('Antes {{:Mecanica Unknown}} depois')
        p = self.page(); p['fields']['notes'] = '{{:Mecanica Unknown}}'
        rows, qa, flags, deferred = m.prepare_actor('rat', {}, {'documents': []}, p)
        self.assertEqual(rows[0]['value']['content'], ['Luta até a morte.', 'Rato.'])
        self.assertEqual(len(deferred), 1)
        self.assertIn('WIKI_NARRATIVE_SOURCE_FIELD_UNRENDERED', flags)
        self.assertEqual(qa[1]['raw'], '{{:Mecanica Unknown}}')

    def test_unique_definition_and_typed_reference_agree(self):
        rows, _, flags, _ = m.prepare_actor('rat', {}, {'documents': []}, self.page())
        identity = rows[0]['value']['identity']
        self.assertEqual(rows[1]['value']['description_document'], {'family': 'Document', **identity})
        self.assertEqual(rows[0]['value']['language'], 'pt-BR')
        self.assertFalse(rows[0]['expected_present'])
        self.assertEqual(rows[0]['pointer'], '/documents/-')

    def test_variant_reference_explicit_and_existing_untouched(self):
        rows, _, flags, _ = m.prepare_actor('old_rat', {}, {'documents': []}, self.page(), variant=True)
        self.assertIn('SOURCE_WIKI_NARRATIVE_SHARED_TITLE_REFERENCE', flags)
        self.assertEqual(rows[0]['value']['title'], 'Rat')
        old = {'encyclopedia': {'description_document': {'key': 'existing'}}}
        self.assertEqual(m.prepare_actor('rat', old, {'documents': []}, self.page())[0], [])

    def test_inputs_immutable_and_no_source_parity_claim(self):
        p=self.page(); original=copy.deepcopy(p)
        rows, _, _, _=m.prepare_actor('rat', {}, {'documents': []}, p)
        self.assertEqual(p, original)
        self.assertEqual(rows[0]['source']['fields']['notes']['raw'], original['fields']['notes'])


if __name__ == '__main__':
    unittest.main()
