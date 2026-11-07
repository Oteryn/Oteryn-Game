import copy
import hashlib
import json
from pathlib import Path
import unittest
import build_spell_appearances as producer

class AppearanceSourceTests(unittest.TestCase):
    def test_actual_authored_native_outfits_have_complete_qualified_values(self):
        document=json.loads((Path(__file__).parent/'spell_appearances.json').read_text())
        self.assertEqual(len(document['records']),6)
        self.assertEqual(document['default_source_sha256'],producer.DEFAULT_SHA)
        for original in document['records']:
            row=copy.deepcopy(original)
            proof=row.pop('qualification_sha256')
            self.assertEqual(proof,hashlib.sha256(json.dumps(row,sort_keys=True,separators=(',',':')).encode()).hexdigest())
            self.assertEqual(row['source']['revision'],producer.PIN)
            self.assertEqual(row['outfit_key'],f'oteryn:outfit.source.canary.look{row["look_type"]}')
            self.assertEqual(set(row['source']['explicit_fields'])|set(row['source']['defaulted_fields']),set(producer.FIELDS))
        rat=next(r for r in document['records'] if r['creature'])
        self.assertEqual(rat['creature']['key'],'canary:creature/rat')
        self.assertEqual(rat['source']['defaulted_fields'],['lookTypeEx'])
        avatar=next(r for r in document['records'] if r['look_type']==1593)
        self.assertIn('lookHead',avatar['source']['defaulted_fields'])

    def test_lua_expression_duplicate_and_unknown_fields_are_not_defaults(self):
        for raw in [b'monster.outfit={lookType=LOOK_RAT}',b'monster.outfit={lookType=21,lookType=22}',
                    b'monster.outfit={lookType=21,lookShader=4}']:
            with self.assertRaises(ValueError): producer.outfit(raw)

    def test_explicit_zero_is_preserved_and_comments_do_not_supply_fields(self):
        self.assertEqual(producer.outfit(b'monster.outfit={--lookHead=45\nlookType=21,lookHead=0,}'),
                         {'lookType':21,'lookHead':0})

if __name__=='__main__': unittest.main()
