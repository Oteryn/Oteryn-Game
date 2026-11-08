"""Install only the three owned Thaian declarations produced by the normal creature Stage."""
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
PACKET='tools/content-schema/quest-authoring/questpc7-thaian-authoring.json'
BATCH='questpc7-thaian-wiki-chosen-r1'
REV='questpc7-thaian-wiki-445240-chosen-r1'


def install(root=ROOT):
    raw=(root/PACKET).read_bytes();packet=json.loads(raw);digest=hashlib.sha256(raw).hexdigest()
    expected={'oteryn:creature.thaian','oteryn:behavior.creature.thaian','oteryn:presentation.creature.thaian'}
    assert {r['identity']['key'] for r in packet['records']}==expected
    assert {r['target']['key'] for r in packet['profiles']}==expected
    assert packet['source']['page_id']==48123 and packet['source']['revision']==445240
    def merge(rows,extra,field):
        keys={r[field]['key'] for r in extra}
        return sorted([r for r in rows if r[field]['key'] not in keys]+extra,key=lambda r:(r[field]['family'],r[field]['key'],r[field]['revision']))
    paths=['definitions/reference.json','definitions/declarations.json','provenance/sources.json','provenance/imports.json']
    docs=[json.loads((root/'content/world'/p).read_bytes()) for p in paths]
    reference,declarations,sources,imports=docs
    reference['records']=merge(reference['records'],packet['records'],'identity')
    declarations['authoring_profiles']=merge(declarations['authoring_profiles'],packet['profiles'],'target')
    sources['source_identity_bindings']=[r for r in sources['source_identity_bindings'] if r['source_revision']!=REV]+packet['bindings']
    sources['sources']=[r for r in sources['sources'] if r['import_batch_id']!=BATCH]+[dict(key='oteryn:source.tibiawiki',import_batch_id=BATCH,revision=REV,sha256=digest,evidence='Derived')]
    imports['batches']=[r for r in imports['batches'] if r['batch_id']!=BATCH]+[dict(batch_id=BATCH,source_repository='tibiawiki.com.br',source_revision=REV,source_artifact_sha256=digest,access_disposition='PENDING',source_generation_profile=packet['schema'],importer='OTERYN_QUESTPC7_THAIAN_AUTHORING/v1',mapper='OTERYN_CREATURE_ADMISSION_STAGE/v1',mapper_revision='creature-admission-r1',mapper_sha256=hashlib.sha256((root/'tools/content-migration/creature_admission_stage.py').read_bytes()).hexdigest(),candidates=[],reimport_states=[])]
    for path,document in zip(paths,docs):
        target=root/'content/world'/path;encoded=(json.dumps(document,ensure_ascii=False,sort_keys=True,separators=(",",":"))+'\n').encode()
        if target.read_bytes()!=encoded:target.write_bytes(encoded)

    import questpc7_world_seal
    questpc7_world_seal.refresh(root)


if __name__=='__main__':install()
