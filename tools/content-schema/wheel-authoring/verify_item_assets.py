"""Reproduce Gem item appearance references from the existing 15.30 assets.

This is reference evidence, not a client asset admission or a perk UI crosswalk.
"""
import hashlib
import json
import sys
from pathlib import Path
from wheel_authoring import ROOT, read

REPO = ROOT.parents[2]
sys.path.insert(0,str(ROOT.parent/'item-authoring'))
from engine_items import load_appearance_objects

def build_reference():
    files=REPO/'content/assets/files'
    manifest_path=REPO/'imports/official/client-assets/15.30/manifest.json'
    manifest=read(manifest_path)
    entries={entry['name']:entry for entry in manifest['files']}
    inputs={}
    def checked(filename):
        data=(files/filename).read_bytes();digest=hashlib.sha256(data).hexdigest()
        if digest!=entries[filename]['sha256'] or len(data)!=entries[filename]['bytes']:
            raise ValueError('ASSET_INPUT_DIGEST: '+filename)
        inputs['content/assets/files/'+filename]=digest
        return data
    appearance='appearances-2dfa943b548472a1ddc7bc5afe97945bc75e14f1f41d74f728f8e622f5dae7e2.dat'
    records=load_appearance_objects(checked(appearance))
    catalogue=json.loads(checked('catalog-content.json'))
    sprites=[entry for entry in catalogue if entry['type']=='sprite']
    parameters=read(ROOT/'samples/source-parameters.json')
    keys=[key for family in parameters['gem_families'].values() for key in family['items'].values()]
    keys+=list(parameters['gems']['fragment_items'].values())+[parameters['gems']['operation_policy']['crusher_item']]
    items={}
    for key in keys:
        identifier=int(key.split('.i')[-1]);record=records[identifier];groups=[]
        for group in record['frame_groups']:
            atlases=[]
            for sprite in group['sprite_ids']:
                matches=[s for s in sprites if s['firstspriteid']<=sprite<=s['lastspriteid']]
                if len(matches)!=1:raise ValueError('SPRITE_ATLAS_BINDING')
                atlas=matches[0];checked(atlas['file'])
                atlases.append({'sprite_id':sprite,**atlas})
            groups.append({'geometry':group['geometry'],'atlases':atlases})
        items[key]={'appearance_object_id':identifier,'frame_groups':groups}
    return {'schema':'OTERYN_WHEEL_ITEM_ASSET_REFERENCE/v1','client_version':'15.30',
        'manifest_sha256':hashlib.sha256(manifest_path.read_bytes()).hexdigest(),
        'input_digests':inputs,'items':items,'runtime_admitted':False,
        'perk_ui_crosswalk':'NOT_PRESENT_IN_THIS_CATALOGUE'}

if __name__=='__main__':
    path=ROOT/'samples/item-asset-reference.json'
    rendered=json.dumps(build_reference(),indent=2,allow_nan=False)+'\n'
    if '--check' in sys.argv:
        if path.read_text()!=rendered:raise ValueError('ITEM_ASSET_REFERENCE_DRIFT')
    else:path.write_text(rendered)
    print('PASS: 18 Gem item appearances and sprite atlas references; reference only.')
