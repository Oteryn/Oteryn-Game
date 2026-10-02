"""Exact-source Spyrat object-wire and Candy Horror Item closure facts; no identity guess."""
from __future__ import annotations
import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/content-schema/item-authoring'))
from appearance_membership import load_admitted, OUT_DIR, ADMITTED, manifest_name, INDEX_NAME
sys.path.insert(0, str(ROOT / 'tools/content-schema/world-authoring'))
from client_map_reader import fields

SOURCES = {'canary': ('47dfd51f45280a59a1d3e50ba7edd573d7234446', 'data-otservbr-global'),
           'crystal': ('00ce02a57ca5a12e48f32a3476e37471167e4c3f', 'data-global')}
IDS = (30375, 30376, 30377, 30378, 48267, 48296)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def direct_object_wire(function):
    return ('msg.add<uint16_t>(outfit.lookTypeEx);' in function
            and not re.search(r'Item::items|clientId|clientid', function))


def function_slice(text, signature):
    start = text.index(signature)
    brace = text.index('{', start)
    depth = 1
    end = brace + 1
    while depth:
        depth += (text[end] == '{') - (text[end] == '}')
        end += 1
    return {'start_line': text[:start].count('\n') + 1,
            'end_line': text[:end].count('\n') + 1, 'text': text[start:end]}


def pinned(checkout, revision, path):
    data = subprocess.check_output(['git', '-C', str(checkout), 'show', revision + ':' + path])
    assert (checkout / path).read_bytes() == data, path + ' differs from its exact donor Git pin'
    return data


# Stubs model only the prefix's context: a transformable corpse with a parent and index0.
# The branch deciding what happens to an absent target is inserted from donor C++ unchanged.
CPP_CONTEXT = r'''
#include <cstdint>
#include <iostream>
#include <map>
#include <memory>
#include <string>
namespace metrics { struct method_latency { explicit method_latency(const char*) {} }; }
#define __METRICS_METHOD_NAME__ "bounded-source-prefix-witness"
struct Item;
struct Tile;
struct Cylinder {
    std::shared_ptr<Tile> getTile() { return nullptr; }
    int32_t getThingIndex(std::shared_ptr<Item>) { return 0; }
};
struct Tile : Cylinder {};
struct ItemType { uint16_t id = 0; };
struct Item {
    static ItemType items[65536];
    std::shared_ptr<Cylinder> parent = std::make_shared<Cylinder>();
    uint16_t getID() { return 48267; }
    uint16_t getSubType() { return 1; }
    bool canTransform() { return true; }
    std::shared_ptr<Cylinder> getParent() { return parent; }
};
ItemType Item::items[65536];
struct Game {
    std::map<std::shared_ptr<Tile>, std::weak_ptr<Cylinder>> browseFields;
    std::shared_ptr<Item> transformItem(std::shared_ptr<Item>, uint16_t, int32_t);
};
'''
CPP_MAIN = r'''
int main() {
    Game game;
    auto corpse = std::make_shared<Item>();
    Item::items[48267].id = 48267;
    // 48296 is absent from both actual donor protobufs and XML. Default ItemType.id is0.
    auto absent = game.transformItem(corpse, 48296, -1);
    if (absent != corpse) return 1;
    // Synthetic valid target control reaches beyond the missing-target prefix.
    Item::items[3031].id = 3031;
    auto valid = game.transformItem(corpse, 3031, -1);
    if (valid != nullptr) return 2;
    std::cout << "ABSENT_TARGET_RETURNS_ORIGINAL;VALID_TARGET_PASSES_GUARD\n";
}
'''


def compile_transform_witness(function):
    marker = '\tconst ItemType &curType = Item::items[item->getID()];'
    prefix = function[:function.index(marker)]
    # nullptr is an explicit witness-only sentinel after the tested prefix, not source semantics.
    code = CPP_CONTEXT + prefix + '\nreturn nullptr;\n}\n' + CPP_MAIN
    with tempfile.TemporaryDirectory(prefix='oteryn-corpse-source-witness-') as directory:
        source, binary = Path(directory) / 'witness.cpp', Path(directory) / 'witness'
        source.write_text(code)
        compile_result = subprocess.run(['c++', '-std=c++20', '-O0', '-Wall', '-Wextra', '-Werror',
                                         str(source), '-o', str(binary)], capture_output=True, text=True)
        if compile_result.returncode:
            raise ValueError('bounded C++ witness failed to compile: ' + compile_result.stderr)
        run = subprocess.run([str(binary)], check=True, capture_output=True, text=True)
        assert run.stdout == 'ABSENT_TARGET_RETURNS_ORIGINAL;VALID_TARGET_PASSES_GUARD\n'
        return {'scope': 'Executed exact donor transformItem prefix with harness context; valid-target tail is sentinel only.',
                'source_prefix_sha256': sha(prefix.encode()), 'translation_unit_sha256': sha(code.encode()),
                'binary_sha256': sha(binary.read_bytes()),
                'compiler_version': subprocess.check_output(['c++', '--version']).decode().splitlines()[0],
                'result': run.stdout.strip(), 'runtime_engine_executed': False}


def verify(sources_dir=Path('/workspace/monster-reference-sources')):
    index, manifests = load_admitted()
    evidence, sources = {}, {}
    for label, (revision, datapack) in SOURCES.items():
        checkout = sources_dir / label
        paths = ('data/scripts/lib/register_monster_type.lua', 'src/lua/functions/lua_functions_loader.cpp',
                 'src/lua/functions/creatures/monster/monster_type_functions.cpp',
                 'src/creatures/monsters/monster.cpp', 'src/server/network/protocol/protocolgame.cpp',
                 'src/items/items.cpp', 'src/items/items.hpp', 'src/items/item.cpp', 'src/items/decay/decay.cpp',
                 'src/items/functions/item/item_parse.cpp', 'src/game/game.cpp',
                 'data/items/appearances.dat', 'data/items/items.xml',
                 datapack + '/monster/magicals/candy_horror.lua',
                 *(datapack + '/monster/raids/spyrat_facing_' + direction + '.lua'
                   for direction in ('east', 'north', 'south', 'west')))
        data = {path: pinned(checkout, revision, path) for path in paths}
        text = {path: value.decode() for path, value in data.items() if not path.endswith('.dat')}
        outfit = function_slice(text['src/server/network/protocol/protocolgame.cpp'], 'void ProtocolGame::AddOutfit(')
        assert direct_object_wire(outfit['text'])
        loader = function_slice(text['src/lua/functions/lua_functions_loader.cpp'], 'Outfit_t Lua::getOutfit(')
        assert 'outfit.lookTypeEx = getField<uint16_t>(L, arg, "lookTypeEx");' in loader['text']
        setter = function_slice(text['src/lua/functions/creatures/monster/monster_type_functions.cpp'],
                                'int MonsterTypeFunctions::luaMonsterTypeOutfit(')
        assert 'monsterType->info.outfit = outfit;' in setter['text']
        monster = text['src/creatures/monsters/monster.cpp']
        assert 'currentOutfit = mType->info.outfit;' in monster
        protocol = text['src/server/network/protocol/protocolgame.cpp']
        assert 'const Outfit_t &outfit = creature->getCurrentOutfit();\n\t\tAddOutfit(msg, outfit);' in protocol
        objects = {}
        for family, wire, payload in fields(data['data/items/appearances.dat']):
            if family == 1 and wire == 2:
                values = {number: value for number, _, value in fields(payload) if number in (1, 3)}
                if 1 in values and 3 in values:
                    objects[values[1]] = sha(payload)
        assert 48267 in objects and 3031 in objects
        assert all(number not in objects for number in (*IDS[:4], 48296))
        item_loader = function_slice(text['src/items/items.cpp'], 'void Items::loadFromProtobuf(')
        assert 'items[object.id()]' in item_loader['text']
        assert 'iType.id = static_cast<uint16_t>(object.id());' in item_loader['text']
        xml_gate = function_slice(text['src/items/items.cpp'], 'void Items::parseItemNode(')
        assert re.search(r'itemType.id == 0.*?itemType.name.empty\(\).*?return;', xml_gate['text'], re.S)
        xml = ET.fromstring(data['data/items/items.xml'])
        rows = {int(node.get('id')): node for node in xml.iter('item') if node.get('id')}
        attrs = {node.get('key').lower(): node.get('value') for node in rows[48267].findall('attribute')}
        assert 'uint16_t id = 0;' in text['src/items/items.hpp']
        assert attrs['decayto'] == '48296' and attrs['duration'] == '5' and 48296 not in rows
        transform = function_slice(text['src/game/game.cpp'], 'std::shared_ptr<Item> Game::transformItem(')
        assert re.search(r'if \(newType.id == 0\) \{\s*return item;', transform['text'])
        decay = function_slice(text['src/items/decay/decay.cpp'], 'void Decay::internalDecayItem(')
        assert 'g_game().transformItem(item, static_cast<uint16_t>(it.decayTo));' in decay['text']
        check_decay = function_slice(text['src/items/decay/decay.cpp'], 'void Decay::checkDecay(')
        assert re.search(r'item->setDecaying\(DECAYING_FALSE\);\s*internalDecayItem\(item\);', check_decay['text'])
        declared = {direction: int(re.search(r'lookTypeEx\s*=\s*(\d+)',
                    text[datapack + '/monster/raids/spyrat_facing_' + direction + '.lua'])[1])
                    for direction in ('east', 'north', 'south', 'west')}
        assert declared == dict(zip(('east', 'north', 'south', 'west'), IDS[:4]))
        sources[label] = {'revision': revision, 'files': {path: sha(value) for path, value in data.items()},
                          'source_object_presence': {str(number): objects.get(number) for number in IDS},
                          'source_item_loader': item_loader, 'source_xml_admission_gate': xml_gate,
                          'outfit_wire': outfit, 'lua_loader': loader, 'monster_outfit_setter': setter,
                          'item_transform_guard': transform, 'decay_dispatch': decay, 'decay_completion': check_decay,
                          'candy_horror_declared_corpse': int(re.search(r'monster.corpse\s*=\s*(\d+)',
                              text[datapack + '/monster/magicals/candy_horror.lua'])[1]),
                          'spyrat_declared_object_ids': declared, 'corpse_48267_xml_attributes': attrs,
                          'compiled_missing_target_witness': compile_transform_witness(transform['text'])}
    for path in [OUT_DIR / INDEX_NAME, *(OUT_DIR / manifest_name(spec) for spec in ADMITTED),
                 ROOT / 'imports/crystalserver/bindings/items.json', ROOT / 'content/items/aliases.json',
                 ROOT / 'content/world/definitions/reference.json',
                 ROOT / 'docs/architecture/reviews/OTERYN_GAME_A12_ITEM_IDENTITY_TIBIA_ID_DECISION_2026-09-29.md',
                 ROOT / 'docs/architecture/OTERYN_G4_MULTI_SOURCE_IDENTITY_BINDING_DECISION.md',
                 ROOT / 'tools/content-schema/monster-authoring/build_formal_schema.py',
                 Path(sys.modules['appearance_membership'].__file__), Path(sys.modules['engine_items'].__file__),
                 Path(sys.modules['client_map_reader'].__file__),
                 ROOT / 'apps/game-server/src/content/project/v2/creature.rs', Path(__file__)]:
        evidence[str(path)] = sha(path.read_bytes())
    membership = {label: {str(number): next((row for row in m['entries'] if row[0] == number), None)
                         for number in IDS} for label, m in manifests.items()}
    assert all(rows[str(number)] is None for rows in membership.values() for number in (*IDS[:4], 48296))
    assert all(rows['48267'] is not None for rows in membership.values())
    aliases = json.loads((ROOT / 'content/items/aliases.json').read_text())['entries']
    tombstones = [row for row in aliases if row.get('evidence', {}).get('source_item_id') in IDS[:4]]
    assert len(tombstones) == 4 and all(row['state'] == 'RETIRED_WITHOUT_SUCCESSOR' for row in tombstones)
    bindings = [row for row in json.loads((ROOT / 'imports/crystalserver/bindings/items.json').read_text())['bindings']
                if row['external_id'] in set(map(str, IDS))]
    record = next(row for row in json.loads((ROOT / 'content/world/definitions/reference.json').read_text())['records']
                  if row['identity']['key'] == 'oteryn:item.tibia.i48267')
    assert [row['external_id'] for row in bindings] == ['48267']
    return {'scope': 'Read-only exact donor pipeline and accepted membership/identity/authoring boundary; no full engine runtime execution.',
            'sources': sources, 'input_sha256': evidence, 'appearance_membership': membership,
            'spyrat': {'raw_source_object_ids': list(IDS[:4]), 'wire_translation': 'DIRECT_OBJECT_ID_NO_ITEM_CLIENT_REMAP',
                       'existing_registry_tombstones': tombstones,
                       'accepted_normalization_candidate': None,
                       'boundary': 'No admitted object or accepted alias. Replacement object/outfit requires exact accepted identity/presentation evidence; D149 forbids resurrection by number.'},
            'candy_horror': {'selected_bundle_donor': 'canary', 'other_donor_corpse_48268_is_not_an_alias': True,
                            'valid_corpse_source_item_id': 48267, 'missing_decay_target_source_item_id': 48296,
                            'exact_existing_bindings': bindings, 'reference_item_48267': record,
                            'effective_donor_decay': '5s timer ends; missing target makes transform return original corpse, with decaying state false.',
                            'boundary': 'No Item48296 may be minted from XML decayTo alone. Monster temporal none forbids duration_ms, so retaining the source no-op timer requires accepted projection; omission/removal/replacement cannot be inferred.'},
            'runtime_qualified': False, 'product_data_adopted': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sources', type=Path, default=Path('/workspace/monster-reference-sources'))
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    report = verify(args.sources)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'donor_pipelines_checked': len(report['sources']), 'compiled_witnesses': 2,
                      'missing_spyrat_ids': report['spyrat']['raw_source_object_ids'],
                      'valid_corpse_item': 48267, 'unbound_decay_target': 48296, 'product_data_adopted': False}))


if __name__ == '__main__':
    main()
