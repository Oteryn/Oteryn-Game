"""Compare TibiaWiki creature ability scenes with the Canary conversion at the 2026-09-27 cut.

Evidence tooling only; nothing is adopted. A wiki `{{Ability|...|scene={{Scene|spell=SHAPE|effect=..|missile=..}}}}`
names a shape of `Module:SceneBuilder/data` (a grid: 2 caster, 3 target, 1 hit tile) and effect/missile
pages whose infobox gives the client id. The Canary side is the plain conversion: an Ability's area is
rebuilt with the engine's own AreaCombat rules (src/creatures/combat/combat.cpp setupArea: radius table,
length/spread cone, matrix), a directional area is anchored on the caster (the matrix centre is the tile in
front of it) and a targeted area on its target. Both hit sets are compared relative to their anchor,
without the anchor tile, under the four rotations (the wiki draws a scene in one facing).

Usage: python wiki_scenes.py --canary <Canary checkout> [--cache DIR]
"""
import argparse
import json
import re
from pathlib import Path

import canary_batch as cb
import wiki_compare as wc

ROOT = Path(__file__).resolve().parent
SCENE_DATA = 'Module:SceneBuilder/data'
OUT = ROOT / 'samples' / 'wiki-scenes-2026-09-27.json'
ELEMENTS = {'physical': 'physical', 'fire': 'fire', 'earth': 'earth', 'poison': 'earth', 'energy': 'energy', 'ice': 'ice',
            'holy': 'holy', 'death': 'death', 'life drain': 'life_drain', 'lifedrain': 'life_drain', 'mana drain': 'mana_drain',
            'manadrain': 'mana_drain', 'drown': 'drowning', 'drowning': 'drowning', 'healing': 'healing'}
# game.cpp Game::combatGetTypeInfo: the hit effect shown when a combat has no effect of its own. Physical depends on the
# target race; a monster attack hits a player, whose race is blood (CONST_ME_DRAWBLOOD).
DEFAULT_HIT = {'physical': 'drawblood', 'energy': 'energyhit', 'earth': 'green_rings', 'drowning': 'loseenergy', 'fire': 'hitbyfire', 'ice': 'iceattack',
               'holy': 'holydamage', 'death': 'smallclouds', 'life_drain': 'magic_red'}
# combat.cpp AreaCombat::setupArea(int32_t radius): a tile is hit when 0 < cell <= radius; 1 is the centre.
RADIUS_TABLE = [
    [0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 8, 8, 7, 8, 8, 0, 0, 0, 0], [0, 0, 0, 8, 7, 6, 6, 6, 7, 8, 0, 0, 0],
    [0, 0, 8, 7, 6, 5, 5, 5, 6, 7, 8, 0, 0], [0, 8, 7, 6, 5, 4, 4, 4, 5, 6, 7, 8, 0], [0, 8, 6, 5, 4, 3, 2, 3, 4, 5, 6, 8, 0],
    [8, 7, 6, 5, 4, 2, 1, 2, 4, 5, 6, 7, 8], [0, 8, 6, 5, 4, 3, 2, 3, 4, 5, 6, 8, 0], [0, 8, 7, 6, 5, 4, 4, 4, 5, 6, 7, 8, 0],
    [0, 0, 8, 7, 6, 5, 5, 5, 6, 7, 8, 0, 0], [0, 0, 0, 8, 7, 6, 6, 6, 7, 8, 0, 0, 0], [0, 0, 0, 0, 8, 8, 7, 8, 8, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0]]


def scene_shapes(cache):
    record = wc.fetch(SCENE_DATA, cache)['cut']
    shapes = {}
    for name, body, cols in re.findall(r'\[["\']([^"\']+)["\']\]\s*=\s*\{\{([^{}]*)\}\s*,\s*(\d+)\s*\}', record['content']):
        values = [int(v) for v in re.split(r'[,\s]+', body.strip()) if v]
        cols = int(cols)
        shapes[name.lower()] = [values[i:i + cols] for i in range(0, len(values), cols)]
    return record, shapes


def templates(text):
    """Top-level `{{...}}` templates of `text` as (name, positional params, named params)."""
    found, depth, start = [], 0, None
    i = 0
    while i < len(text):
        pair = text[i:i + 2]
        if pair == '{{':
            if depth == 0:
                start = i
            depth += 1
            i += 2
            continue
        if pair == '}}' and depth:
            depth -= 1
            if depth == 0:
                found.append(parse_template(text[start + 2:i]))
            i += 2
            continue
        i += 1
    return found


def parse_template(body):
    parts, depth, buffer = [], 0, ''
    i = 0
    while i < len(body):
        pair = body[i:i + 2]
        if pair in ('{{', '[['):
            depth += 1
            buffer += pair
            i += 2
            continue
        if pair in ('}}', ']]'):
            depth -= 1
            buffer += pair
            i += 2
            continue
        if body[i] == '|' and depth == 0:
            parts.append(buffer)
            buffer = ''
        else:
            buffer += body[i]
        i += 1
    parts.append(buffer)
    name, positional, named = parts[0].strip(), [], {}
    for part in parts[1:]:
        key, sep, value = part.partition('=')
        if sep and re.fullmatch(r'\s*[\w ]+\s*', key) and '{{' not in key:
            named[key.strip().lower()] = value.strip()
        else:
            positional.append(part.strip())
    return name, positional, named


def page_id(name, field, cache, ids):
    """Client id from the `effectid`/`missileid` field of an effect or missile page at the cut."""
    key = (name.strip().lower(), field)
    if key not in ids:
        record = wc.fetch(name.strip(), cache)['cut']
        match = re.search(r'^\|\s*' + field + r'\s*=\s*(\d+)', record['content'], re.M) if record else None
        ids[key] = int(match.group(1)) if match else None
    return ids[key]


def rotations(tiles):
    result, current = [], set(tiles)
    for _ in range(4):
        result.append(frozenset(current))
        current = {(-y, x) for x, y in current}
    return result


def wiki_geometry(grid):
    """(kind, hit tiles relative to the anchor without it) of a SceneBuilder grid, or (None, reason)."""
    cells = {(x, y): v for y, row in enumerate(grid) for x, v in enumerate(row) if v}
    if any(v > 3 for v in cells.values()):
        return None, 'composite scene (values above 3)'
    caster = next((p for p, v in cells.items() if v == 2), None)
    target = next((p for p, v in cells.items() if v == 3), None)
    hits = {p for p, v in cells.items() if v in (1, 3)}
    if caster is None:
        return None, 'no caster tile'
    if not any(v == 1 for v in cells.values()):
        return ('target', frozenset()) if target else ('self', frozenset())

    def symmetric(anchor):
        return all((2 * anchor[0] - x, 2 * anchor[1] - y) in hits | {anchor} for x, y in hits)

    if symmetric(caster):
        kind, anchor = 'area_self', caster
    elif target and symmetric(target):
        kind, anchor = 'area_target', target
    else:
        kind, anchor = 'area_direction', caster
    return kind, frozenset((x - anchor[0], y - anchor[1]) for x, y in hits if (x, y) != anchor)


def matrix_tiles(rows):
    """Hit tiles and centre of a north-facing Canary area matrix (1/3 hit, 2/3 centre)."""
    hits, centre = set(), None
    for y, row in enumerate(rows):
        for x, v in enumerate(row):
            if v in (1, 3):
                hits.add((x, y))
            if v in (2, 3):
                centre = (x, y)
    return hits, centre


def cone(length, spread):
    """combat.cpp AreaCombat::setupArea(length, spread) as a north-facing matrix."""
    rows, cols = length, (((length - (length % spread)) // spread) * 2 + 1 if spread else 1)
    matrix, col_spread = [], cols
    for y in range(1, rows + 1):
        mincol, maxcol = cols - col_spread + 1, cols - (cols - col_spread)
        row = []
        for x in range(1, cols + 1):
            if y == rows and x == (cols - cols % 2) // 2 + 1:
                row.append(3)
            else:
                row.append(1 if mincol <= x <= maxcol else 0)
        matrix.append(row)
        if spread > 0 and y % spread == 0:
            col_spread -= 1
    return matrix


def canary_geometry(ability):
    area = ability.get('area')
    if 'chain' in ability:
        return None, 'chain (targets picked at cast time)'
    if ability.get('kind') == 'melee':
        return ('target' if ability.get('range_tiles', 1) > 1 else 'melee'), frozenset()
    if not area:
        # spells.cpp CombatSpell::castSpell: a combat without area always runs on the target creature, which is the
        # attacked creature for an attack and the caster itself for a defense.
        return ('self' if '/defense-' in ability['identity']['key'] else 'target'), frozenset()
    if 'radius_tiles' in area:
        rows = [[3 if c == 1 else (1 if 0 < c <= area['radius_tiles'] else 0) for c in row] for row in RADIUS_TABLE]
    elif 'length_tiles' in area:
        rows = cone(area['length_tiles'], area.get('spread_tiles', 0))
    elif 'matrix' in area:
        rows = [[{'.': 0, 'x': 1, 'c': 2, 'C': 3}[c] for c in row] for row in area['matrix']['north']]
    else:
        return None, 'unsupported area form'
    hits, centre = matrix_tiles(rows)
    if hits <= {centre} and not ability.get('needs_direction'):
        # An area that hits only its centre (radius 1) is a single tile: the target, or the caster itself.
        return ('target' if ability.get('needs_target') or '/attack-' in ability['identity']['key'] else 'self'), frozenset()
    if ability.get('needs_direction'):
        # The matrix centre is the tile in front of the caster; the caster stands one tile south of it.
        anchor, kind = (centre[0], centre[1] + 1), 'area_direction'
    else:
        anchor, kind = centre, 'area_target' if ability.get('needs_target') else 'area_self'
    return kind, frozenset((x - anchor[0], y - anchor[1]) for x, y in hits if (x, y) != anchor)


def canary_abilities(monster, deps, effect_ids, missile_ids):
    effects = {e['identity']['key']: e for e in deps['effects']}
    formulas = {f['identity']['key']: f for f in deps['formulas']}
    out = []
    for ability in deps['abilities']:
        if 'variants' in ability:
            continue
        linked = [effects[r['key']] for r in ability.get('effects', []) if r['key'] in effects]
        damage = next((e for e in linked if e.get('operation') in ('damage', 'heal')), linked[0] if linked else {})
        presentation = next((e['presentation'] for e in linked if e.get('presentation')), {})
        magnitude = formulas.get(damage.get('formula', {}).get('key'), {}).get('magnitude', {})

        def asset_id(binding, table):
            name = (binding or '').rsplit('/', 1)[-1]
            return int(name[3:]) if name.startswith('id-') else table.get(name)

        kind, tiles = canary_geometry(ability)
        note, tiles = (None, tiles) if kind else (tiles, frozenset())
        out.append({'key': ability['identity']['key'].rsplit('/', 1)[-1], 'kind': kind, 'tiles': tiles, 'shape_note': note,
                    'element': 'healing' if damage.get('operation') == 'heal' else damage.get('damage_type'),
                    'maximum': magnitude.get('maximum'),
                    'effect': asset_id(presentation.get('impact_asset_binding'), effect_ids),
                    'default_hit': effect_ids.get(DEFAULT_HIT.get(damage.get('damage_type'))),
                    'missile': asset_id(presentation.get('projectile_asset_binding'), missile_ids)})
    return out


def wiki_abilities(text, shapes, cache, ids):
    fields = wc.infobox(text)
    out = []
    for list_name, items, _ in templates(fields.get('abilities', '')):
        if list_name.lower() != 'ability list':
            continue
        for item in items:
            for name, positional, named in templates(item):
                entry = ability_entry(name, positional, named, shapes, cache, ids)
                if entry:
                    out.append(entry)
    return out


def ability_entry(name, positional, named, shapes, cache, ids):
    lowered = name.lower()
    if lowered not in ('ability', 'healing'):
        return None
    label = positional[0] if lowered == 'ability' and positional else 'Healing'
    damage = named.get('range') if lowered == 'healing' else (positional[1] if len(positional) > 1 else named.get('damage'))
    element = 'healing' if lowered == 'healing' else (named.get('element') or (positional[2] if len(positional) > 2 else ''))
    maximum = re.findall(r'\d+', (damage or '').replace(',', ''))
    entry = {'name': label, 'element': ELEMENTS.get(element.strip().lower()), 'element_raw': element.strip(),
             'maximum': int(maximum[-1]) if maximum else None}
    scene = next((t for t in templates(named.get('scene', '')) if t[0].lower() == 'scene'), None)
    if scene is None:
        entry['kind'] = None
        entry['shape_note'] = 'no scene'
        return entry
    params = scene[2]
    shape = params.get('spell', '').strip().lower()
    entry['shape'] = shape
    if shape not in shapes:
        entry['kind'], entry['shape_note'] = None, 'shape not in ' + SCENE_DATA
    else:
        entry['kind'], tiles = wiki_geometry(shapes[shape])
        if entry['kind'] is None:
            entry['shape_note'] = tiles
        else:
            entry['tiles'] = tiles
    entry['effect'] = page_id(params['effect'], 'effectid', cache, ids) if params.get('effect') else None
    entry['missile'] = page_id(params['missile'], 'missileid', cache, ids) if params.get('missile') else None
    entry['effect_raw'], entry['missile_raw'] = params.get('effect'), params.get('missile')
    return entry


def same_shape(wiki, canary):
    if wiki.get('kind') is None or canary['kind'] is None:
        return None
    if wiki['kind'] != canary['kind']:
        return False
    return wiki.get('tiles', frozenset()) in rotations(canary['tiles'])


def score(wiki, canary):
    points = 0
    if wiki['element'] and wiki['element'] == canary['element']:
        points += 3
    if wiki.get('effect') is not None and wiki['effect'] in (canary['effect'], canary['default_hit']):
        points += 2
    if wiki.get('missile') is not None and wiki['missile'] == canary['missile']:
        points += 1
    if wiki.get('kind') is not None and wiki['kind'] == canary['kind']:
        points += 1
    if same_shape(wiki, canary):
        points += 1
    if wiki['maximum'] is not None and wiki['maximum'] == canary['maximum']:
        points += 1
    return points


def compare_monster(wiki, canary):
    rows, used = [], set()
    pairs = sorted(((score(w, c), i, j) for i, w in enumerate(wiki) for j, c in enumerate(canary)
                    if w.get('shape_note') != 'no scene'), reverse=True)
    matched = {}
    for points, i, j in pairs:
        if points >= 3 and i not in matched and j not in used:
            matched[i] = j
            used.add(j)
    for i, w in enumerate(wiki):
        row = {'wiki': w['name'], 'element': w['element_raw']}
        if w.get('shape_note') == 'no scene':
            rows.append({**row, 'status': 'NO_SCENE'})
            continue
        if i not in matched:
            rows.append({**row, 'status': 'NO_CANARY_MATCH'})
            continue
        c = canary[matched[i]]
        row['canary'] = c['key']
        shape = same_shape(w, c)
        row['shape'] = 'NOT_COMPARED' if shape is None else ('MATCH' if shape else 'DIFF')
        if shape is False:
            row['shape_detail'] = {'wiki': w.get('shape'), 'wiki_kind': w['kind'], 'canary_kind': c['kind'],
                                   'wiki_tiles': len(w.get('tiles', ())), 'canary_tiles': len(c['tiles'])}
        if shape is None:
            row['shape_note'] = w.get('shape_note') or c['shape_note']
        for aspect in ('effect', 'missile'):
            if w.get(aspect) is None and c[aspect] is None:
                row[aspect] = 'NONE'
            elif w.get(aspect) is None:
                row[aspect] = 'NOT_ON_WIKI' if w.get(aspect + '_raw') is None else 'WIKI_UNRESOLVED'
            elif aspect == 'effect' and c['effect'] is None and w['effect'] == c['default_hit']:
                row[aspect] = 'MATCH_DEFAULT_HIT'
            else:
                row[aspect] = 'MATCH' if w[aspect] == c[aspect] else 'DIFF'
                if row[aspect] == 'DIFF':
                    row[aspect + '_detail'] = {'wiki': w[aspect], 'wiki_name': w[aspect + '_raw'], 'canary': c[aspect]}
        rows.append(row)
    unmatched = [c['key'] for j, c in enumerate(canary) if j not in used and c['kind'] != 'melee']
    return rows, unmatched


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--canary', required=True, type=Path)
    parser.add_argument('--cache', type=Path, default=Path('/tmp/oteryn-wiki-cache'))
    args = parser.parse_args()
    args.cache.mkdir(parents=True, exist_ok=True)
    objects = cb.load_appearance_objects(args.canary / 'data/items/appearances.dat')
    items = cb.load_items_xml(args.canary / 'data/items/items.xml')
    names, index = cb.name_index(objects, items)
    converter = cb.Converter(args.canary, objects, items, names, index)
    converter.wiki = {}
    effect_ids = {k[len('CONST_ME_'):].lower(): v for k, v in converter.magic_effects.items() if k.startswith('CONST_ME_')}
    missile_ids = {k[len('CONST_ANI_'):].lower(): v for k, v in converter.missiles.items() if k.startswith('CONST_ANI_')}
    scene_record, shapes = scene_shapes(args.cache)
    ids, results, totals = {}, [], {}
    for path in sorted((args.canary / cb.MONSTER_DIR).rglob('*.lua')):
        relative = str(path.relative_to(args.canary / cb.MONSTER_DIR))[:-4]
        converter.pending_definitions = set()
        try:
            name = cb.load_monster(path, [])[0]
            slug, monster, deps, *_ = converter.convert(relative)
        except Exception:
            continue
        record = wc.fetch(name, args.cache)['cut']
        if not record:
            continue
        wiki = wiki_abilities(record['content'], shapes, args.cache, ids)
        if not wiki:
            continue
        rows, unmatched = compare_monster(wiki, canary_abilities(monster, deps, effect_ids, missile_ids))
        for row in rows:
            for aspect in ('status', 'shape', 'effect', 'missile'):
                if aspect in row:
                    totals.setdefault(aspect, {}).setdefault(row[aspect], 0)
                    totals[aspect][row[aspect]] += 1
        totals.setdefault('canary_not_on_wiki', 0)
        totals['canary_not_on_wiki'] += len(unmatched)
        interesting = [r for r in rows if r.get('status') == 'NO_CANARY_MATCH'
                       or 'DIFF' in (r.get('shape'), r.get('effect'), r.get('missile'))]
        results.append({'monster': slug, 'wiki_title': name, 'page_id': record['page_id'], 'cut_revision_id': record['revision_id'],
                        'abilities': len(rows), 'rows': interesting, 'canary_not_on_wiki': unmatched})
    report = {'source': 'TibiaWiki (Fandom), CC BY-SA; only compared facts are recorded', 'api': wc.API,
              'target_cut': wc.TARGET_CUT, 'scene_data': {'title': SCENE_DATA, 'page_id': scene_record['page_id'],
                                                           'revision_id': scene_record['revision_id'], 'shapes': len(shapes)},
              'scope': 'Wiki ability scenes (shape, effect id, missile id) against the plain Canary conversion; nothing is '
                       'adopted. Per monster only abilities without a Canary match or with a DIFF are listed.',
              'matching': 'Greedy by score: element 3, effect id (own or default hit) 2, anchor kind 1, shape 1, missile id 1, '
                          'maximum damage 1; at least 3. Wiki abilities without a scene are counted as NO_SCENE and not matched.',
              'totals': {k: dict(sorted(v.items())) if isinstance(v, dict) else v for k, v in sorted(totals.items())},
              'monsters': []}
    head = json.dumps(report, ensure_ascii=False, indent=2)[:-len('\n  "monsters": []\n}')]
    lines = ',\n'.join('    ' + json.dumps(r, ensure_ascii=False, separators=(',', ':')) for r in results)
    OUT.write_text(head + '\n  "monsters": [\n' + lines + '\n  ]\n}\n', encoding='utf-8', newline='\n')
    print(json.dumps({'monsters': len(results), 'totals': report['totals']}))


if __name__ == '__main__':
    main()
