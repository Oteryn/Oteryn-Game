"""TUTORIAL-1 movement overlay; fail closed on the exact record set and walkability."""
import hashlib
import json

PATH = 'tools/content-schema/native-gameplay/movement-item-profiles.json'
IDS = {3892, 3893, 6216, 6217, *range(6373, 6383), 6385, 19394, 37001} | {
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


def admit(root, generated, profile_path, manifest_path, order, canonical):
    overlay = json.loads((root / PATH).read_bytes())
    records = overlay['records']
    keys = [row['production_definition']['production_key'] for row in records]
    if (overlay['schema'] != 'OTERYN_NATIVE_MOVEMENT_ITEM_PROFILES/v1'
            or len(keys) != len(IDS) or set(keys) != {f'oteryn:item.tibia.i{i}' for i in IDS}):
        raise ValueError('MOVEMENT_ITEM_PROFILES_RECORD_SET')
    if any(row['attributes']['blocks_movement'] is not False for row in records):
        raise ValueError('MOVEMENT_ITEM_PROFILES_NOT_WALKABLE')
    document = json.loads(generated[profile_path])
    if {row['production_definition']['production_key'] for row in document['records']} & set(keys):
        raise ValueError('MOVEMENT_ITEM_PROFILES_RECORD_SET')
    document['records'] = sorted(document['records'] + records, key=order)
    data = (json.dumps(document, indent=2, ensure_ascii=False) + '\n').encode()
    generated[profile_path] = data
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
