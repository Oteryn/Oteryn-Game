"""PREY0 §3, §6.1 and RL-16 budget qualification of exact native output bytes.

This is a required stage when the native producer receives a PREY provider. It
does not admit draw/grade tables, authorize runtime activation or rewrite content.
The provider names the complete authored pool; source exclusions must already be
explicit in the exact Creature profile, never inferred from an oversized table.
"""
import hashlib
import json
import re

PROVIDER_SCHEMA = 'OTERYN_PREY_LOOT_PROVIDER/v1'
FIELDS = {'schema', 'creature_profiles_sha256', 'loot_tables_sha256', 'preyable_creatures'}
MAX_PROVIDER_BYTES = 8 * 1024 * 1024  # Existing native input bound.
MAX_RECORDS = 4096  # Existing native Creature/Loot record bound.
MIN_POOL = 27  # PREY0 §6.1; fewer Creatures disable Prey for the generation.
MAX_ENTRIES = 7  # PREY0-RL-16: two rolls + trigger <= 29 draws and 14 outputs.


def _object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f'PREY_PROVIDER_INVALID: duplicate JSON field {key}')
        result[key] = value
    return result


def _decode(raw):
    try:
        return json.loads(raw, object_pairs_hook=_object)
    except (TypeError, UnicodeError, json.JSONDecodeError) as error:
        raise ValueError('PREY_PROVIDER_INVALID: expected UTF-8 JSON') from error


def _ref(value, family):
    if (not isinstance(value, dict) or set(value) != {'family', 'key', 'revision'}
            or value['family'] != family
            or any(not isinstance(value[field], str) or not value[field]
                   for field in ('key', 'revision'))):
        raise ValueError(f'PREY_PROVIDER_INVALID: exact {family} DefinitionRef required')
    return value['family'], value['key'], value['revision']


def _preyable(target, profile):
    if not isinstance(profile, dict):
        raise ValueError(f'PREY_ELIGIBILITY_UNKNOWN: {target[1]} canonical/native profile missing')
    bestiary = profile.get('bestiary')
    if bestiary is None:
        return False
    xp = profile.get('experience')
    if not isinstance(bestiary, dict) or not bestiary or type(xp) is not int or not 0 <= xp < 2**64:
        raise ValueError(f'PREY_ELIGIBILITY_UNKNOWN: {target[1]} Bestiary/experience')
    if xp == 0:
        return False
    details = profile.get('details')
    eligibility = details.get('system_eligibility') if isinstance(details, dict) else None
    prey = eligibility.get('prey') if isinstance(eligibility, dict) else None
    if type(prey) is not bool:
        raise ValueError(f'PREY_ELIGIBILITY_UNKNOWN: {target[1]} explicit prey fact required')
    return prey


def validate_prey_loot_provider(provider_raw, creatures_raw, loot_raw, canonical_creatures):
    """Refuse before emission unless the whole declared pool fits the exact loot output.

    `canonical_creatures` comes from the producer's actual canonical definition
    shards. A source-only native profile's null loot binding is not an admitted
    no-loot Creature. Matching hashes establish content binding, not live authority.
    """
    if not isinstance(provider_raw, bytes) or len(provider_raw) > MAX_PROVIDER_BYTES:
        raise ValueError('PREY_PROVIDER_INVALID: missing or oversized provider')
    provider = _decode(provider_raw)
    if not isinstance(provider, dict) or set(provider) != FIELDS or provider['schema'] != PROVIDER_SCHEMA:
        raise ValueError('PREY_PROVIDER_INVALID: closed provider schema required')
    for field, raw in (('creature_profiles_sha256', creatures_raw), ('loot_tables_sha256', loot_raw)):
        pin = provider[field]
        if not isinstance(pin, str) or re.fullmatch('[0-9a-f]{64}', pin) is None:
            raise ValueError(f'PREY_PROVIDER_INVALID: {field}')
        if pin != hashlib.sha256(raw).hexdigest():
            raise ValueError(f'PREY_SOURCE_MISMATCH: {field}')
    pool = provider['preyable_creatures']
    if not isinstance(pool, list) or len(pool) > MAX_RECORDS:
        raise ValueError('PREY_PROVIDER_INVALID: bounded preyable_creatures required')
    claimed = [_ref(value, 'Creature') for value in pool]
    keys = [value[1].encode('utf-8') for value in claimed]
    if keys != sorted(set(keys)):
        raise ValueError('PREY_PROVIDER_INVALID: pool must have unique keys in UTF-8 key order')
    creatures, loot = _decode(creatures_raw), _decode(loot_raw)
    if (creatures.get('schema') != 'OTERYN_NATIVE_CREATURE_PROFILES/v1'
            or loot.get('schema') != 'OTERYN_NATIVE_LOOT_TABLES/v1'):
        raise ValueError('PREY_PROVIDER_INVALID: native source schemas required')
    canonical = {}
    for record in canonical_creatures:
        target = _ref(record['definition']['identity'], 'Creature')
        authoring = record.get('authoring')
        canonical[target] = authoring.get('profile') if isinstance(authoring, dict) else None
    expected, targets = set(), set()
    for record in creatures['records']:
        target = _ref(record['profile']['target'], 'Creature')
        if target in targets:
            raise ValueError(f'PREY_PROVIDER_INVALID: duplicate native Creature {target[1]}')
        targets.add(target)
        eligible = _preyable(target, record['profile']['data']['profile'])
        if target in canonical and eligible != _preyable(target, canonical[target]):
            raise ValueError(f'PREY_SOURCE_MISMATCH: {target[1]} canonical/native eligibility')
        if eligible:
            expected.add(target)
    if set(claimed) != expected:
        missing, extra = expected - set(claimed), set(claimed) - expected
        raise ValueError(f'PREY_POOL_MISMATCH: missing {sorted(missing)}; unexpected {sorted(extra)}')
    if len(claimed) < MIN_POOL:
        raise ValueError(f'PREY_POOL_TOO_SMALL: {len(claimed)} < {MIN_POOL}; generation disabled')
    bindings, tables = {}, {}
    for row in loot['creature_loot']:
        target = _ref(row['creature'], 'Creature')
        if target in bindings:
            raise ValueError(f'PREY_PROVIDER_INVALID: duplicate loot binding {target[1]}')
        bindings[target] = row['loot']
    if set(bindings) != targets:
        raise ValueError('PREY_SOURCE_MISMATCH: native Creature/loot binding coverage')
    for table in loot['tables']:
        target = _ref(table['identity'], 'Loot')
        if target in tables:
            raise ValueError(f'PREY_PROVIDER_INVALID: duplicate exact Loot {target[1]}')
        tables[target] = table
    for target in claimed:
        if target not in canonical:
            raise ValueError(f'PREY_CREATURE_DEFINITION_MISSING: {target[1]}')
        binding = bindings[target]
        if binding is None:
            continue  # The canonical Creature explicitly has no Loot reference.
        table = tables.get(_ref(binding, 'Loot'))
        if table is None:
            raise ValueError(f'PREY_SOURCE_MISMATCH: {target[1]} exact Loot absent')
        if (set(table) != {'identity', 'algorithm', 'entries'}
                or table['algorithm'] != 'IndependentBernoulliPpm'
                or not isinstance(table['entries'], list)):
            raise ValueError(f'PREY_LOOT_UNSUPPORTED: {target[1]} table shape/algorithm')
        if len(table['entries']) > MAX_ENTRIES:
            raise ValueError(f'PREY_LOOT_TABLE_TOO_LARGE: {target[1]} has {len(table["entries"])} entries; maximum {MAX_ENTRIES}')
        for entry in table['entries']:
            if not isinstance(entry, dict) or set(entry) != {'item', 'min_count', 'max_count', 'probability_ppm'}:
                raise ValueError(f'PREY_LOOT_UNSUPPORTED: {target[1]} nested/alternate/unknown entry')
            _ref(entry['item'], 'Item')
            low, high, chance = (entry[field] for field in ('min_count', 'max_count', 'probability_ppm'))
            if (any(type(value) is not int for value in (low, high, chance))
                    or not 0 <= low <= high or high == 0 or not 0 <= chance <= 1_000_000):
                raise ValueError(f'PREY_LOOT_UNSUPPORTED: {target[1]} quantity/chance')
        # Count every emitted entry, including zero-chance and repeated Item refs.
        # No source candidate, alternate table or deduplicated item list is used.
