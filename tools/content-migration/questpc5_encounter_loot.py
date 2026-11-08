"""Refresh owned encounter admissions from native facts without changing source drops."""
import hashlib
import importlib.util
import json

PATH = 'tools/content-schema/native-gameplay/loot-tables.json'
CORPSES = {5960: 7, 5973: 12, 5983: 10}


def apply(root, generated, profile_path, manifest_path, canonical):
    spec = importlib.util.spec_from_file_location('questpc5_loot_producer',
        root / 'tools/content-schema/native-gameplay/build_full_spell_artifact.py')
    producer = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(producer)
    canonical_facts = producer.item_facts()
    document = json.loads(generated.get(PATH, (root / PATH).read_bytes()))
    profiles = {row['production_definition']['production_key']: row
                for row in json.loads(generated[profile_path])['records']}
    selected = {f'oteryn:item.tibia.i{number}' for number in CORPSES}
    for table in document['tables']:
        if table['identity']['key'] in {'oteryn:loot.creature.' + name
                                       for name in ('troll', 'dragon', 'minotaur_guard')}:
            selected.update(entry['item']['key'] for entry in table['entries'])
    for facts in document['items']:
        key = facts['item']['key']
        if key not in selected:
            continue
        profile = profiles.get(key)
        if profile is None:
            raise ValueError('QUESTPC5_ENCOUNTER_NATIVE_ITEM_MISSING:' + key)
        number = int(key.rsplit('.i', 1)[1])
        if number in CORPSES:
            capacity = producer.native_corpse_capacity(facts['item'], profile)
            if capacity != CORPSES[number]:
                raise ValueError('QUESTPC5_ENCOUNTER_CORPSE_CAPACITY:' + key)
            facts.update(materializable=True, container_capacity=capacity, stackable=False)
        else:
            base = canonical_facts[key]
            native = base if base['materializable'] else producer.native_loot_yield_facts(facts['item'], profile)
            if native is not None:
                facts.update(native)
            else:
                raise ValueError('QUESTPC5_ENCOUNTER_LOOT_NOT_ADMITTED:' + key)
    data = generated[PATH] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest['loot_tables']['sha256'] = hashlib.sha256(data).hexdigest()
    generated[manifest_path] = canonical(manifest)
