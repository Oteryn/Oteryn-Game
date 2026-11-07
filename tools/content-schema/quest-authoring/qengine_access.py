"""Exact pinned source bindings for the QENGINE-1 shared access operations."""
import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / 'content/quests/shared-access-callbacks.json'
PIN = 'crystal:201189f871d23597b82caf88fd611454a2ddefb8:'


def position(x, y, z):
    return dict(x=x, y=y, floor=-z)


def callback(name, quest, trigger, source, *, unique=None, action=None,
             at=None, appearance=None, lever=None, reversible=False,
             passages=None, teleport=None, **effects):
    return dict(key='oteryn:quest-callback/' + name,
                quest='oteryn:quest.' + quest, trigger=trigger,
                selector=dict(position=at, unique=unique, action=action,
                              appearance=appearance),
                lever=lever, reversible=reversible, passages=passages or [],
                teleport=teleport, storage=[], offerings=[],
                return_on_failure=False, failure_message=None,
                effects=dict(transform=None, facing=None, magic=None,
                             portals=None, item_cycle=None, encounter=None) | effects,
                sources=[PIN + source])


def document():
    thais = 'data-global/scripts/quests/thais_lighthouse/'
    stairs = [dict(position=position(32225, 32282, 9), appearance=429, replacement=7767)]
    rows = [callback('thais-lighthouse-plate' + suffix, 'thais_lighthouse_quest', trigger,
                     thais + 'movement-stair.lua:' + lines, unique=25010, appearance=431, at=position(32225,32268,9),
                     passages=stairs, transform=appearance)
            for suffix, trigger, appearance, lines in
            [('', 'step_in', 430, '6-16'), ('-out', 'step_out', 431, '24-33')]]
    rows.append(callback('thais-lighthouse-portals', 'thais_lighthouse_quest', 'use',
                         thais + 'action-lever.lua:17-46', unique=30005,
                         appearance=2772, at=position(32225,32285,10), lever=[2772, 2773], reversible=True,
                         portals=dict(appearance=1949, lifetime_us=None, portals=[
                             dict(position=position(32232, 32276, 9), destination=position(32225, 32274, 10)),
                             dict(position=position(32225, 32276, 10), destination=position(32230, 32276, 9))])))
    rows.append(callback('triangle-tower-lever', 'triangle_tower_quest', 'use',
                         'data-global/scripts/quests/triangle_tower_quest/actions_triangle_tower_lever.lua:3-18',
                         unique=30034, appearance=2772, at=position(32573,32121,7), lever=[2772, 2773], reversible=True,
                         item_cycle=dict(position=position(32566, 32119, 7), remove=1270, create=1025)))
    for name, at, to, facing, splash in [
        ('cliff', (32966, 32626, 5), (32968, 32626, 5), None, (32968, 32626, 5)),
        ('splash', (32968, 32630, 7), None, None, (32968, 32630, 7)),
        ('cave-entry', (32968, 32629, 7), (32968, 32631, 8), 'south', None),
        ('cave-exit', (32967, 32630, 8), (32971, 32620, 8), 'east', None),
    ]:
        rows.append(callback('waterfall-' + name, 'waterfall_quest', 'step_in',
                             'data-global/scripts/quests/waterfall/movements_waterfall.lua:16-32',
                             action=50022, at=position(*at), teleport=position(*to) if to else None,
                             facing=facing, magic=dict(position=position(*splash), effect=54) if splash else None))
    # The UID30015 data-crystal example is a different map, not the global encounter.
    rows.append(callback('annihilator-entry', 'the_annihilator', 'use',
                         'data-global/scripts/quests/the_annihilator/lever.lua:1-101',
                         unique=30025, appearance=2772, at=position(33226,31671,13), lever=[2772, 2773],
                         encounter=dict(minimum_level=100,
                             players=[dict(from_=position(x, 31671, 13), to=position(x-3, 31659, 13))
                                      for x in [33225, 33224, 33223, 33222]],
                             monsters=[dict(position=position(x,y,13), creature='oteryn:creature.angry_demon')
                                       for x,y in [(33219,31657),(33221,31657),(33223,31659),
                                                   (33224,31659),(33220,31661),(33222,31661)]],
                             room_min=position(33217,31655,13), room_max=position(33225,31663,13))))
    reward_door = callback('annihilator-reward-door', 'the_annihilator', 'use',
                           'data-global/scripts/quests/the_annihilator/door.lua:4-25',
                           action=10102, appearance=5113, lever=[5113, 5114], reversible=True,
                           reward_door=dict(claim='oteryn:reward-claim.quest.u7_24.the_annihilator.reward'))
    reward_door['failure_message'] = 'The door seems to be sealed against unwanted intruders.'
    reward_door['sources'] = [
        'canary:04b83b512114bfd888000d6e1433ed8ecaec7c5b:'
        'data-otservbr-global/scripts/quests/the_annihilator/door.lua:4-25'
        '#sha256:03c6833cf9c9e4c335e2d33716b1180159696eb45cf1772d9e09e6036ef50ca1']
    rows.append(reward_door)
    for row in rows:
        encounter = row['effects']['encounter']
        if encounter:
            for player in encounter['players']:
                player['from'] = player.pop('from_')
    return dict(schema=1, callbacks=sorted(rows, key=lambda row: row['key']))


def verify_reward_door_source():
    # The retained corpus and manifest pins are shared with the encounter proof.
    import hashlib
    import importlib.util
    import tarfile
    path = ROOT / 'tools/content-migration/group_encounter_callbacks.py'
    spec = importlib.util.spec_from_file_location('pinned_group_source', path)
    group = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(group)
    group.verify_source()
    blob = '1170e128e15ee05582db092987beea7a6935726a'
    digest = '03c6833cf9c9e4c335e2d33716b1180159696eb45cf1772d9e09e6036ef50ca1'
    with tarfile.open(ROOT / group.ARCHIVE) as archive:
        manifest = json.load(archive.extractfile('corpus-manifest.json'))
        rows = [row for row in manifest['files'] if row['source'] == 'canary'
                and row['path'] == 'data-otservbr-global/scripts/quests/the_annihilator/door.lua']
        if len(rows) != 1 or rows[0]['revision'] != group.REVISION or rows[0]['git_blob_sha1'] != blob or rows[0]['sha256'] != digest:
            raise ValueError('REWARD_DOOR_SOURCE_BINDING')
        helper = '2ab1104989a5d1f50669dbb1e2d2aeef2f7c8dec'
        helper_digest = 'a34f5bad153897359cd1f2499b6cf189d32c61e3d7ae18dd35a67eeedfd357c7'
        rows = [row for row in manifest['files'] if row['source'] == 'canary'
                and row['path'] == 'data/libs/functions/creature.lua']
        if len(rows) != 1 or rows[0]['revision'] != group.REVISION or rows[0]['git_blob_sha1'] != helper or rows[0]['sha256'] != helper_digest:
            raise ValueError('REWARD_DOOR_CLOSE_SOURCE_BINDING')
        helper_body = archive.extractfile('blobs/' + helper).read()
        if hashlib.sha256(helper_body).hexdigest() != helper_digest or hashlib.sha1(f'blob {len(helper_body)}\0'.encode() + helper_body).hexdigest() != helper:
            raise ValueError('REWARD_DOOR_CLOSE_SOURCE_BYTES')
        body = archive.extractfile('blobs/' + blob).read()
        if hashlib.sha256(body).hexdigest() != digest or hashlib.sha1(f'blob {len(body)}\0'.encode() + body).hexdigest() != blob:
            raise ValueError('REWARD_DOOR_SOURCE_BYTES')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    verify_reward_door_source()
    text = json.dumps(document(), ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if OUTPUT.read_text() != text:
            raise ValueError('stale shared access callbacks')
    else:
        OUTPUT.write_text(text)


if __name__ == '__main__':
    main()
