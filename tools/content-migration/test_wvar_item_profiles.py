import os
import unittest
from pathlib import Path
import wvar_item_profiles as wvar

ARCHIVE=Path(os.environ.get("OTERYN_TEST_CRYSTAL_ARCHIVE", "/tmp/wvar-crystal.tar.gz"))

class WvarItemProfilesTests(unittest.TestCase):
    @unittest.skipUnless(ARCHIVE.is_file(), "optional pinned donor archive unavailable")
    def test_pinned_native_sandfish_is_portable_stackable_and_edible(self):
        data=wvar.build_archive(ARCHIVE)
        self.assertEqual(len(data['records']),1)
        row=data['records'][0]
        self.assertEqual(row['production_definition']['production_key'],'oteryn:item.tibia.i13992')
        self.assertEqual(row['semantics']['consumption'],{'state':'KNOWN','value':{'kind':'FOOD','value':{'regeneration_seconds':660}}})
        self.assertEqual(row['semantics']['physical']['value']['weight'],{'state':'KNOWN','value':660})
        self.assertEqual(row['admission'],{'materializable':True,'stack_class':'StackCapable','legal_destinations':['CharacterInventory','Ground']})
        self.assertEqual(row['semantics']['container'],{'state':'NOT_APPLICABLE'})
    def test_native_membership_is_installed_once_without_replacing_other_profiles(self):
        import json
        generated={'profiles':b'{"records":[]}', 'manifest':b'{"item_profiles":{"sha256":"old"}}'}
        canonical=lambda value:(json.dumps(value,sort_keys=True)+'\n').encode()
        order=lambda row:row['production_definition']['production_key']
        wvar.admit(wvar.ROOT,generated,'profiles','manifest',order,canonical)
        self.assertEqual(len(json.loads(generated['profiles'])['records']),1)
        with self.assertRaisesRegex(ValueError,'WVAR_ITEM_PROFILE_DUPLICATE'):
            wvar.admit(wvar.ROOT,generated,'profiles','manifest',order,canonical)

    @unittest.skipUnless(ARCHIVE.is_file(), "optional pinned donor archive unavailable")
    def test_altered_donor_bytes_cannot_grant_native_admission(self):
        sources=wvar.read_archive(ARCHIVE)
        sources[0]=(sources[0][0],sources[0][1]+b'\n')
        with self.assertRaisesRegex(ValueError,'WVAR_ITEM_SOURCE_HASH'):
            wvar.build_sources(sources)

if __name__=='__main__': unittest.main()
