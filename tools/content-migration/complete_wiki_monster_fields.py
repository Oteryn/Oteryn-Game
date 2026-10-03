"""Generate guarded field patches from a frozen, actual WikiBR page capture.

This tool does not edit population bundles or claim live spell execution.
"""
import argparse
import copy
import gzip
import hashlib
import json
import re
import unicodedata
from collections import Counter
from fractions import Fraction
from pathlib import Path

BASELINE_SHA = '43cf343cba51a843ce6116260bfdedf9f8765fb79d030f9ef4b7ec61906538b1'
BR_SHA = '785509ec404bbd47b9e799711546cff8a2389168d419eae7f61927338863854a'
DIFFICULTIES = {'inofensivo': 'harmless', 'inofensiva': 'harmless', 'trivial': 'trivial', 'facil': 'easy', 'medio': 'medium', 'dificil': 'hard', 'desafiador': 'challenging'}
OCCURRENCES = {'comum': 'common', 'incomum': 'uncommon', 'raro': 'rare', 'rara': 'rare', 'muito raro': 'very_rare', 'muito rara': 'very_rare'}
CLASSES = {'anfibios': ('Amphibic', 'amphibic'), 'aquaticos': ('Aquatic', 'aquatic'), 'aves': ('Bird', 'bird'), 'constructos': ('Construct', 'construct'), 'demonios': ('Demon', 'demon'), 'dragoes': ('Dragon', 'dragon'), 'elementais': ('Elemental', 'elemental'), 'extra dimensionais': ('Extra Dimensional', 'extra_dimensional'), 'fadas': ('Fey', 'fey'), 'gigantes': ('Giant', 'giant'), 'humanos': ('Human', 'human'), 'humanoides': ('Humanoid', 'humanoid'), 'imortais': ('Immortal', 'immortal'), 'inkborn': ('Inkborn', 'inkborn'), 'licantropos': ('Lycanthrope', 'lycanthrope'), 'mamiferos': ('Mammal', 'mammal'), 'criaturas magicas': ('Magical', 'magical'), 'plantas': ('Plant', 'plant'), 'plantas (criatura)': ('Plant', 'plant'), 'repteis': ('Reptile', 'reptile'), 'slimes': ('Slime', 'slime'), 'mortos-vivos': ('Undead', 'undead'), 'vermes': ('Vermin', 'vermin')}
THRESHOLDS = {'harmless': [5, 10, 25], 'trivial': [10, 100, 250], 'easy': [25, 250, 500], 'medium': [50, 500, 1000], 'hard': [100, 1000, 2500], 'challenging': [200, 2000, 5000]}
DAMAGE = {'physical': 'physical', 'energy': 'energy', 'earth': 'earth', 'fire': 'fire', 'death': 'death', 'holy': 'holy', 'ice': 'ice', 'lifedrain': 'life_drain', 'manadrain': 'mana_drain', 'drown': 'drowning'}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    path = Path(path)
    if path.suffix == '.gz':
        with gzip.open(path, 'rt', encoding='utf-8') as stream:
            return json.load(stream)
    return json.loads(path.read_text())


def normalize(value):
    return unicodedata.normalize('NFKD', str(value)).encode('ascii', 'ignore').decode().lower().strip()


def numeric(raw):
    value = str(raw).strip().replace('%', '').replace(',', '.').replace(' ', '')
    return Fraction(value) if re.fullmatch(r'-?\d+(?:\.\d+)?', value) else None


def rational(value):
    return {'numerator': value.numerator, 'denominator': value.denominator}


def get(document, pointer):
    value = document
    for segment in pointer.strip('/').split('/'):
        if not isinstance(value, dict) or segment not in value:
            return False, None
        value = value[segment]
    return True, value


def source(page, fields=()):
    result = {key: page[key] for key in ('url', 'revision_id', 'content_sha256', 'method')}
    result['source_file'] = page['page_title']
    result['source_line'] = min((page.get('field_lines', {}).get(key, 1) for key in fields), default=1)
    result['source_field'] = ','.join(fields) or 'documented helper rule'
    result['fields'] = {key: {'raw': page['fields'].get(key), 'source_line': page.get('field_lines', {}).get(key)} for key in fields}
    return result


def patch_actor(actor, document, page, *, variant=False, estimated=False, helpers=None, class_fact=None):
    """Coalesce array edits while retaining unrelated entries and precise expectations."""
    helpers = helpers or {}
    patches, deferred = [], []
    resolved = []
    if variant:
        return patches, [{'field': '*', 'reason': 'SOURCE_VARIANT_SHARED_WIKI_PAGE_PARITY_UNPROVEN'}], resolved
    fields = page['fields']

    def emit(pointer, value, keys, reason, additional=(), override=None):
        present, old = get(document, pointer)
        if present and old == value:
            return
        row = {'monster': actor, 'file': 'monster.json', 'pointer': pointer, 'expected_present': present, 'expected_value': old, 'value': value, 'source': override or source(page, keys), 'reason': reason}
        if additional:
            row['supporting_sources'] = list(additional)
        patches.append(row)

    stats = document['creature']['stats']
    for key, target in [('hp', 'max_health'), ('exp', 'experience'), ('speed', 'speed'), ('defense', 'armor'), ('mitigation', 'mitigation_percent')]:
        value = numeric(fields.get(key, ''))
        if value is None:
            deferred.append({'field': 'stats.' + target, 'reason': 'WIKI_CELL_ABSENT_EMPTY_OR_NONNUMERIC'})
            continue
        if value < 0 or (key == 'hp' and value == 0) or (key != 'mitigation' and value.denominator != 1):
            deferred.append({'field': 'stats.' + target, 'reason': 'WIKI_VALUE_NOT_ADMISSIBLE_OR_HP_ZERO_PLACEHOLDER'})
            continue
        if key == 'speed' and value and stats[target] and (value == 2 * stats[target] or 2 * value == stats[target]):
            deferred.append({'field': 'stats.speed', 'reason': 'ENGINE_DISPLAY_FACTOR_TWO_UNIT_UNRESOLVED'})
            continue
        typed = rational(value) if key == 'mitigation' else int(value)
        emit('/creature/stats/' + target, typed, [key], 'Direct same-actor actual WikiBR numeric cell.' )
        if key == 'hp' and stats['initial_health'] == stats['max_health']:
            emit('/creature/stats/initial_health', int(value), [key], 'Initial health followed old maximum; preserve full-health spawn after maximum correction.')
        if key == 'mitigation' and estimated:
            # A provenance-only equal-value patch is intentional when replacing an estimate.
            if stats[target] == typed:
                patches.append({'monster': actor, 'file': 'monster.json', 'pointer': '/creature/stats/mitigation_percent', 'expected_present': True, 'expected_value': typed, 'value': typed, 'source': source(page, [key]), 'reason': 'Direct same-actor wiki confirmation replaces estimate provenance.'})
            resolved.append('OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE')

    resistances = copy.deepcopy(document['creature']['resistances'])
    immunities = list(document['creature']['immunities']['damage_types'])
    damage_keys = []
    for br, damage in DAMAGE.items():
        key = br + 'DmgMod'
        taken = numeric(fields.get(key, ''))
        if taken is None:
            deferred.append({'field': 'resistance.' + damage, 'reason': 'NO_NUMERIC_RECEIVED_DAMAGE_CELL'})
            continue
        if taken < 0:
            deferred.append({'field': 'resistance.' + damage, 'reason': 'NEGATIVE_RECEIVED_DAMAGE_NOT_ADMISSIBLE'})
            continue
        damage_keys.append(key)
        reduction = 100 - taken
        current = next((entry for entry in resistances if entry['damage_type'] == damage), None)
        effective = Fraction(100) if damage in immunities else Fraction(current['reduction_percent']['numerator'], current['reduction_percent']['denominator']) if current else Fraction(0)
        if effective == reduction:
            continue
        if damage in immunities:
            immunities.remove(damage)
        if current:
            current['reduction_percent'] = rational(reduction)
        elif reduction:
            resistances.append({'damage_type': damage, 'reduction_percent': rational(reduction)})
    emit('/creature/resistances', resistances, damage_keys, 'Coalesced effective damage reduction =100 minus wiki received-damage percentage; unrelated types retained.')
    emit('/creature/immunities/damage_types', immunities, damage_keys, 'Remove only damage immunities explicitly contradicted by numeric received-damage cell; resistance expresses new percentage.')

    raw = normalize(fields.get('immunities', ''))
    tokens = [value.strip() for value in re.split('[,/]', raw) if value.strip()]
    known = raw in ('none', 'nenhuma', 'nenhum', '--') or bool(tokens) and all(any(tag in token for tag in ['invisib', 'paraly', 'parali', 'drunk', 'embri', 'fire', 'earth', 'energy', 'death', 'holy', 'ice', 'physical']) for token in tokens)
    if known:
        conditions = list(document['creature']['immunities']['conditions'])
        paralyze = any(word in raw for word in ['paralysis', 'paralise', 'paralyze'])
        if paralyze and 'paralyze' not in conditions:
            conditions.append('paralyze')
        elif not paralyze and 'paralyze' in conditions:
            conditions.remove('paralyze')
        emit('/creature/immunities/conditions', conditions, ['immunities'], 'Explicit WikiBR paralysis immunity; retain other condition immunities, including unresolved Invisibility.')
    deferred.append({'field': 'immunity.invisible', 'reason': 'SENSORY_VS_INTRINSIC_CONDITION_IMMUNITY_SEMANTICS_UNRESOLVED'})

    familiar = document['creature']['summoning']['is_familiar']
    mana_values = []
    for key, flag in [('summon', 'summonable'), ('convince', 'convinceable')]:
        value = numeric(fields.get(key, ''))
        if familiar:
            if key == 'summon' and value is not None and value > 0 and value.denominator == 1:
                emit('/creature/summoning/familiar/mana_cost', int(value), [key], 'Familiar owner summon ability mana; not ordinary Summon Creature eligibility.')
            continue
        permit = value > 0 if value is not None else False if normalize(fields.get(key, '')) in ('--', 'nao', 'no') else None
        if permit is not None:
            emit('/creature/summoning/' + flag, permit, [key], 'Explicit numeric mana or explicit no/-- ordinary summoning eligibility.')
        if value is not None and value > 0 and value.denominator == 1:
            mana_values.append(int(value))
    if not familiar and mana_values:
        if len(set(mana_values)) == 1:
            emit('/creature/summoning/mana_cost', mana_values[0], ['summon', 'convince'], 'Actual wiki mana cost; all documented applicable ordinary costs agree.')
        else:
            deferred.append({'field': 'summoning.mana_cost', 'reason': 'MULTIPLE_DISTINCT_SUMMON_CONVINCE_COSTS_NOT_REPRESENTABLE'})

    difficulty = DIFFICULTIES.get(normalize(fields.get('dificuldade', '')))
    occurrence = OCCURRENCES.get(normalize(fields.get('ocorrencia', '')))
    class_pair = class_fact['class_pair'] if class_fact else None
    old_bestiary = document['creature'].get('bestiary')
    if difficulty and occurrence and (old_bestiary or class_pair):
        bestiary = copy.deepcopy(old_bestiary) if old_bestiary else {}
        bestiary.update({'difficulty': difficulty, 'occurrence': occurrence})
        if class_pair:
            bestiary.update({'class': class_pair[0], 'taxonomy': class_pair[1]})
        # Star counts are the documented difficulty rank (harmless0..challenging5).
        if 'stars' in bestiary:
            bestiary['stars'] = list(THRESHOLDS).index(difficulty)
        if occurrence != 'very_rare' and 'difficulty' in helpers:
            bestiary['kill_thresholds'] = THRESHOLDS[difficulty]
        charm = numeric(fields.get('charm', ''))
        if charm is not None and charm >= 0 and charm.denominator == 1:
            bestiary['charm_points'] = int(charm)
        if all(key in bestiary for key in ['class', 'taxonomy', 'difficulty', 'occurrence', 'kill_thresholds', 'charm_points']):
            if old_bestiary:
                keys = {'class': [], 'taxonomy': [], 'difficulty': ['dificuldade'], 'occurrence': ['ocorrencia'], 'stars': ['dificuldade'], 'kill_thresholds': ['dificuldade', 'ocorrencia'], 'charm_points': ['charm']}
                for key, raw_keys in keys.items():
                    if key not in bestiary:
                        continue
                    override = class_fact['source'] if key in ('class', 'taxonomy') and class_fact else None
                    emit('/creature/bestiary/' + key, bestiary[key], raw_keys, 'Actual Bestiary field correction; retain unrelated native narrative/class fields and rare intermediate thresholds.', [source(helper) for helper in helpers.values()] if key in ('stars', 'kill_thresholds') else (), override)
            else:
                emit('/creature/bestiary', bestiary, ['dificuldade', 'ocorrencia', 'charm'], 'Complete positive Wiki Bestiary facts; class comes from actual Fandom Bestiary class, not BR generic creature category.', [source(helper) for helper in helpers.values()] + ([class_fact['source']] if class_fact else []))
        else:
            deferred.append({'field': 'bestiary', 'reason': 'INCOMPLETE_POSITIVE_WIKI_BESTIARY_FACTS'})
    else:
        deferred.append({'field': 'bestiary', 'reason': 'OPTIONAL_APPLICABILITY_UNPROVEN_WITHOUT_POSITIVE_WIKI_BESTIARY_FACTS'})
    for key in ('plural', 'article'):
        raw = fields.get(key)
        if isinstance(raw, str) and raw and '{{' not in raw and '[[' not in raw and '?' not in raw:
            if 'name_forms' in document['creature']:
                emit('/creature/name_forms/' + key, raw, [key], 'Explicit wiki name form; no fabricated inflection.')
            else:
                emit('/creature/name_forms', {key: raw}, [key], 'Explicit wiki name form; no fabricated inflection.')
    deferred.append({'field': 'bosstiary', 'reason': 'NO_ACTOR_SPECIFIC_WIKI_CATEGORY_AND_STAGE_PROOF_IN_CAPTURE'})
    return patches, deferred, resolved


def generate(baseline, audit, output):
    baseline, audit, output = Path(baseline), Path(audit), Path(output)
    if digest(baseline / 'population-index.json') != BASELINE_SHA:
        raise ValueError('Baseline population index digest mismatch')
    wiki = audit / 'wiki'
    marker = load(wiki / 'br-final.json')
    if not marker['complete'] or marker['files_sha256']['pages.json.gz'] != BR_SHA:
        raise ValueError('Required locked WikiBR marker missing')
    for name, expected in marker['files_sha256'].items():
        if digest(wiki / name) != expected:
            raise ValueError('Wiki capture drift: ' + name)
    pages = {normalize(page['page_title']): page for page in load(wiki / 'pages.json.gz')['pages']}
    coverage = {row['monster']: row for row in load(wiki / 'actor-page-coverage.json.gz')['actors']}
    audit_summary = load(audit / 'stats/summary.json')
    variants = set(audit_summary['shared_title_variant_unverified_actors'])
    helpers = {}
    template = pages.get(normalize('Predefinição:Infobox Criatura'))
    if template:
        helpers['br_template'] = template
    en_path = wiki / 'en-pages.json.gz'
    if en_path.exists():
        for page in load(en_path)['pages']:
            if page['page_title'] == 'Bestiary/Difficulties' and page['revision_id'] == 1132172:
                helpers['difficulty'] = {**page, 'fields': {}, 'field_lines': {}}
    class_facts = {}
    samples = Path(__file__).resolve().parents[1] / 'content-schema/monster-authoring/samples'
    canonical_classes = {normalize(pair[0]): pair for pair in CLASSES.values()}
    for name in ['wiki-population-2026-09-27.json', 'wiki-population-crystal-00ce02a5-2026-09-27.json']:
        for record in load(samples / name)['monsters']:
            for row in record.get('rows', []):
                pair = canonical_classes.get(normalize(row.get('wiki'))) if row['field'] == 'bestiary.class' else None
                if pair and record.get('cut_revision_id') and record.get('cut_content_sha256') and row.get('wiki_line'):
                    class_facts[record['monster']] = {'class_pair': pair, 'source': {'url': record['page_url'], 'revision_id': record['cut_revision_id'], 'content_sha256': record['cut_content_sha256'], 'method': 'REPOSITORY_ARCHIVED_ACTUAL_FANDOM_FIELD_OBSERVATION', 'source_file': record.get('cut_title', record['monster']), 'source_line': row['wiki_line'], 'source_field': 'bestiaryclass', 'raw': row.get('wiki_raw')}}
    patches, qualifications, flags, resolved = [], {}, {}, {}
    index = load(baseline / 'population-index.json')
    for actor in index['monsters']:
        slug = actor['monster']
        document = load(baseline / 'bundles' / slug / 'monster.json')
        record = coverage[slug]
        if record['status'] != 'FRESH_PAGE_CAPTURED':
            qualifications[slug] = [{'field': '*', 'reason': record['status']}]
            flags[slug] = ['WIKI_FIELDS_UNVERIFIED']
            continue
        page = pages[normalize(record['resolved_title'])]
        result, deferred, removed = patch_actor(slug, document, page, variant=slug in variants or 'SOURCE_DERIVED_PRIMAL_VARIANT' in actor.get('completion_flags', []), estimated='OWNER_ACCEPTED_NON_GLOBAL_ESTIMATE' in actor.get('completion_flags', []), helpers=helpers, class_fact=class_facts.get(slug))
        patches.extend(result)
        qualifications[slug] = deferred
        flags[slug] = ['WIKI_FIELDS_PARTIALLY_VERIFIED'] if deferred else ['WIKI_FIELDS_VERIFIED_WITHIN_CAPTURE_SCOPE']
        if removed:
            resolved[slug] = removed
    output.mkdir(parents=True, exist_ok=True)
    packet = {'schema': 'OTERYN_MONSTER_FIELD_PATCH/v1', 'lane': 'stats', 'baseline_index_sha256': BASELINE_SHA, 'patches': patches, 'actor_flags': flags, 'resolved_actor_flags': resolved, 'source_snapshot_sha256': marker['files_sha256'], 'supporting_snapshot_sha256': {'en-pages.json.gz': digest(en_path), **{name: digest(samples / name) for name in ['wiki-population-2026-09-27.json', 'wiki-population-crystal-00ce02a5-2026-09-27.json']}}, 'global_parity_asserted': False}
    (output / 'completion.json').write_text(json.dumps(packet, ensure_ascii=False, indent=2) + '\n')
    with gzip.open(output / 'qualifications.json.gz', 'wt') as stream:
        json.dump(qualifications, stream, ensure_ascii=False)
    summary = {'population': len(index['monsters']), 'patches': len(patches), 'changed_actors': len({row['monster'] for row in patches}), 'patch_pointer_counts': dict(Counter(row['pointer'] for row in patches)), 'resolved_estimate_actors': sorted(resolved), 'qualified_variant_actors_preserved': len(variants), 'packet_sha256': digest(output / 'completion.json'), 'source_snapshot_sha256': marker['files_sha256'], 'baseline_index_sha256': BASELINE_SHA}
    (output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', required=True)
    parser.add_argument('--audit', required=True)
    parser.add_argument('--output', required=True)
    arguments = parser.parse_args()
    print(json.dumps(generate(arguments.baseline, arguments.audit, arguments.output)))
