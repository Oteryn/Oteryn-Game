"""PARITY-IMBUE-FORGE-WIRE-1: the imbuement recipe catalogue from pinned Canary imbuements.xml.

Canary is the value source and Crystal the cross-check: every family, tier, premium flag and
material list must match the committed Crystal facts except the declared differences below,
or generation stops. The output is the Rust table the server-only apply writer reads.

    python3 tools/content-migration/imbuement_catalogue.py [--check]
    python3 tools/content-migration/imbuement_catalogue.py --stage-crystal <crystal imbuements.xml>
"""
import argparse
import hashlib
import json
import subprocess
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CANARY = 'imports/canary/imbuements-xml/imbuements.xml'
CANARY_REVISION = '47dfd51f45280a59a1d3e50ba7edd573d7234446'
CANARY_SHA256 = '82ea7b6c28b47b1b3d631ecff6d0015d5019d41327656945609d84340bf95d63'
CRYSTAL_FACTS = 'imports/crystalserver/facts/imbuement-materials.json'
CRYSTAL_REVISION = 'ff7ede593c69d4c658b382c97443e8155926924a'
CRYSTAL_SHA256 = '7e4ae3be28e0b3e740e856e8cec2c8c9ff01e7f9ab4b2b8d3b970b51ca1df8fd'
OUTPUT = 'crates/gameplay/items/src/imbuement_catalogue.rs'
# Canary category id -> ImbuementFamily wire value (protocol-oteryn imbuement_v1.proto).
# Category 0 (elemental damage) carries five element names in one category and has no
# single family recipe; it is left out (declared difference).
FAMILY = {1: 11, 2: 12, 3: 1, 4: 3, 5: 4, 6: 6, 7: 8, 8: 5, 9: 7, 10: 10, 11: 14, 12: 20,
          13: 15, 14: 19, 15: 16, 16: 18, 17: 9, 18: 17, 19: 13}
# Declared Canary/Crystal differences; Canary values are used.
BASE_PRICE_DIFFERS = True
CATEGORY_NAME_DIFFERS = {19}
MATERIALS_DIFFER = {('Punch', 1)}


def parse(xml_bytes):
    """Categories, tiers and `{(name, tier): (category, premium, materials)}` as Canary
    loads them (src/creatures/players/imbuements/imbuements.cpp): a repeated item id is
    ignored and a missing count is 1."""
    root = ET.fromstring(xml_bytes)
    categories = {int(c.get('id')): c.get('name') for c in root.iter('category')}
    bases = {int(b.get('id')): {'price': int(b.get('price')), 'removecost': int(b.get('removecost')),
                                'duration': int(b.get('duration'))} for b in root.iter('base')}
    recipes = {}
    for imbuement in root.iter('imbuement'):
        materials, seen = [], set()
        for attribute in imbuement.findall('attribute'):
            if attribute.get('key') != 'item':
                continue
            item = int(attribute.get('value'))
            if item in seen:
                continue
            seen.add(item)
            materials.append([item, int(attribute.get('count', '1'))])
        key = (imbuement.get('name'), int(imbuement.get('base')))
        if key in recipes or key[1] not in bases:
            raise ValueError(f'IMBUEMENT_CATALOGUE_RECIPE {key}')
        recipes[key] = (int(imbuement.get('category')), imbuement.get('premium') == '1', materials)
    return categories, bases, recipes


def crystal_facts(xml_bytes):
    if hashlib.sha256(xml_bytes).hexdigest() != CRYSTAL_SHA256:
        raise ValueError('IMBUEMENT_CATALOGUE_CRYSTAL_SOURCE')
    categories, bases, recipes = parse(xml_bytes)
    return {
        'bases': {str(k): v for k, v in sorted(bases.items())},
        'categories': {str(k): v for k, v in sorted(categories.items())},
        'evidence': 'OtsHypothesisOnly',
        'extracted_by': 'tools/content-migration/imbuement_catalogue.py --stage-crystal',
        'recipes': [{'category': c, 'materials': m, 'name': n, 'premium': p, 'tier': t}
                    for (n, t), (c, p, m) in sorted(recipes.items())],
        'schema': 'OTERYN_CRYSTAL_IMBUEMENT_MATERIAL_FACTS/v1',
        'source': {'path': 'data/XML/imbuements.xml', 'repository': 'zimbadev/crystalserver',
                   'revision': CRYSTAL_REVISION, 'sha256': CRYSTAL_SHA256},
    }


def cross_check(canary, crystal):
    categories, bases, recipes = canary
    if {int(k) for k in crystal['categories']} != set(categories) or any(
            crystal['categories'][str(k)] != v for k, v in categories.items() if k not in CATEGORY_NAME_DIFFERS):
        raise ValueError('IMBUEMENT_CATALOGUE_CRYSTAL_CATEGORIES')
    for tier, base in bases.items():
        other = crystal['bases'][str(tier)]
        if other['removecost'] != base['removecost'] or other['duration'] != base['duration'] or (
                not BASE_PRICE_DIFFERS and other['price'] != base['price']):
            raise ValueError(f'IMBUEMENT_CATALOGUE_CRYSTAL_BASE {tier}')
    other = {(r['name'], r['tier']): (r['category'], r['premium'], r['materials']) for r in crystal['recipes']}
    if set(other) != set(recipes):
        raise ValueError('IMBUEMENT_CATALOGUE_CRYSTAL_RECIPES')
    for key, (category, premium, materials) in recipes.items():
        theirs = other[key]
        if theirs[:2] != (category, premium) or (theirs[2] != materials and key not in MATERIALS_DIFFER):
            raise ValueError(f'IMBUEMENT_CATALOGUE_CRYSTAL_RECIPE {key}')


def rust(canary):
    _, bases, recipes = canary
    rows = []
    for (name, tier), (category, premium, materials) in recipes.items():
        if category == 0:
            continue
        if not 1 <= len(materials) <= 3 or bases[tier]['removecost'] != 15000 or bases[tier]['duration'] != 72000:
            raise ValueError(f'IMBUEMENT_CATALOGUE_SHAPE {name} {tier}')
        key = 'oteryn:imbuement.' + name.lower().replace(' ', '_').replace("'", '')
        rows.append((FAMILY[category], tier, key, premium, bases[tier]['price'], materials))
    rows.sort()
    if [(r[0], r[1]) for r in rows] != [(f, t) for f in sorted(FAMILY.values()) for t in (1, 2, 3)]:
        raise ValueError('IMBUEMENT_CATALOGUE_FAMILIES')
    out = [
        f'// @generated by tools/content-migration/imbuement_catalogue.py from {CANARY}',
        f'// (opentibiabr/canary {CANARY_REVISION}, data/XML/imbuements.xml), cross-checked',
        f'// against {CRYSTAL_FACTS}. Do not edit; rerun the generator.',
        '',
        'use super::ImbuementRecipe;',
        '',
        '/// SHA-256 of the pinned source; the durable `imbuement_revision`.',
        f'pub const IMBUEMENT_CATALOGUE_REVISION: &str = "{CANARY_SHA256}";',
        '',
        '/// Ascending by `(family, tier)`.',
        'pub const IMBUEMENT_RECIPES: [ImbuementRecipe; %d] = [' % len(rows),
    ]
    for family, tier, key, premium, price, materials in rows:
        pairs = ', '.join(f'(MATERIAL_{item}, {count})' for item, count in materials)
        out += [
            '    ImbuementRecipe {',
            f'        family: {family},',
            f'        tier: {tier},',
            f'        key: "{key}",',
            f'        requires_premium: {str(premium).lower()},',
            f'        apply_gold: {price},',
            f'        materials: &[{pairs}],',
            '    },',
        ]
    out.append('];')
    for item in sorted({item for row in rows for item, _ in row[5]}):
        out += ['', f'const MATERIAL_{item}: &str = "oteryn:item.tibia.i{item}";']
    return '\n'.join(out) + '\n'


def generate(root):
    raw = (root / CANARY).read_bytes()
    if hashlib.sha256(raw).hexdigest() != CANARY_SHA256:
        raise ValueError('IMBUEMENT_CATALOGUE_CANARY_SOURCE')
    canary = parse(raw)
    cross_check(canary, json.loads((root / CRYSTAL_FACTS).read_bytes()))
    # The repository rustfmt.toml is the only layout, so `cargo fmt` never rewrites the table.
    return subprocess.run(['rustfmt', '--config-path', str(root / 'rustfmt.toml'), '--emit', 'stdout'],
                          input=rust(canary).encode(), capture_output=True, check=True).stdout


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--stage-crystal', type=Path)
    args = parser.parse_args()
    if args.stage_crystal:
        facts = crystal_facts(args.stage_crystal.read_bytes())
        (ROOT / CRYSTAL_FACTS).write_text(json.dumps(facts, indent=2, sort_keys=True) + '\n')
        return 0
    generated = generate(ROOT)
    target = ROOT / OUTPUT
    if args.check:
        if not target.exists() or target.read_bytes() != generated:
            print(f'{OUTPUT} is stale; rerun imbuement_catalogue.py', file=sys.stderr)
            return 1
        return 0
    target.write_bytes(generated)
    return 0


if __name__ == '__main__':
    sys.exit(main())
