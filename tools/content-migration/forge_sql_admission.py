"""Check/generate the immutable SQL projection of the existing Forge policy.

Like migration 0070's Wheel capacities, this is proof data installed only by a
migration. It creates no item/resource owner and accepts no runtime-written
classification. A future native profile or Forge price change must update this
projection in its own migration before that policy can mutate durable value.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path

from forge_item_profiles import owners

ROOT = Path(__file__).resolve().parents[2]
MIGRATION = ROOT / 'apps/game-server/migrations/0727_native_forge_profile_admission.sql'
BEGIN = '-- BEGIN GENERATED FORGE POLICY\n'
END = '-- END GENERATED FORGE POLICY\n'
SLOTS = {'WEAPON': 'hand', 'SHIELD': 'hand', 'HEAD': 'head', 'TORSO': 'armor',
         'LEGS': 'legs', 'FEET': 'feet'}


def policy():
    native = json.loads((ROOT / 'content/items/definitions/spell-native-profiles.json').read_bytes())
    admitted = owners(ROOT)
    profiles, materials = [], []
    for row in native['records']:
        identity = row['production_definition']
        if identity['production_key'] in ('oteryn:item.tibia.i37109', 'oteryn:item.tibia.i37110'):
            if (row['admission']['stack_class'] != 'StackCapable'
                    or row['semantics']['stack']['value']['stack_max']['value'] != 100):
                raise ValueError('FORGE_SQL_MATERIAL_PROFILE')
            materials.append((identity['production_key'], identity['revision_ref']))
        forge = row.get('authoring', {}).get('forge')
        if forge is None:
            continue
        owner = admitted[identity['production_key']]
        if (forge != owner['authoring']['forge']
                or row['production_binding']['target'] != owner['definition']['identity']):
            raise ValueError('FORGE_SQL_PROFILE_OWNER')
        # Shared native cohorts can admit stackable items carrying reference
        # Forge classifications. ForgeItemShape rejects those items; its SQL
        # admission must exclude them without changing their existing custody.
        if row['admission']['stack_class'] != 'NonStackable':
            continue
        patterns = row['semantics']['equipment']['value']['patterns']['value']
        slots = {SLOTS[p['primary_slot']['value']] for p in patterns}
        if len(slots) != 1:
            raise ValueError('FORGE_SQL_PROFILE_SLOT')
        profiles.append((identity['production_key'], identity['revision_ref'],
                         forge['classification'], forge['max_tier'], slots.pop()))
    profiles.sort()
    materials.sort()
    if len(profiles) != len({row[0] for row in profiles}) or not profiles:
        raise ValueError('FORGE_SQL_PROFILE_SET')
    if len(materials) != 2:
        raise ValueError('FORGE_SQL_MATERIAL_SET')

    text = (ROOT / 'crates/gameplay/items/src/forge/prices.rs').read_text()
    prices = []
    for kind, values in re.findall(r'^    (row|convergence)\(([\d_, ]+)\),$', text, re.MULTILINE):
        values = [int(value.replace('_', '')) for value in values.split(',')]
        if kind == 'row':
            classification, tier, gold, cores = values
            prices.append((classification, tier, gold, cores, 0, 0))
        else:
            tier, gold, cores, fusion, transfer = values
            prices.append((4, tier, gold, cores, fusion, transfer))
    if len(prices) != 16 or len({row[:2] for row in prices}) != 16:
        raise ValueError('FORGE_SQL_PRICE_SET')
    return profiles, prices, materials


def generated():
    profiles, prices, materials = policy()
    fingerprint = hashlib.sha256(json.dumps([profiles, prices, materials], separators=(',', ':')).encode()).hexdigest()
    profile_rows = ',\n'.join(
        f"  ('{key}','{revision}',{classification},{maximum},'{slot}')"
        for key, revision, classification, maximum, slot in profiles)
    price_rows = ',\n'.join('  (' + ','.join(map(str, row)) + ')' for row in prices)
    material_rows = ',\n'.join(f"  ('{key}','{revision}')" for key, revision in materials)
    return f"""{BEGIN}-- Producer: tools/content-migration/forge_sql_admission.py --write
-- Canonical native profiles + crates/gameplay/items/src/forge/prices.rs.
-- Projection SHA256: {fingerprint}
CREATE OR REPLACE FUNCTION game_forge_admitted_profile(p_key TEXT,p_revision TEXT)
RETURNS TABLE(classification INT,max_tier INT,equipment_slot TEXT)
LANGUAGE sql IMMUTABLE AS $$
 SELECT c,m,s FROM (VALUES
{profile_rows}
 ) p(k,r,c,m,s) WHERE p.k=p_key AND p.r=p_revision
$$;

-- Crystal 201189f item_tiers.lua:1-90; result-tier price for transfer.
CREATE OR REPLACE FUNCTION game_forge_admitted_price(p_class INT,p_tier INT)
RETURNS TABLE(gold BIGINT,cores INT,convergence_fusion BIGINT,convergence_transfer BIGINT)
LANGUAGE sql IMMUTABLE AS $$
 SELECT g::bigint,n,f::bigint,t::bigint FROM (VALUES
{price_rows}
 ) p(c,r,g,n,f,t) WHERE p.c=p_class AND p.r=p_tier
$$;

CREATE OR REPLACE FUNCTION game_forge_admitted_material(p_key TEXT,p_revision TEXT) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT EXISTS(SELECT 1 FROM (VALUES
{material_rows}
 ) p(k,r) WHERE p.k=p_key AND p.r=p_revision)
$$;
{END}"""


def checked_projection():
    text = MIGRATION.read_text()
    start = text.index(BEGIN)
    end = text.index(END, start) + len(END)
    return text, text[:start] + generated() + text[end:]


def check():
    actual, expected = checked_projection()
    if actual != expected:
        raise SystemExit('FORGE_SQL_ADMISSION_DRIFT: update the policy admission migration')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if args.write:
        _, expected = checked_projection()
        MIGRATION.write_text(expected)
    else:
        check()


if __name__ == '__main__':
    main()
