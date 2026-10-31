#!/usr/bin/env python3
"""TUTORIAL-1: derive source-backed Newhaven plain Item profiles using the existing pinned importer.

Only --capture reads references. No original source/appearance files are redistributed.
"""
import argparse
import hashlib
import json
from pathlib import Path

import build_source_item_profiles as source
from build_spell_item_identities import BINDINGS, CANARY_PIN, binding_index, bind_profiles
from lower_item_consumption_packet import fetch

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = Path(__file__).with_name('movement-item-profiles.json')
PATH_IDS = {3892, 3893, 6216, 6217, *range(6373, 6383), 6385, 19394, 37001}
# Unprofiled plain Items placed on Newhaven; solid and unknown source flags stay out.
# Starter operands 3272,3559,21348 have their own source overlay.
NEWHAVEN_IDS = {
    620, 650, 1050, 1634, 1635, 1636, 1637, 1755, 1757, 1759, 1760, 1761, 1762, 1763,
    1764, 1767, 1768, 1781, 1816, 1854, 1855, 1856, 1857, 1858, 1865, 1866, 1867, 1892,
    1894, 1895, 1896, 1897, 1898, 1899, 1900, 1913, 1996, 1998, 1999, 2001, 2316, 2319,
    2370, 2371, 2378, 2381, 2384, 2481, 2511, 2512, 2513, 2514, 2515, 2516, 2517, 2518,
    2519, 2608, 2619, 2662, 2694, 2695, 2696, 2697, 2698, 2699, 2700, 2701, 2726, 2730,
    2736, 2744, 2853, 2855, 2892, 2894, 2897, 2899, 2905, 2908, 2910, 2929, 2931, 2937,
    2939, 2974, 3106, 3108, 3109, 3111, 3112, 3113, 3115, 3116, 3117, 3118, 3119, 3121,
    3123, 3125, 3126, 3127, 3128, 3132, 3133, 3139, 3294, 3337, 3466, 3467, 3471, 3474,
    3475, 3509, 3595, 3654, 3656, 3657, 3665, 3667, 3886, 3888, 3889, 3890, 3891, 3894,
    3906, 3907, 3914, 4980, 4981, 4985, 4986, 4998, 5000, 5017, 5024, 5295, 5296, 5297,
    5298, 5416, 5482, 5679, 5779, 5780, 5781, 5782, 5784, 5789, 5951, 6215, 6218, 6219,
    6220, 6221, 6222, 6223, 6224, 6226, 6227, 6229, 6230, 6356, 6362, 6383, 6384, 6391,
    6473, 6474, 6475, 6476, 6477, 6478, 6479, 6480, 6481, 6482, 6483, 6484, 6485, 6486,
    6487, 6981, 6982, 6986, 7137, 7138, 7139, 7143, 7144, 7523, 8817, 9480, 9621, 9671,
    9821, 9822, 9823, 10490, 10491, 10493, 10965, 11182, 11183, 11809, 11817, 11818, 11819, 11820,
    11823, 11824, 11825, 11826, 11827, 11828, 11831, 11832, 11834, 12648, 12649, 15689, 16573, 17396,
    17397, 17398, 17399, 17400, 17401, 17402, 17403, 17404, 18435, 21419, 21421, 25387, 28155, 28156,
    31166, 31167, 31327, 32411, 32417, 33722, 33724, 33725, 33726,
}
IDS = PATH_IDS | NEWHAVEN_IDS


def capture(directory=None):
    proof = json.loads((ROOT / 'docs/reference/spells/r21-local-candidate/active-artifact/item-source-proof.json').read_bytes())
    captured = {}
    sources = []
    for row in proof['loader_sources']:
        if row['path'] not in ('data/items/appearances.dat', 'data/items/items.xml'):
            continue
        raw = ((directory / Path(row['path']).name).read_bytes() if directory else
               fetch(f'https://raw.githubusercontent.com/opentibiabr/canary/{CANARY_PIN}/{row["path"]}'))
        if hashlib.sha256(raw).hexdigest() != row['sha256']:
            raise ValueError('pinned source digest mismatch')
        captured[row['path']] = raw
        sources.append(row)
    xml = source.xml_index(captured['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(captured['data/items/appearances.dat'])
    physical = {row['server_item_id']: row for row in source.policies(captured['data/items/appearances.dat'], captured['data/items/items.xml'], IDS)}
    bindings = binding_index(BINDINGS.read_bytes())
    records = [source.profile(i, xml.get(i), objects[i], physical[i], bindings, None) for i in sorted(IDS)]
    # Preserve admitted shared stats while adding the source physical/map facts.
    from build_newhaven_item_profiles import fill_unknown
    admitted = {}
    for path in sorted((ROOT / 'content/items/definitions').glob('items-*.json')):
        for item in json.loads(path.read_bytes())['records']:
            definition = item['definition']
            if 'semantics' in definition:
                admitted[definition['identity']['key']] = definition['semantics']
    for row in records:
        item = int(row['authoring']['item']['key'].rsplit('/', 1)[1])
        row['semantics'] = fill_unknown(admitted.get(f'oteryn:item.tibia.i{item}', {'state': 'UNKNOWN'}), row['semantics'])
        if row['attributes']['blocks_movement'] is not False:
            raise ValueError('MOVEMENT_ITEM_PROFILES_NOT_WALKABLE')
    bound = bind_profiles({'schema': 'OTERYN_NATIVE_ITEM_PROFILES/v1', 'records': records}, bindings, IDS)
    return {'schema': 'OTERYN_NATIVE_MOVEMENT_ITEM_PROFILES/v1',
            'producer': 'tools/content-schema/native-gameplay/build_tutorial_movement_profiles.py',
            'sources': sources, 'records': bound['records']}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture', action='store_true', required=True)
    parser.add_argument('--source-directory', type=Path)
    args = parser.parse_args()
    OUTPUT.write_text(json.dumps(capture(args.source_directory), indent=2, ensure_ascii=False) + '\n')
