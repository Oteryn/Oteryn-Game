"""Source-template extraction, provenance fences and strict item operation shapes."""
import copy
import hashlib
from pathlib import Path
import tempfile
import unittest

from jsonschema import Draft202012Validator
import native_world_items as native


FOOD = '''local foods = {3577,3582,3592,3585,3600,3601,3607}
function spell.onCastSpell(creature, variant)
    if math.random(0, 1) == 1 then
        creature:addItem(foods[math.random(#foods)])
    end
    creature:addItem(foods[math.random(#foods)])
    creature:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
    return true
end
'''
CHAMELEON = '''local condition = Condition(CONDITION_OUTFIT)
condition:setTicks(200000)
function rune.onCastSpell(creature, variant, isHotkey)
    local position, item = variant:getPosition()
    if position.x == CONTAINER_POSITION then
        local container = creature:getContainerById(position.y - 64)
        if container then
            item = container:getItem(position.z)
        else
            item = creature:getSlotItem(position.y)
        end
    else
        item = Tile(position):getTopDownItem()
    end
    if not item or item.itemid == 0 or not isMovable(item.uid) then
        creature:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
        creature:getPosition():sendMagicEffect(CONST_ME_POFF)
        return false
    end
    condition:setOutfit({ lookTypeEx = item.itemid })
    creature:addCondition(condition)
    creature:getPosition():sendMagicEffect(CONST_ME_MAGIC_RED)
    return true
end
'''
DISINTEGRATE = '''local corpseIds = {4240,4241,4242,4243,4246,4247,4248}
local removalLimit = 500
function rune.onCastSpell(creature, variant, isHotkey)
    local position = variant:getPosition()
    local tile = Tile(position)
    if tile then
        local items = tile:getItems()
        if items then
            for i, item in ipairs(items) do
                if item:getType():isMovable() and item:getUniqueId() > 65535 and item:getActionId() == 0 and not table.contains(corpseIds, item:getId()) then
                    item:remove()
                end
                if i == removalLimit then
                    break
                end
            end
        end
    end
    creature:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
    position:sendMagicEffect(CONST_ME_POFF)
    return true
end
'''
DESTROY = '''local fields = {105,2118,2119,2120,2121,2122,2123,2124,2125,2126,2132,2133,2134,2135,21465}
function rune.onCastSpell(creature, variant, isHotkey)
    local inPz = creature:getTile():hasFlag(TILESTATE_PROTECTIONZONE)
    if inPz then
        creature:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
        creature:getPosition():sendMagicEffect(CONST_ME_POFF)
        return false
    end
    local position = Variant.getPosition(variant)
    local tile = Tile(position)
    local field = tile and tile:getItemByType(ITEM_TYPE_MAGICFIELD)
    if field and table.contains(fields, field:getId()) then
        field:remove()
        position:sendMagicEffect(CONST_ME_POFF)
        return true
    end
    creature:sendCancelMessage(RETURNVALUE_NOTPOSSIBLE)
    creature:getPosition():sendMagicEffect(CONST_ME_POFF)
    return false
end
'''


def barrier_source(wild=False, source='canary'):
    variable = 'wildGrowth' if wild else 'magicWall'
    function = 'onCreateWildGrowth' if wild else 'onCreateMagicWall'
    symbol = 'ITEM_WILDGROWTH' if wild else 'ITEM_MAGICWALL'
    duration = '30' if wild else '16, 24'
    selection = (f'local {variable}\n    if Game.getWorldType() == WORLD_TYPE_NO_PVP then\n        {variable} = {symbol}_SAFE\n    else\n        {variable} = {symbol}\n    end' if source == 'canary' else
                 f'local {variable} = Game.getWorldType() == WORLDTYPE_OPTIONAL and {symbol}_SAFE or {symbol}')
    return f'''function {function}(creature, position)
    local tile = Tile(position)
    if not tile then
        return false
    end
    if tile:hasFlag(TILESTATE_FLOORCHANGE) then
        return false
    end
    if tile:getTopCreature() and not tile:getTopCreature():isPlayer() then
        return false
    end
    {selection}
    local item = Game.createItem({variable}, 1, position)
    if item then
        item:setDuration({duration})
        item:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, string.format("Casted by: %s", creature:getName()))
    end
end
combat:setParameter(COMBAT_PARAM_DISTANCEEFFECT, CONST_ANI_ENERGY)
combat:setCallback(CALLBACK_PARAM_TARGETTILE, "{function}")
function rune.onCastSpell(creature, variant, isHotkey)
    return combat:execute(creature, variant)
end
'''


def packet(name, texts):
    kind, path = native.PATHS[name]
    records, captured = {}, {}
    for source, text in texts.items():
        raw = text.encode()
        records[source] = {'name': name, 'spell_type': kind, 'file': path,
                           'blob': hashlib.sha1(f'blob {len(raw)}\0'.encode() + raw).hexdigest()}
        captured[source, path] = text
    return records, captured


class NativeWorldItemTests(unittest.TestCase):
    def build(self, name, text, source='canary'):
        records, texts = packet(name, {source: text})
        return native.build(name, native.PATHS[name][0], records, texts)

    def test_food_preserves_pool_order_independent_draw_and_overflow(self):
        behavior = self.build('food', FOOD)
        p = behavior['parameters']
        self.assertEqual([r['key'] for r in p['pool']], [f'candidate:item/{i}' for i in (3577,3582,3592,3585,3600,3601,3607)])
        self.assertEqual((p['guaranteed'], p['extra'], p['extra_chance_percent']), (1,1,50))
        self.assertEqual(p['selection'], 'uniform_independent')
        self.assertEqual(p['overflow'], 'drop_on_caster_tile')
        self.assertTrue(p['always_succeeds'])

    def test_food_refuses_changed_chance_coupled_draw_or_failed_cast(self):
        for text in (FOOD.replace('math.random(0, 1)', 'math.random(0, 3)'),
                     FOOD.replace('math.random(#foods)', '1', 1), FOOD.replace('return true', 'return false')):
            with self.subTest(text=text), self.assertRaises(native.NativeItemUnresolved):
                self.build('food', text)

    def test_food_rejects_symbolic_zero_duplicate_and_wrong_length_pool(self):
        for text in (FOOD.replace('3577,', '0,'), FOOD.replace('3577,', 'ITEM_MEAT,'),
                     FOOD.replace('3577,', '3582,'), FOOD.replace('3577,', '')):
            with self.subTest(text=text), self.assertRaises(native.NativeItemUnresolved):
                self.build('food', text)

    def test_chameleon_has_all_three_targets_and_extracts_literal_duration(self):
        for duration in (200000, 60000):
            p = self.build('chameleon rune', CHAMELEON.replace('200000', str(duration)))['parameters']
            self.assertEqual(p['duration_ms'], duration)
            self.assertEqual(p['source'], ['tile_top_item','container_slot','equipment_slot'])
            self.assertTrue(p['require_movable'])
            self.assertTrue(p['reject_creature_target'])

    def test_chameleon_missing_duration_or_changed_movable_guard_is_rejected(self):
        for text in (CHAMELEON.replace('200000', 'duration'), CHAMELEON.replace('isMovable', 'isPickupable')):
            with self.subTest(text=text), self.assertRaises(native.NativeItemUnresolved):
                self.build('chameleon rune', text)

    def test_disintegrate_counts_visited_preserves_protection_and_corrects_cancel_defect(self):
        p = self.build('desintegrate rune', DISINTEGRATE)['parameters']
        self.assertEqual((p['max_items'], p['max_items_counts']), (500, 'visited'))
        self.assertEqual([r['key'] for r in p['exclude_items']], [f'candidate:item/{i}' for i in (4240,4241,4242,4243,4246,4247,4248)])
        for key in ('exclude_script_tagged','exclude_action_tagged','allow_in_pz','empty_tile_succeeds'):
            self.assertTrue(p[key])
        self.assertFalse(p['aggressive'])
        self.assertFalse(p['send_cancel_on_success'])

    def test_disintegrate_never_drops_a_source_item_protection_guard(self):
        for text in (DISINTEGRATE.replace('item:getActionId() == 0', 'true'),
                     DISINTEGRATE.replace('item:getUniqueId() > 65535', 'true'),
                     DISINTEGRATE.replace('local removalLimit = 500', 'local removalLimit = 0')):
            with self.subTest(text=text), self.assertRaises(native.NativeItemUnresolved):
                self.build('desintegrate rune', text)

    def test_destroy_field_respects_first_magic_field_and_pz_failure(self):
        p = self.build('destroy field rune', DESTROY)['parameters']
        self.assertEqual(p['source'], ['first_magic_field'])
        self.assertFalse(p['allow_in_pz'])
        self.assertEqual(p['failure_effect_position'], 'caster')
        self.assertEqual(p['failure_message'], 'not_possible')

    def test_destroy_field_disagreement_uses_s21_and_preserves_both_observations(self):
        crystal = DESTROY.replace('2132,2133,2134,2135,21465', '2131,2132,2133,2134,2135')
        records, texts = packet('destroy field rune', {'canary': DESTROY, 'crystal': crystal})
        selected = native.build('destroy field rune','rune',records,texts)
        ids = [r['key'] for r in selected['parameters']['field_items']]
        self.assertIn('candidate:item/21465', ids)
        self.assertNotIn('candidate:item/2131', ids)
        rows = native.evidence('destroy field rune','rune',records,texts)
        self.assertEqual([r['source'] for r in rows if 'source' in r], ['canary','crystal'])
        self.assertTrue(any(r.get('policy') == 'S21' for r in rows))

    def test_barrier_variants_description_and_accepted_duration_ranges(self):
        for name, wild, normal, safe, duration in [('magic wall rune',False,2128,10181,(16000,24000)),
                                                   ('wild growth rune',True,2130,10182,(30000,60000))]:
            with self.subTest(name=name):
                records, texts = packet(name,{s:barrier_source(wild,s) for s in ('canary','crystal')})
                self.assertIsNone(native.build(name,'rune',records,texts))
                p = native.barrier_effect(name,records,texts)
                self.assertEqual(p['created_item']['key'], f'candidate:item/{normal}')
                self.assertEqual(p['pvp_safe_item']['key'], f'candidate:item/{safe}')
                self.assertEqual((p['duration_range_ms']['minimum'],p['duration_range_ms']['maximum']),duration)
                self.assertEqual(p['description_template'],'Casted by: {caster_name}')
                self.assertEqual(p['refuse_on'],['floor_change_tile','creature_on_tile'])
                self.assertEqual(p['presentation']['projectile_asset_binding'],'canary.appearance:missile/energy')

    def test_barrier_rejects_changed_world_branch_missing_guard_and_reversed_range(self):
        original = barrier_source()
        for text in (original.replace('WORLD_TYPE_NO_PVP','WORLD_TYPE_PVP'),
                     original.replace('TILESTATE_FLOORCHANGE','TILESTATE_PROTECTIONZONE'),
                     original.replace('16, 24','24, 16'), original.replace('CONST_ANI_ENERGY','CONST_ANI_FIRE')):
            records,texts=packet('magic wall rune',{'canary':text})
            with self.subTest(text=text), self.assertRaises(native.NativeItemUnresolved):
                native.barrier_effect('magic wall rune',records,texts)

    def test_wild_growth_evidence_retains_fixed_engine_duration_and_wiki_override(self):
        records,texts=packet('wild growth rune',{'canary':barrier_source(True)})
        rows=native.evidence('wild growth rune','rune',records,texts)
        self.assertEqual(rows[0]['extracted_parameters']['duration_range_ms'], {'minimum':30000,'maximum':30000})
        self.assertTrue(any(r.get('policy') == 'S27 D.6.1 / F1056364' for r in rows))

    def test_digest_identity_revision_and_missing_text_fail_closed(self):
        for mutation in ('digest','identity','revision','text'):
            records,texts=packet('food',{'canary':FOOD})
            if mutation=='digest':records['canary']['blob']='0'*40
            elif mutation=='identity':records['canary']['file']='other.lua'
            elif mutation=='revision':records['canary']['revision']='main'
            else:texts.clear()
            with self.subTest(mutation=mutation), self.assertRaises(native.NativeItemUnresolved):
                native.build('food','instant',records,texts)

    def test_barrier_source_root_header_hash_is_fenced(self):
        records,texts=packet('magic wall rune',{'canary':barrier_source()})
        with tempfile.TemporaryDirectory() as root:
            header=Path(root)/'src/utils/utils_definitions.hpp'
            header.parent.mkdir(parents=True)
            header.write_text('enum ItemID_t { ITEM_MAGICWALL = 0 };')
            records['canary']['source_root']=root
            with self.assertRaisesRegex(native.NativeItemUnresolved,'ItemID_t header'):
                native.barrier_effect('magic wall rune',records,texts)

    def test_unrecognized_name_or_wrong_carrier_returns_none(self):
        self.assertIsNone(native.build('food','rune',{},{}))
        self.assertIsNone(native.build('another spell','instant',{},{}))

    def test_native_schemas_reject_unknown_and_missing_fields_and_wrong_reference_family(self):
        for name,text in [('food',FOOD),('chameleon rune',CHAMELEON),('desintegrate rune',DISINTEGRATE),('destroy field rune',DESTROY)]:
            behavior=self.build(name,text)
            validator=Draft202012Validator(native.schemas()[behavior['key']])
            p=behavior['parameters']
            self.assertEqual(list(validator.iter_errors(p)),[])
            for key in p:
                changed=copy.deepcopy(p);del changed[key]
                self.assertTrue(list(validator.iter_errors(changed)),(name,key))
            changed=copy.deepcopy(p);changed['unmodeled']=1
            self.assertTrue(list(validator.iter_errors(changed)))
        p=self.build('food',FOOD)['parameters'];p['pool'][0]['family']='Creature'
        self.assertTrue(list(Draft202012Validator(native.schemas()['random_item_grant']).iter_errors(p)))


if __name__ == '__main__':
    unittest.main()
