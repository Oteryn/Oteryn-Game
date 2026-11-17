"""Admit owned canonical Thaian data, reusing only the existing Ghost melee ability."""
import copy
import hashlib
import json
import questpc7_thaian_authoring as author
CREATURES='content/creatures/definitions/spell-native-profiles.json'
PRESENTATIONS='content/presentations/definitions/spell-native-profiles.json'
LOOT='tools/content-schema/native-gameplay/loot-tables.json'


def apply(root,generated,canonical):
    packet=json.loads((root/author.PACKET).read_bytes());profiles={p['target']['family']:p for p in packet['profiles']}
    creatures=json.loads(generated[CREATURES]);ghost,=[r for r in creatures['records'] if r['profile']['target']['key']=='oteryn:creature.ghost']
    melee=copy.deepcopy(ghost['monster_melee'])
    assert melee['source_sha256']=='af3651ff2920212b9616e86ebfdd54022a10d47c08c2db862f14c872d384af82'
    assert (melee['minimum'],melee['maximum'],melee['interval_ms'])==(0,80,2000)
    presentation=profiles['Presentation'];creature=profiles['Creature'];key=creature['target']['key']
    record=dict(profile=creature,behavior=profiles['Behavior'],presentation=presentation['target'],monster_melee=melee)
    creatures['records']=[r for r in creatures['records'] if r['profile']['target']['key']!=key]+[record]
    creatures['records'].sort(key=lambda r:r['profile']['target']['key'])
    pictures=json.loads(generated[PRESENTATIONS]);pictures['records']=[r for r in pictures['records'] if r['target']!=presentation['target']]+[presentation]
    pictures['records'].sort(key=lambda r:r['target']['key'])
    generated[CREATURES]=canonical(creatures);generated[PRESENTATIONS]=canonical(pictures)
    loot=json.loads(generated.get(LOOT,(root/LOOT).read_bytes()))
    loot['creature_loot']=[r for r in loot['creature_loot'] if r['creature']['key']!=key]+[dict(creature=creature['target'],loot=None)]
    loot['creature_loot'].sort(key=lambda r:r['creature']['key']);generated[LOOT]=canonical(loot)
    manifest=json.loads(generated['content/spells.manifest.json'])
    for field,path in [('creature_profiles',CREATURES),('presentation_profiles',PRESENTATIONS)]:manifest[field]['sha256']=hashlib.sha256(generated[path]).hexdigest()
    generated['content/spells.manifest.json']=canonical(manifest)
