import copy,json,shutil
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
import import_precombat_healing_candidates as importer

class PrecombatHealingImportTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.prepared={rev:importer.prepare_import(importer.ROOT,rev) for rev in ['r42','r44']}
 def test_both_exact_reviewed_families_and_base_archived_schema_bytes(self):
  for revision,(data,manifest) in self.prepared.items():
   self.assertEqual(manifest['source_keys'],[importer.CONFIGS[revision]['registration_key']])
   self.assertEqual(manifest['counts'],{'records':1,'status_counts':{'CANDIDATE_SCHEMA_VALID':1},'abilities':1,'effects':2,'formulas':1})
   self.assertEqual(manifest['source_package']['sha256'],importer.CONFIGS[revision]['manifest_sha256'])
   self.assertFalse(manifest['runtime_activation']);self.assertFalse(manifest['canonical_selection_changed'])
   for name,body in data.items():
    if name.startswith('schemas/'):self.assertEqual(body,(importer.ROOT/'imports/spells/r28'/name).read_bytes())
 def test_rehashed_source_mutation_refused_by_review_pin(self):
  with TemporaryDirectory(dir=importer.ROOT) as tmp:
   source=Path(tmp)/'packet';shutil.copytree(importer.ROOT/importer.CONFIGS['r42']['source'],source)
   member=next(source.glob('*/*/receipt.json'));row=json.loads(member.read_bytes());row['runtime_activation']=True;member.write_bytes(importer.encoded(row))
   path=source/'package-manifest.json';manifest=json.loads(path.read_bytes());manifest['files'][member.relative_to(source).as_posix()]=importer.digest(member.read_bytes());path.write_bytes(importer.encoded(manifest))
   with self.assertRaisesRegex(ValueError,'SOURCE_REVIEW_PIN_MISMATCH'):importer.prepare_import(importer.ROOT,'r42',source)
 def test_base_tampering_and_unknown_family_refused_before_write(self):
  with TemporaryDirectory() as tmp:
   root=Path(tmp);path=root/'imports/spells/r28/import-manifest.json';path.parent.mkdir(parents=True);path.write_bytes(b'tamper')
   with self.assertRaisesRegex(ValueError,'BASE_MANIFEST_PIN_MISMATCH'):importer.prepare_import(root,'r42')
   with self.assertRaisesRegex(ValueError,'UNSUPPORTED_PRECOMBAT_FAMILY'):importer.prepare_import(root,'r43')
 def candidate(self):
  data,manifest=self.prepared['r42'];packet={n.split('/',1)[1]:b for n,b in data.items() if n.startswith('player-source-candidates/')}
  spell=json.loads(next(b for n,b in packet.items() if n.endswith('/spell.json')));deps=json.loads(next(b for n,b in packet.items() if n.endswith('/dependencies.json')))
  return spell,deps,spell['spell']['identity']['key'],spell['spell']['identity']['revision']
 def test_dependency_unresolved_reference_wrong_namespace_and_unused_definition_rejected(self):
  for change,expected in [('ref','DEPENDENCY_REFERENCE_UNRESOLVED'),('namespace','DEPENDENCY_NAMESPACE_MISMATCH'),('count','DEPENDENCY_POPULATION_MISMATCH')]:
   spell,deps,key,revision=self.candidate()
   if change=='ref':spell['spell']['execution']['ability']['key']+='/missing'
   elif change=='namespace':deps['formulas'][0]['identity']['revision']='source-player-r99'
   else:deps['effects'].append(copy.deepcopy(deps['effects'][0]))
   with self.assertRaisesRegex(ValueError,expected):importer.validate_dependency_closure(spell,deps,{'definitions':[]},key,revision)
 def test_precombat_phase_or_external_catalog_not_silently_accepted(self):
  spell,deps,key,revision=self.candidate();deps['effects'][0]['presentation']['caster_effect_timing']='after_combat'
  with self.assertRaisesRegex(ValueError,'PRECOMBAT_HEALING_SEMANTICS_MISMATCH'):importer.validate_dependency_closure(spell,deps,{'definitions':[]},key,revision)
  spell,deps,key,revision=self.candidate()
  with self.assertRaisesRegex(ValueError,'EXTERNAL_CATALOG_REFUSED'):importer.validate_dependency_closure(spell,deps,{'definitions':[{}]},key,revision)
 def test_wrong_family_destination_false_flags_and_altered_bytes_refused(self):
  data,manifest=self.prepared['r42']
  with TemporaryDirectory() as tmp:
   root=Path(tmp)
   with self.assertRaisesRegex(ValueError,'WRONG_PRECOMBAT_IMPORT_FAMILY'):importer.write_import(root,'r44',root/'imports/spells/r44',data,manifest)
   with self.assertRaisesRegex(ValueError,'IMPORT_DESTINATION_REFUSED'):importer.write_import(root,'r42',root/'imports/spells/r28',data,manifest)
   active=copy.deepcopy(manifest);active['native_execution_qualified']=True
   with self.assertRaisesRegex(ValueError,'ACTIVE_OR_EQUIVALENT_CLAIM_REFUSED'):importer.write_import(root,'r42',root/'imports/spells/r42',data,active)
   altered=dict(data);altered[next(n for n in altered if n.endswith('/spell.json'))]=b'changed'
   with self.assertRaisesRegex(ValueError,'OUTPUT_PIN_MISMATCH'):importer.write_import(root,'r42',root/'imports/spells/r42',altered,manifest)
   self.assertFalse((root/'imports/spells/r42').exists())
 def test_actual_new_write_exact_copy_closure_protects_prior_sets_and_refuses_replay(self):
  for revision,(data,manifest) in self.prepared.items():
   with TemporaryDirectory() as tmp:
    root=Path(tmp);protected=root/'imports/spells/r28/import-manifest.json';protected.parent.mkdir(parents=True);protected.write_bytes(b'immutable')
    target=root/'imports/spells'/revision;result=importer.write_import(root,revision,target,data,manifest)
    self.assertFalse(result['runtime_activation']);self.assertEqual(protected.read_bytes(),b'immutable')
    self.assertEqual({p.relative_to(target).as_posix() for p in target.rglob('*') if p.is_file()},set(data)|{'import-manifest.json','SHA256SUMS'})
    for name,body in data.items():self.assertEqual((target/name).read_bytes(),body)
    with self.assertRaisesRegex(ValueError,'IMPORT_SET_ALREADY_EXISTS'):importer.write_import(root,revision,target,data,manifest)
if __name__=='__main__':unittest.main()
