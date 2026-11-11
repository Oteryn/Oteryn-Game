"""Close the existing Galthen loot tables with exact Canary carried Item policies."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

import boss_corpse_profiles as base
import m25_corpse_items

ROOT = base.ROOT
PATH = 'tools/content-schema/native-gameplay/galthen-boss-loot.json'
PIN_PATH = 'tools/content-schema/native-gameplay/galthen-boss-loot-pin.json'
RUST_PIN = 'crates/gameplay/content/src/content/native_gameplay/galthen_loot_pin.rs'
IDS = (40527, 40528, 40529, 40535, 40578, 44438, 44439, 44619, 44620,
       44636, 44637, 44642, 44643, 44648, 44649, 44727, 44728, 50255, 50291)
CANARY = m25_corpse_items.REVISIONS['canary']
SOURCES = {**m25_corpse_items.SOURCE_HASHES['canary'],
           'data-otservbr-global/monster/undeads/ahau.lua':
           '9236c2dd65d960e81efc5ed8e9fe251967147856d3fe2e2322013d83ced0d3d8'}


def complete_source_abilities(row, attributes):
    """Reuse the equipment owner's typed, signed modifier/resistance decoder."""
    from lower_equip_abilities_packet import canary_modifiers, canary_resistances
    import lower_mantra_bond_modifier_packet as mantra
    _, modifiers = canary_modifiers({key: value for key, value in attributes.items()
                                    if key != 'mantra'})
    _, resistances = canary_resistances(attributes)
    if any(value is not None and not isinstance(value, list)
           for value in (modifiers, resistances)):
        raise ValueError('M25_LOOT_EQUIPMENT_SOURCE')
    if 'mantra' in attributes:
        typed = mantra.whole_vector({'mantra': [attributes['mantra']]}, mantra.shared.native_order())
        if typed is None:
            raise ValueError('M25_LOOT_EQUIPMENT_SOURCE')
        modifiers = sorted((modifiers or []) + typed['value'], key=lambda entry: entry['kind'])
    armor = None
    if 'armor' in attributes:
        try:
            armor = int(attributes['armor'])
        except (TypeError, ValueError) as error:
            raise ValueError('M25_LOOT_EQUIPMENT_SOURCE') from error
        if not -(2**31) <= armor < 2**31:
            raise ValueError('M25_LOOT_EQUIPMENT_SOURCE')
    # Validate both groups before publishing either one.
    if modifiers is not None:
        row['semantics']['skill_modifiers'] = base.source.K({
            'modifiers': base.source.K(modifiers)})
    if resistances is not None or armor is not None:
        row['semantics']['protection'] = base.source.K({
            'armor': base.source.K(armor) if armor is not None else base.source.U,
            'resistances': base.source.K(resistances) if resistances is not None else base.source.U})


def table_proofs(root):
    selected = []
    used = set()
    for path in sorted((root / 'content/loot').glob('loot-*.json')):
        for record in json.loads(path.read_bytes())['records']:
            table = record['definition']
            if table['identity']['key'] in ('oteryn:loot.creature.ahau', 'oteryn:loot.creature.mitmah_vanguard'):
                used.update(entry['item']['key'] for entry in table['entries'])
                selected.append({'key': table['identity']['key'],
                                 'sha256': hashlib.sha256(base.encode(table)).hexdigest()})
    if len(selected) != 2 or not {f'oteryn:item.tibia.i{item}' for item in IDS} <= used:
        raise ValueError('M25_LOOT_EXACT_TABLES')
    return sorted(selected, key=lambda row: row['key'])


def capture(canary, crystal, root=ROOT):
    from build_full_spell_artifact import native_loot_yield_facts
    import imbuement_item_profiles as imbue
    blobs = {path: (canary / path).read_bytes() for path in SOURCES}
    if any(hashlib.sha256(raw).hexdigest() != SOURCES[path] for path, raw in blobs.items()):
        raise ValueError('M25_LOOT_SOURCE_HASH')
    source = base.source
    crystal_xml = (crystal / 'data/items/items.xml').read_bytes()
    if hashlib.sha256(crystal_xml).hexdigest() != base.SOURCE_HASHES['data/items/items.xml']:
        raise ValueError('M25_LOOT_CRYSTAL_SOURCE_HASH')
    slots = imbue.slots(blobs['data/items/items.xml'], set(IDS))
    crystal_slots = imbue.slots(crystal_xml, set(IDS))
    if ({item: row['slot_count'] for item, row in slots.items()}
            != {item: row['slot_count'] for item, row in crystal_slots.items()}):
        raise ValueError('M25_LOOT_IMBUEMENT_DONOR_CONFLICT')
    # Existing Imbuement policy makes Canary the value source. Crystal201189f
    # has OT tier10 boots and omits one robe family; retain those differences
    # as evidence rather than importing them into the current Tibia policies.
    differences = {item: {'canary': row, 'crystal': crystal_slots[item]}
                   for item, row in slots.items() if row != crystal_slots[item]}
    if set(differences) != {44648, 44649, 50255}:
        raise ValueError('M25_LOOT_IMBUEMENT_DIFFERENCE_SET')
    xml = source.xml_index(blobs['data/items/items.xml'])
    objects = source.engine_items.load_appearance_objects(blobs['data/items/appearances.dat'])
    physics = {row['server_item_id']: row for row in source.policies(
        blobs['data/items/appearances.dat'], blobs['data/items/items.xml'], set(IDS))}
    bindings = base.binding_index((root / 'imports/crystalserver/bindings/items.json').read_bytes())
    taxonomy = {row['target']['key']: row['source_taxonomy'] for row in
                json.loads((root / 'content/items/taxonomy/items.json').read_bytes())['records']}
    records = []
    for item in IDS:
        binding = bindings[str(item)]
        if binding['disposition'] != 'EXACT' or binding['target'] != {
                'family': 'Item', 'key': f'oteryn:item.tibia.i{item}', 'revision': 'definition-r1'}:
            raise ValueError('M25_LOOT_EXISTING_IDENTITY')
        row = source.profile(item, xml.get(item), objects[item], physics[item], bindings, None)
        base.complete_source_top_order(row, objects[item])
        complete_source_abilities(row, source.attrs(xml[item]) if item in xml else {})
        if item in slots:
            fact = slots[item]
            row['semantics']['imbuement'] = source.K({
                'slot_count': source.K(fact['slot_count']),
                'allowed_family_tiers': source.K([
                    {'family': family, 'tier': imbue.TIERS[fact['families'][family]]}
                    for family in imbue.ORDER if family in fact['families']]),
                'excluded_families': source.K([])})
        row['production_binding'] = copy.deepcopy(binding)
        row['production_binding_qualification'] = None
        row['production_definition'] = {'family': 'Item', 'production_key': binding['target']['key'],
                                        'revision_ref': binding['target']['revision']}
        if binding['target']['key'] in taxonomy:
            row['authoring']['taxonomy'] = copy.deepcopy(taxonomy[binding['target']['key']])
        if native_loot_yield_facts(binding['target'], row) is None:
            raise ValueError('M25_LOOT_CARRIED_SOURCE:' + str(item))
        records.append(row)
    return {'source_revision': CANARY, 'tables': table_proofs(root),
            'crystal_cross_check': {'revision': base.PIN, 'path': 'data/items/items.xml',
                                    'sha256': base.SOURCE_HASHES['data/items/items.xml']},
            'imbuement_slots': slots,
            'imbuement_value_source': 'Canary47df, matching the existing Imbuement owner and no-OT-features decision',
            'crystal_family_differences': differences,
            'sources': [{'repository': 'opentibiabr/canary', 'path': path, 'sha256': sha}
                        for path, sha in SOURCES.items()], 'records': records}


def admit(root, generated, destination, manifest_path, order, canonical):
    raw = (root / PATH).read_bytes()
    if json.loads((root / PIN_PATH).read_bytes()) != {'path': PATH, 'sha256': hashlib.sha256(raw).hexdigest()}:
        raise ValueError('M25_LOOT_CAPTURE_PIN')
    packet = json.loads(raw)
    if (packet['source_revision'] != CANARY or packet['tables'] != table_proofs(root)
            or [int(row['production_binding']['external_id']) for row in packet['records']] != list(IDS)):
        raise ValueError('M25_LOOT_CLOSED_TABLES')
    document = json.loads(generated[destination])
    existing = {row['production_definition']['production_key']: row for row in document['records']}
    for row in packet['records']:
        key = row['production_definition']['production_key']
        if key in existing and existing[key] != row:
            raise ValueError('M25_LOOT_KNOWN_POLICY_CONFLICT:' + key)
        if key not in existing:
            document['records'].append(row)
    document['records'].sort(key=order)
    generated[destination] = canonical(document)
    manifest = json.loads(generated[manifest_path])
    manifest['item_profiles']['sha256'] = hashlib.sha256(generated[destination]).hexdigest()
    generated[manifest_path] = canonical(manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--canary-root', type=Path, required=True)
    parser.add_argument('--crystal-root', type=Path, required=True)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    raw = base.encode(capture(args.canary_root, args.crystal_root))
    sha = hashlib.sha256(raw).hexdigest()
    rust = ('// Generated by tools/content-migration/galthen_boss_loot.py.\n'
            'pub(super) const CAPTURE_SHA256: &str =\n    "' + sha + '";\n').encode()
    for path, value in {PATH: raw, PIN_PATH: base.encode({'path': PATH, 'sha256': sha}), RUST_PIN: rust}.items():
        if args.check:
            if (ROOT / path).read_bytes() != value:
                raise ValueError('M25_LOOT_GENERATED_DRIFT:' + path)
        else:
            (ROOT / path).write_bytes(value)
    print('PASS M25 source-qualified carried yields', len(IDS))


if __name__ == '__main__':
    main()
