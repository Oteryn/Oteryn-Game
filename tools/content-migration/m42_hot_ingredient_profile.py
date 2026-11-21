"""Exact recipe-16 food ingredient: loot category is not an Item use/timer effect."""
import json
import qengine_reward_item_profiles as items

PROOF = 'tools/content-schema/native-gameplay/m42-hot-ingredient-profile.json'


def admit(root, generated, profile_path, manifest_path, order, canonical):
    attrs = items.source.attrs(items.source.xml_index((root / items.XML).read_bytes())[48273])
    if attrs != {'loottype': 'creature products', 'primarytype': 'food', 'weight': '20'}:
        raise ValueError('M42_INGREDIENT_EXACT_SOURCE_ATTRIBUTES')
    # The reusable qualifier checks both immutable source banks, exact canonical
    # physical/stack/container facts and binding. No food-use effect is invented.
    packet = items.capture(root, ids={48273})
    packet['scope'] = 'M42 exact recipe ingredient custody; no custom use or loot-category activation.'
    generated[PROOF] = (json.dumps(packet, ensure_ascii=False, indent=2) + '\n').encode()
    items.admit_records(root, generated, profile_path, manifest_path, order, canonical, packet['records'])
