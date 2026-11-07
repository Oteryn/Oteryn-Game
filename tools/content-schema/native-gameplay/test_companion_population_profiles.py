"""Exact canonical source population and bounded active provider regressions."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch
import build_companion_population_profiles as population
import build_full_spell_artifact as full
import build_spell_appearances as appearances

SOURCE=Path('/workspace/spells-r21-implemented/companion-population-profiles')
FAMILIARS=Path('/workspace/spells-r21-implemented/familiar-profiles')
SKELETON=Path('/workspace/spells-r21-implemented/skeleton-profiles')

@unittest.skipUnless(SOURCE.exists(),'requires the genuine canonical source export')
class CompanionPopulationTests(unittest.TestCase):
    def merged(self):
        creatures,presentations,_=full.merged_profiles(FAMILIARS)
        full.merge_skeleton(creatures,presentations,SKELETON)
        before=copy.deepcopy(creatures)
        proof=full.merge_companion_population(creatures,presentations,SOURCE)
        return creatures,presentations,proof,before

    def test_all_supported_canonical_policies_are_preserved_within_existing_bounds(self):
        creatures,presentations,proof,before=self.merged()
        self.assertEqual(156,proof['canary_eligible_count'])
        self.assertEqual(155,proof['added_creature_count'])
        self.assertEqual(162,len(creatures['records']))
        self.assertEqual(before['records'],creatures['records'][:7])
        self.assertLess(len(full.encoded(creatures)),8*1024**2)
        self.assertLess(len(full.encoded(presentations)),8*1024**2)
        self.assertTrue(all(p['byte_identical'] and p['source_initial_health']==p['source_maximum_health']
                            for p in proof['unchanged_full_definitions']))
        self.assertFalse(proof['ordinary_group_can_summon_all'])
        self.assertFalse(proof['ordinary_group_can_convince_all'])
        self.assertEqual([],proof['unsupported_source_profiles'])
        self.assertTrue(proof['rift_object_support_requested'])
        self.assertEqual(1,len(proof['unchanged_foreign_definitions']))

    def test_appearance_closure_uses_actual_source_bindings_for_canonical_identities(self):
        creatures,presentations,proof,_=self.merged()
        provider=appearances.build(Path('/workspace/spell-sources/canary'),creatures,proof['appearance_source_paths'])
        closure=full.qualify_appearance_closure(creatures,presentations,provider)
        self.assertEqual(125,closure['illusionable_creature_count'])
        self.assertEqual(130,len(provider['records']))
        wolf=next(r for r in provider['records'] if r['creature'] and r['creature']['key']=='oteryn:creature.wolf')
        self.assertEqual('definition-r1',wolf['creature']['revision'])
        self.assertEqual(proof['appearance_source_paths'][wolf['creature']['key']],wolf['source']['path'])

    def test_current_source_and_source_initial_health_cannot_be_substituted(self):
        original=full.read
        for mutation,message in [('blob','full Lua blob differs'),('health','initial health differs')]:
            def changed(path):
                raw,value=original(path)
                if path.name=='companion-population-source-proof.json':
                    value=copy.deepcopy(value)
                    if mutation=='blob':value['unchanged_full_definitions'][0]['current_git_blob']='0'*40
                    else:value['unchanged_full_definitions'][0]['source_initial_health']-=1
                return raw,value
            with patch.object(full,'read',side_effect=changed):
                with self.assertRaisesRegex(ValueError,message):self.merged()

    def test_source_dependency_carrier_is_actual_and_explicitly_unprofiled_where_required(self):
        dependency=json.loads((SOURCE/'companion-population-dependency-profiles.json').read_bytes())
        self.assertEqual(1842,len(dependency['records']))
        self.assertEqual(1209,len(dependency['authoring_profiles']))
        self.assertEqual([],dependency['missing_exact_records'])
        self.assertEqual(633,len(dependency['record_only_dependencies']))


    def test_object_rift_and_crystal_stag_retain_actual_distinct_source_authorities(self):
        creatures,presentations,proof,_=self.merged()
        by_key={r['target']['key']:r for r in presentations['records']}
        self.assertEqual('canary.appearance:object/2122',by_key['oteryn:presentation.creature.rift_fragment']['data']['profile']['asset_binding'])
        provider=appearances.build(Path('/workspace/spell-sources/canary'),creatures,proof['appearance_source_paths'])
        stag=next(r for r in provider['records'] if r['creature'] and r['creature']['key']=='oteryn:creature.stag')
        self.assertEqual('oteryn:outfit.source.crystal.look1913',stag['outfit_key'])
        self.assertEqual(appearances.CRYSTAL_PIN,stag['source']['revision'])
        self.assertEqual(appearances.CRYSTAL_DEFAULT_SHA,stag['source']['default_source']['sha256'])
        self.assertTrue(all('default_source' not in r['source'] for r in provider['records'] if r['source']['revision']==appearances.PIN))
        self.assertFalse(any(r['creature'] and r['creature']['key']=='oteryn:creature.rift_fragment' for r in provider['records']))

    def test_six_complete_source_conversions_preserve_normalized_acquisition_exclusion(self):
        audit=Path('/workspace/spells-r21-implemented/missing-companion-profiles')
        proof=full.qualify_missing_companion_audit(audit)
        self.assertEqual(0,proof['normalized_eligible_count'])
        self.assertEqual(6,len(proof['bundles']))
        self.assertTrue(all(not b['summoning']['convinceable'] and not b['summoning']['summonable'] for b in proof['bundles']))
        original=full.read
        def changed(path):
            raw,value=original(path)
            if path.name=='missing-companion-source-proof.json':
                value=copy.deepcopy(value);value['unchanged_full_definitions'][0]['normalized_summoning']['convinceable']=True
            return raw,value
        with patch.object(full,'read',side_effect=changed):
            with self.assertRaisesRegex(ValueError,'normalized exclusion was substituted'):
                full.qualify_missing_companion_audit(audit)

if __name__=='__main__':unittest.main()
