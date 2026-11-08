"""Chosen Morla map contact; no source side missions or whole-quest completion inferred."""
import hashlib
import json
import tarfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
PIN = '9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'
DIGEST = '6d4996501d42f7750acf94e99f8e9b6fc36e65d88d77ae190f5d73875b5b62ec'


def extend_rewards(callbacks, greetings, recipe, chosen_source):
    stage = recipe['stages'][1]
    if stage['kind'] != 'explore' or stage['targets'] != ['Morla / Aragonia', 'Old Treasure Map']:
        raise ValueError('TARGUNA_CHOSEN_MAP_CONTACT_DRIFT')
    with tarfile.open(ROOT / 'tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz') as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        entry, = [r for r in manifest['files'] if r['file_id'] == f'crystalserver:{PIN}:data-global/npc/morla.lua']
        raw = archive.extractfile(entry.get('blob_path', entry.get('cache_path'))).read()
    if hashlib.sha256(raw).hexdigest() != DIGEST or b'OLD_TREASURE_MAP_ID = 53158' not in raw:
        raise ValueError('TARGUNA_MORLA_MAP_SOURCE_DRIFT')
    sources = [chosen_source, f'crystal:{PIN}:data-global/npc/morla.lua:58,110-114:sha256:{DIGEST}']
    npc = 'oteryn:npc.morla'
    greetings.append(dict(npc=npc, greet='Welcome to Aragonia. Ask about the old treasure map.',
        farewell='Safe travels.', walkaway='Return when you are ready.', sources=sources))
    callbacks.append(dict(key='oteryn:quest-reward/questpc5-targuna-morla-map',
        quest='oteryn:quest.targuna_quest', npc=npc, keywords=['map', 'mission'], topic=0, next_topic=0,
        storage=[dict(track='oteryn:quest-progress/targuna/stage', minimum=0, maximum=0, set=1)],
        take=[], give=dict(item='oteryn:item.tibia.i53158', count=1, contents=[]), outfit=None, achievement=None,
        reply=['The map shows encrypted coordinates. Take it to Sterling.'], sources=sources))
