#!/usr/bin/env python3
"""Read-only pinned source census for the positive Cloak health-event branch."""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path
import re
import xml.etree.ElementTree as ET

PIN = '201189f871d23597b82caf88fd611454a2ddefb8'
CPP = {
 'src/game/game.cpp': ['7212-7233', '7401', '7803-7844', '7933-7948'],
 'src/lua/scripts/scripts.cpp': ['127-137'],
 'src/lua/functions/creatures/combat/variant_functions.cpp': ['54-62'],
 'src/creatures/monsters/monster.cpp': ['1349-1371'],
 'src/creatures/creatures_definitions.hpp': ['1732-1738'],
 'src/creatures/combat/combat.cpp': ['197-271', '754-802', '944', '1638-1670', '1673-1716'],
 'src/creatures/creature.cpp': ['875-940'],
 'src/creatures/players/player.cpp': ['12899-12926', '13009'],
 'src/creatures/players/wheel/player_wheel.cpp': ['3165-3173', '4061-4066'],
 'src/creatures/combat/spells.cpp': ['1155-1189', '1490-1525'],
 'src/creatures/combat/condition.cpp': ['1337-1357', '1638-1670', '1712-1767', '1839-1876'],
 'src/items/functions/item/item_parse.cpp': ['656-689'],
 'src/lua/functions/creatures/creature_functions.cpp': ['591-608'],
 'src/lua/functions/core/game/global_functions.cpp': ['355-397'],
 'src/lua/callbacks/event_callback.cpp': ['243-282'],
}
CRITICAL = {
 'data-global/scripts/quests/soul_war/soul_war_mechanics.lua': ['807-824'],
 'data-global/monster/quests/soul_war/normal_monsters/furious_crater/cloak_of_terror.lua': ['29-32', '53-62'],
 'data/events/scripts/player.lua': ['670-685'],
 'data-global/scripts/quests/soul_war/eventcallback_on_combat_taint.lua': ['73-92'],
 'data-global/scripts/quests/rotten_blood_quest/eventcallback_on_combat_rottenblood.lua': ['250-268'],
 'data/scripts/spells/party/heal_party.lua': ['18-58'],
 'data-global/scripts/spells/monster/ratmiral_ball.lua': ['1-62'],
}
HEALTH = re.compile(r'COMBAT_HEALING|[aA]ddHealth\(|CONDITION_PARAM_HEALTHGAIN|[sS]etHealth\(')
# Enumerate call sites across every runtime root, including variable-typed
# combats and positive formula inputs that have no literal HEALING token.
COMBAT_POTENTIAL = re.compile(
    r'COMBAT_HEALING|\bCombat\s*\(|\bdo\w*(?:Combat|Health)\w*\s*\('
    r'|\bset(?:Combat|Condition)\w*\s*\(|:\s*(?:setFormula|setParameter|addDamage|addDamageCondition'
    r'|addHealth|changeHealth|setHealth|castSpell|doCombat|execute)\s*\('
    r'|function\s+[^\n]*(?:onHealthChange|[cC]reatureOnCombat|[pP]layerOnCombat)'
)
REGISTRATIONS = re.compile(r'\b(?:Spell|Action|MoveEvent|GlobalEvent|CreatureEvent|EventCallback|TalkAction|Game\.createNpcType|Game\.createMonsterType)\s*\(')


def split_lua_arguments(text):
    """Split only top-level commas; nested RNG calls remain one argument."""
    result, start, depth, quote, escaped = [], 0, 0, None, False
    for index, char in enumerate(text):
        if quote:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == quote:
                quote = None
        elif char in ("'", '"'):
            quote = char
        elif char in '({[':
            depth += 1
        elif char in ')}]':
            depth -= 1
        elif char == ',' and depth == 0:
            result.append(text[start:index].strip())
            start = index + 1
    result.append(text[start:].strip())
    return result



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-dir', type=Path, required=True)
    parser.add_argument('--tree', type=Path, required=True)
    parser.add_argument('--items-xml', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    parser.add_argument('--canary-source-dir', type=Path)
    parser.add_argument('--canary-tree', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    tree = json.loads(args.tree.read_bytes())
    if tree['sha'] != PIN or tree.get('truncated'):
        raise ValueError('exact complete pinned source tree required')
    rows = {row['path']: row for row in tree['tree'] if row['type'] == 'blob'}
    records = {}
    census = []
    candidate_index = []
    condition_calls = []
    periodic_setters = []
    variable_combat_types = []
    variable_parameter_keys = []
    direct_health_calls = []
    legacy_condition_parameter_calls = []
    def verify(path, raw=None):
        raw = (args.source_dir/path).read_bytes() if raw is None else raw
        blob = hashlib.sha1(f'blob {len(raw)}\0'.encode()+raw).hexdigest()
        if blob != rows[path]['sha']:
            raise ValueError(f'pinned source mismatch: {path}')
        return {'path': path, 'git_blob': blob, 'sha256': hashlib.sha256(raw).hexdigest()}
    lua = sorted(path for path in rows if path.endswith('.lua') and path.startswith(('data/', 'data-global/', 'data-crystal/')))
    for path in lua:
        facts = verify(path)
        census.append(f"{path}\0{facts['git_blob']}\n")
        text = (args.source_dir/path).read_text()
        # Preserve balanced call expressions, including multiline third arguments;
        # a lexical minus is DATA, never an automatic semantic sign proof.
        for match in re.finditer(r':\s*addDamage\s*\(', text):
            start = match.end()
            depth, quote, escaped = 1, None, False
            end = start
            for end in range(start, len(text)):
                char = text[end]
                if quote:
                    if escaped:
                        escaped = False
                    elif char == "\\":
                        escaped = True
                    elif char == quote:
                        quote = None
                elif char in ("'", '"'):
                    quote = char
                elif char == '(':
                    depth += 1
                elif char == ')':
                    depth -= 1
                    if depth == 0:
                        break
            if depth:
                raise ValueError(f'unbalanced addDamage call: {path}')
            condition_calls.append({'path': path,
                'line': text.count('\n', 0, match.start()) + 1,
                'arguments': text[start:end].strip(),
                'signed_damage_expression': split_lua_arguments(text[start:end])[-1]})
        for number, line in enumerate(text.splitlines(), 1):
            if re.search(r'\bsetConditionParam\s*\(', line):
                legacy_condition_parameter_calls.append({'path': path, 'line': number, 'text': line.strip()})
                if 'CONDITION_PARAM_TICKS' not in line:
                    raise ValueError('new legacy condition parameter needs sign review')
        for match in re.finditer(r'\b(doTargetCombatHealth|doAreaCombatHealth)\s*\(', text):
            start, depth, quote, escaped = match.end(), 1, None, False
            for end in range(start, len(text)):
                char = text[end]
                if quote:
                    if escaped:
                        escaped = False
                    elif char == "\\":
                        escaped = True
                    elif char == quote:
                        quote = None
                elif char in ("'", '"'):
                    quote = char
                elif char == '(':
                    depth += 1
                elif char == ')':
                    depth -= 1
                    if depth == 0:
                        break
            if depth:
                raise ValueError('unbalanced direct health call')
            arguments = split_lua_arguments(text[start:end])
            type_index = 2 if match[1] == 'doTargetCombatHealth' else 1
            direct_health_calls.append({'path': path, 'line': text.count('\n', 0, match.start()) + 1,
                'function': match[1], 'arguments': arguments,
                'combat_type': arguments[type_index] if len(arguments) > type_index else None})
        for number, line in enumerate(text.splitlines(), 1):
            setter = re.search(r':setParameter\s*\(\s*([^,]+),', line)
            if setter and not re.fullmatch(r'[A-Z][A-Z0-9_]*', setter[1].strip()):
                variable_parameter_keys.append({'path': path, 'line': number, 'text': line.strip()})
            if 'CONDITION_PARAM_PERIODICDAMAGE' in line:
                periodic_setters.append({'path': path, 'line': number, 'text': line.strip()})
            if re.search(r'setParameter\s*\(\s*COMBAT_PARAM_TYPE\s*,', line) and not re.search(r',\s*COMBAT_[A-Z_]+\s*\)', line):
                variable_combat_types.append({'path': path, 'line': number, 'text': line.strip()})
        potential = [{'line': n, 'text': line.strip()} for n, line in enumerate(text.splitlines(), 1)
                     if COMBAT_POTENTIAL.search(line)]
        if potential:
            entry = dict(facts)
            entry['disabled_by_source_loader'] = Path(path).name.startswith('#')
            entry['registration_sites'] = [{'line': n, 'text': line.strip()}
                for n, line in enumerate(text.splitlines(), 1) if REGISTRATIONS.search(line)]
            entry['combat_potential_sites'] = potential
            entry['literal_healing_sites'] = [hit for hit in potential if 'COMBAT_HEALING' in hit['text']]
            entry['review_class'] = ('disabled-filename' if entry['disabled_by_source_loader'] else
                'monster-definition-with-runtime-callbacks' if '/monster/' in path and re.search(r'(?:onThink|onSpawn|onHealthChange|onDeath)\s*=',text) else
                'monster-definition' if '/monster/' in path else
                'runtime-entry-or-helper-requires-dispatch-review')
            candidate_index.append(entry)
        if path.startswith(('data/scripts/spells/', 'data/scripts/runes/', 'data/scripts/actions/')) and HEALTH.search(text):
            hits = [{'line': line, 'text': value.strip()} for line, value in enumerate(text.splitlines(), 1)
                    if HEALTH.search(value) or 'isSelfTarget(true)' in value or 'hasPlayerNameParam(true)' in value]
            facts['health_and_target_facts'] = hits
            facts['disabled_by_source_loader'] = Path(path).name.startswith('#')
            records[path] = facts
    for path, lines in (CPP | CRITICAL).items():
        facts = records.get(path, verify(path))
        facts['reviewed_lines'] = lines
        records[path] = facts
    for vocation in ('druid', 'knight', 'monk', 'paladin', 'sorcerer'):
        path = f'data-global/monster/familiars/{vocation}_familiar.lua'
        facts = verify(path)
        facts['healing_lines'] = [n for n, line in enumerate((args.source_dir/path).read_text().splitlines(), 1) if 'COMBAT_HEALING' in line]
        records[path] = facts
    # Includes every non-player helper that explicitly constructs a healing
    # combat, rather than assuming all monster spells are player-inaccessible.
    for path in sorted(rows):
        if path.startswith('data-global/scripts/spells/monster/') and path.endswith('.lua'):
            text = (args.source_dir/path).read_text()
            if 'COMBAT_HEALING' in text:
                facts = verify(path)
                facts['healing_lines'] = [{'line': n, 'text': line.strip()} for n, line in enumerate(text.splitlines(), 1)
                                          if 'COMBAT_HEALING' in line or 'doTargetCombatHealth' in line or 'needLearn' in line]
                records[path] = facts
    condition_producers = []
    helper_references = []
    for path in lua:
        text = (args.source_dir/path).read_text()
        for number, line in enumerate(text.splitlines(), 1):
            if 'addDamageCondition' in line:
                helper_references.append({'path': path, 'line': number, 'text': line.strip()})
    for path in sorted({row['path'] for row in condition_calls}):
        calls = [row for row in condition_calls if row['path'] == path]
        expressions = sorted({row['signed_damage_expression'] for row in calls})
        nonliteral = [expression for expression in expressions if not re.fullmatch(r'-\d+', expression)]
        if not nonliteral:
            classification = 'All literal damage arguments are strictly negative.'
        elif path in ('data/libs/functions/creature.lua', 'data/scripts/lib/register_spells.lua'):
            classification = 'Generic helper definition; full verified Lua corpus contains only two definitions plus an uncalled compatibility alias and no addDamageCondition caller. Core helper rejects damage<=0; register override is not assumed safe for arbitrary hypothetical inputs.'
        elif path.endswith('/mitmah_soulfire.lua'):
            classification = 'Both bounds are negative (-3000,-7000); reversed math.random interval fails before addDamage. No positive-condition repair is inferred.'
        elif path.endswith(('/humongous_fungus_poison.lua', '/hideous_fungus_poison.lua')):
            classification = 'Loop i is25..30; each actual argument -i through -i+4 is strictly negative.'
        elif path.endswith('_curse.lua'):
            classification = 'Reviewed positive initial loop/base damage and positive multiplicative growth; negation/floor-negation yields nonpositive condition damage.'
        else:
            raise ValueError(f'unreviewed nonliteral condition producer: {path}')
        lines = (args.source_dir/path).read_text().splitlines()
        condition_producers.append({'path': path, 'classification': classification,
            'expressions': expressions,
            'bound_and_assignment_evidence': [{'line': number, 'text': line.strip()}
                for number, line in enumerate(lines, 1)
                if re.search(r'for i|(?:local )?(?:damage|value|minDmg|maxDmg|randomDamage)\s*=|damage\s*<=|function Creature:addDamageCondition|while damage', line)]})
    if len(helper_references) != 3 or any(not (row['text'].startswith('function Creature:addDamageCondition') or row['text'] == 'addDamageCondition = Condition.addDamage') for row in helper_references):
        raise ValueError('new generic condition helper caller requires reachability review')
    variable_type_bindings = []
    for path in sorted({row['path'] for row in variable_combat_types}):
        text = (args.source_dir/path).read_text()
        if not path.startswith('data/scripts/spells/attack/') or 'COMBAT_HEALING' in text:
            raise ValueError(f'new variable combat type needs review: {path}')
        variable_type_bindings.append({'path': path,
            'review': 'All actual element selections/calls are FIRE, ENERGY or DEATH; all use Combat execution and CreatureOnCombat type normalization.',
            'binding_lines': [{'line': number, 'text': line.strip()}
                for number, line in enumerate(text.splitlines(), 1)
                if re.search(r'COMBAT_[A-Z]+DAMAGE|combatType\s*=|elementForStance|create\w*Combat\(', line)]})
    if variable_parameter_keys != [{'path': 'data/libs/systems/concoctions.lua', 'line': 57,
        'text': 'self.condition:setParameter(self.config.condition[1], self.config.condition[2])'}]:
        raise ValueError('new numeric/aliased parameter-key site needs source review')
    for call in direct_health_calls:
        if call['combat_type'] != 'COMBAT_HEALING':
            call['eligibility_review'] = 'Nonhealing Combat type passes CreatureOnCombat damage sign normalization; no positive secondary is constructed by this helper.'
        elif call['arguments'][0] == '0':
            call['eligibility_review'] = 'Nonnil attacker predicate fails: explicit nil caster0.'
        elif call['path'] == 'data/scripts/spells/healing/heal_malice.lua':
            call['eligibility_review'] = 'Actual Monster target is itself the caster and target-name guard selects Malice; secondary0, not Player.'
        elif call['path'] == 'data/scripts/spells/healing/mass_healing.lua':
            call['eligibility_review'] = 'Target selection rejects masterless monster except closed named bosses; Cloak is absent.'
        elif call['path'] == 'data/scripts/actions/items/potions.lua':
            call['eligibility_review'] = 'Actual onUse target guard lines69-72 rejects nonPlayer userdata before healing.'
        else:
            raise ValueError('new nonnil direct healing caller needs target review')
    xml = verify('data/items/items.xml', args.items_xml.read_bytes())
    xml['healing_weapon_combat_type_overrides'] = len(re.findall(r'value="healing"', args.items_xml.read_text()))
    records[xml['path']] = xml
    field_damage = []
    for item in ET.fromstring(args.items_xml.read_bytes()):
        for field in item.findall("attribute[@key='field']"):
            for attribute in field.iter('attribute'):
                if attribute.get('key') == 'damage':
                    field_damage.append({'item_id_or_from': item.get('id', item.get('fromid')),
                        'raw_xml_damage': int(attribute.attrib['value'])})
    if any(row['raw_xml_damage'] < 0 for row in field_damage):
        raise ValueError('positive condition from negative XML damage needs source review')
    xml['field_damage_inputs'] = field_damage
    xml['field_damage_sign_fact'] = 'All nested field damage values are nonnegative; item_parse.cpp negates this input before ConditionDamage.'
    monster_healing_sections = []
    for entry in candidate_index:
        if '/monster/' not in entry['path'] or not entry['literal_healing_sites']:
            continue
        text = (args.source_dir/entry['path']).read_text()
        for hit in entry['literal_healing_sites']:
            preceding = '\n'.join(text.splitlines()[:hit['line']])
            sections = list(re.finditer(r'monster\.(\w+)\s*=', preceding))
            if sections:
                section = sections[-1][1]
                monster_healing_sections.append({'path': entry['path'], 'line': hit['line'], 'section': section,
                    'review': 'Monster defense casts Monster-to-itself; original source is not replaced by master Player.' if section == 'defenses' else 'Non-producing element resistance entry (COMBAT_HEALINGDAMAGE source typo), not an attack.'})
                if section not in ('defenses', 'elements'):
                    raise ValueError('new producing monster healing section needs review')
    result = {
        'schema': 'SRX3_CLOAK_POSITIVE_HEALTH_CALLER_CLOSURE/v1',
        'source_repository': 'zimbadev/crystalserver', 'source_revision': PIN,
        'lua_census': {'verified_files': len(lua), 'missing': 0, 'blob_mismatches': 0,
                       'path_blob_census_sha256': hashlib.sha256(''.join(census).encode()).hexdigest()},
        'facts': [
          'Cloak requires a nonnil attacker, then (actual Player and positive primary) or positive secondary.',
          'Ordinary damage reaches the health event with negative components; direct Creature:addHealth has nil attacker and origin NONE.',
          'Healing-from-damage calls combatChangeHealth with nil attacker; ordinary healing secondary defaults to zero.',
          'Player self heals and player-name heals cannot select a wild Cloak; area heals whitelist other named monsters.',
          'All five familiar defenses cast actual Monster-to-itself healing; no master Player replaces the source at the health event.',
          'Monster area-healing helper calls use nil caster; Heal Malice selects Malice only; Ratmiral has no positive combat formula and its explicit addHealth targets other named monsters.',
          'The two registered creatureOnCombat callbacks preserve combat types; Player:onCombat returns both components unchanged.',
          'The broad test healing rune exists only in #example.lua; the source loader skips filenames starting with # (scripts.cpp127-137).',
        ],
        'sources': [records[path] for path in sorted(records)],
        'condition_sign_audit': {
            'balanced_add_damage_calls': condition_calls,
            'producer_sign_review': condition_producers,
            'generic_helper_references': helper_references,
            'periodic_damage_setters': periodic_setters,
            'variable_combat_type_setters': variable_combat_types,
            'variable_type_binding_review': variable_type_bindings,
            'numeric_or_aliased_parameter_keys': variable_parameter_keys,
            'legacy_condition_parameter_calls': legacy_condition_parameter_calls,
            'legacy_parameter_review': 'All six setConditionParam calls set TICKS only; no legacy damage/periodic/numeric parameter path is present.',
            'parameter_key_review': 'The sole variable parameter key is Concoction CONDITION_ATTRIBUTES constructor (lines54-57), not a ConditionDamage or Combat constructor. No numeric combat parameter-key site exists in the complete runtime Lua census.',
            'interpretation': 'Raw signed expressions retained for semantic producer review. Unary minus alone does not prove negative values. Periodic and addDamage values preserve sign in CPP; generated MIN/MAX condition damage uses negative absolute values.',
            'xml_field_damage_count': len(field_damage),
            'xml_negative_damage_count': 0,
        },
        'direct_health_combat_call_index': direct_health_calls,
        'monster_literal_healing_section_review': monster_healing_sections,
        'all_runtime_combat_candidate_index': candidate_index,
        'candidate_index_scope': 'All verified Lua files in data, data-global and data-crystal, including NPCs/custom actions/monster callbacks and numeric or aliased combat setters; only actual filename-disabled entries are marked disabled. This index is not an automatic reachability verdict.',
        'conclusion': 'Complete combat-potential candidate index now includes every runtime Lua root. Source-inert classification remains under review; a positive canonical owner unit alone does not establish an admitted player action or MINT caller.',
        'wiki_status': 'wiki-pending', 'NEED-FETCH': [],
    }
    if args.canary_source_dir is not None:
        if args.canary_tree is None:
            raise ValueError('Canary comparator tree required')
        canary_pin = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
        canary = json.loads(args.canary_tree.read_bytes())
        if canary['sha'] != canary_pin or canary.get('truncated'):
            raise ValueError('exact Canary tree required')
        reference = {r['path']: r for r in canary['tree'] if r['type'] == 'blob'}
        comparisons = []
        for path in ('data/scripts/runes/intense_healing_rune.lua',
                     'data/scripts/runes/ultimate_healing_rune.lua',
                     'data/scripts/spells/healing/mass_healing.lua',
                     'src/lua/functions/creatures/combat/variant_functions.cpp'):
            raw = (args.canary_source_dir/path).read_bytes()
            blob = hashlib.sha1(f'blob {len(raw)}\0'.encode()+raw).hexdigest()
            if blob != reference[path]['sha']:
                raise ValueError('Canary source comparator mismatch')
            comparisons.append({'path': path, 'git_blob': blob,
                'sha256': hashlib.sha256(raw).hexdigest(),
                'reviewed_lines': [{'line': n, 'text': line.strip()}
                    for n, line in enumerate(raw.decode().splitlines(), 1)
                    if ('Monster(var:getNumber' in line or 'needTarget(true)' in line
                        or 'target:isMonster()' in line or 'local bosses' in line
                        or 'luaVariantGetNumber' in line or 'variant.number' in line)]})
        result['canary_corroboration'] = {'source_revision': canary_pin,
            'scope': 'four source comparators; the complete Lua census above is Crystal',
            'facts': 'Both runes reject the actual numeric Monster target; Variant:getNumber ignores the superfluous argument and reads self. Mass Healing excludes a masterless Cloak.',
            'sources': comparisons}
    serialized = json.dumps(result, indent=2)+'\n'
    if args.check:
        if args.out.read_text() != serialized:
            raise ValueError('Cloak caller-closure evidence is stale')
    else:
        args.out.write_text(serialized)
    print(f'verified {len(lua)} Lua blobs; {len(records)} reviewed source files; {args.out}')


if __name__ == '__main__':
    main()
