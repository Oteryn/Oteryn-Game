"""Validate reward claims, door gates and the quest catalogue: JSON Schema plus the semantic rules of the format.

Usage: python validate_quest_content.py CLAIMS.json QUESTS.json [--catalog CATALOG.json] [--manifest MANIFEST.json]
                                        [--gates GATES.json [--gates-manifest MANIFEST.json]]
                                        [--progress PROGRESS.json]   (QUESTS.json is then the whole catalogue, gates required)
                                        [--interactions INTERACTIONS.json --interactions-manifest MANIFEST.json]
                                                                     (progress required)
Prints one JSON report; the exit code is 1 when the documents are invalid.
"""
import argparse
import json
import sys
from pathlib import Path

import jsonschema

ROOT = Path(__file__).resolve().parent
MANIFEST_STATUS = ('mapped', 'conflict', 'approved_omission', 'unresolved_semantics')
# D36: children whose owner has no accepted contract stay in the definition, blocked with this reason.
# D37 (relocation) and D38 (world-object overlay) now name the owner (the scope runtime); what stays
# blocked under these two reasons is narrower: a Movement child whose target is computed rather than a
# named anchor (out of scope per proposal §3, until a DUR-04 component or anchor can name it), and a
# WorldObject child whose source call the converter has not yet classified into a D38 operation kind.
BLOCKED = {'Movement': 'computed relocation target: no anchor named (GAME-INTERACTION-01 §19.3; D37 owner accepted)',
           'WorldObject': 'world-object operation kind not yet classified from source (D38 owner accepted; re-transcription pending)'}
# a scheduled revert (addEvent(Position.revertItem, delay, ...)) whose delay is not itself a positive
# literal (non-literal, or a literal <= 0 -- the schema requires revert_after_ms >= 1) fails closed: it
# never merges silently into the operation it would revert without a recorded revert_after_ms, so it
# stays its own blocked WorldObject child with this distinct reason.
BLOCKED_SCHEDULED_REVERT_DELAY = 'scheduled revert (addEvent) has a non-literal delay; revert_after_ms cannot be recorded without one'
# a second scheduled revert that provably targets an operation which already carries a revert_after_ms
# (from an earlier one, in source order) never overwrites it -- the operation's actual revert delay
# cannot be inferred from two conflicting schedules -- so it stays its own blocked WorldObject child too.
BLOCKED_DUPLICATE_SCHEDULED_REVERT = 'a second scheduled revert names the same already-reverted operation; revert_after_ms is not overwritten'
BLOCKED_REASONS = {'Movement': {BLOCKED['Movement']},
                   'WorldObject': {BLOCKED['WorldObject'], BLOCKED_SCHEDULED_REVERT_DELAY, BLOCKED_DUPLICATE_SCHEDULED_REVERT}}


def refs(value):
    if isinstance(value, dict):
        if set(value) == {'family', 'key', 'revision'}:
            yield value
            return
        for child in value.values():
            yield from refs(child)
    elif isinstance(value, list):
        for child in value:
            yield from refs(child)


def committed_text(value, where=''):
    """Paths that carry narrative text; LICENSE-ASSETS.md reserves it, so only text references are committed."""
    if isinstance(value, dict):
        for key, child in value.items():
            if key in ('text', 'strings'):
                yield f'{where}/{key}'
            else:
                yield from committed_text(child, f'{where}/{key}')
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from committed_text(child, f'{where}/{index}')


def validate(claims_doc, quests_doc, catalog=None, manifest=None):
    schema = json.loads((ROOT / 'quest_content.schema.json').read_text())
    validator = jsonschema.Draft202012Validator(schema)
    errors = [f'{path}: narrative text must not be committed (use text_ref)'
              for doc in (claims_doc, quests_doc) for path in committed_text(doc)]
    errors += [f'{"/".join(map(str, e.absolute_path))}: {e.message}'
               for doc in (claims_doc, quests_doc) for e in validator.iter_errors(doc)]
    if errors:
        return errors
    claims, quests = claims_doc['claims'], quests_doc['quests']
    claim_keys, quest_keys, positions = set(), set(), {}
    for claim in claims:
        key = claim['identity']['key']
        if key in claim_keys:
            errors.append(f'{key}: duplicate claim key')
        claim_keys.add(key)
        for placement in claim['placements']:
            reward = placement['reward']
            if not reward['items'] and not reward.get('random_one_of'):
                errors.append(f'{key}: a placement hands out nothing')
            position = tuple(placement['position'].values())
            if position in positions:
                errors.append(f'{key}: position {position} already belongs to {positions[position]}')
            positions[position] = key
            text = reward.get('written_text')
            carriers = [q['item'] for q in reward['items']] + ([reward['container']] if reward.get('container') else [])
            if text and text['item'] and text['item'] not in carriers:
                errors.append(f'{key}: written text names an item the chest does not hand out')
        if (claim['quest'] is None) != (claim['quest_link_basis'] is None):
            errors.append(f'{key}: quest and quest_link_basis must be set together')
        if claim['quest'] and claim['quest_candidate_from_section']:
            errors.append(f'{key}: a linked claim has no section candidate')
    paths = {}
    for quest in quests:
        key = quest['identity']['key']
        if key in quest_keys:
            errors.append(f'{key}: duplicate quest key')
        quest_keys.add(key)
        path = key.split(':', 1)[1]
        if paths.setdefault(path, key) != key:
            errors.append(f'{key}: the same quest is also {paths[path]} (one identity per quest across namespaces)')
        for claim_ref in quest['claims']:
            if claim_ref['family'] != 'RewardClaim' or claim_ref['key'] not in claim_keys:
                errors.append(f'{key}: unknown claim {claim_ref["key"]}')
    linked = {(c['identity']['key'], c['quest']['key']) for c in claims if c['quest']}
    listed = {(r['key'], q['identity']['key']) for q in quests for r in q['claims']}
    for claim_key, quest_key in sorted(linked ^ listed):
        errors.append(f'{claim_key}: claim and quest {quest_key} do not point at each other')
    if catalog is not None:
        known = {(d['family'], d['key']) for d in catalog['definitions']}
        for r in refs(claims):
            if r['family'] in ('Item', 'Achievement') and (r['family'], r['key']) not in known:
                errors.append(f'{r["key"]}: {r["family"]} reference missing from the catalog')
    if manifest is not None:
        for entry in manifest['entries']:
            if entry['status'] not in MANIFEST_STATUS:
                errors.append(f'manifest: unknown status {entry["status"]}')
            if entry.get('destination') and entry['destination'] not in claim_keys:
                errors.append(f'manifest: destination {entry["destination"]} is not a claim')
            if entry['status'] in ('mapped', 'conflict') and not entry.get('destination'):
                errors.append(f'manifest: {entry["status"]} entry at {entry.get("position")} has no destination')
        mapped = {e['destination'] for e in manifest['entries'] if e.get('destination')}
        for key in sorted(claim_keys - mapped):
            errors.append(f'{key}: no manifest entry maps a source chest to this claim')
    return sorted(set(errors))


def validate_gates(gates_doc, claims_doc, manifest=None):
    schema = json.loads((ROOT / 'quest_content.schema.json').read_text())
    errors = [f'{"/".join(map(str, e.absolute_path))}: {e.message}'
              for e in jsonschema.Draft202012Validator(schema).iter_errors(gates_doc)]
    if errors:
        return errors
    claims = {c['identity']['key']: c for c in claims_doc['claims']}
    keys, positions = set(), {}
    for gate in gates_doc['gates']:
        key, condition = gate['identity']['key'], gate['condition']
        if key in keys:
            errors.append(f'{key}: duplicate gate key')
        keys.add(key)
        if (gate['quest'] is None) != (gate['quest_link_basis'] is None):
            errors.append(f'{key}: quest and quest_link_basis must be set together')
        if (gate['state'] == 'shared_lock') != (condition['kind'] in ('door_key', 'lever')):
            errors.append(f'{key}: only a key or lever door has a shared lock')
        for placement in gate['placements']:
            position = tuple(placement['position'].values())
            if position in positions:
                errors.append(f'{key}: position {position} already belongs to {positions[position]}')
            positions[position] = key
        if condition['kind'] == 'quest_progress' and condition['claim']:
            claim = claims.get(condition['claim']['key'])
            if claim is None:
                errors.append(f'{key}: unknown claim {condition["claim"]["key"]}')
            elif claim['identity']['key'].split('/', 1)[1] != condition['progress'].split('/', 1)[1]:
                errors.append(f'{key}: the claim does not record the progress the door reads')
        if condition['kind'] == 'door_key':
            number = condition['key_binding'].rsplit('/', 1)[1]
            for claim_ref in condition['key_from_claims']:
                claim = claims.get(claim_ref['key'])
                bindings = {p['reward'].get('key_binding', '').rsplit('/', 1)[-1] for p in claim['placements']} if claim else set()
                if number not in bindings:
                    errors.append(f'{key}: {claim_ref["key"]} hands out no key for this door')
    if manifest is not None:
        for entry in manifest['entries']:
            if entry['status'] not in MANIFEST_STATUS:
                errors.append(f'manifest: unknown status {entry["status"]}')
            if entry.get('destination') and entry['destination'] not in keys:
                errors.append(f'manifest: destination {entry["destination"]} is not a gate')
            if entry['status'] in ('mapped', 'conflict') and not entry.get('destination'):
                errors.append(f'manifest: {entry["status"]} entry at {entry.get("position")} has no destination')
        mapped = {e['destination'] for e in manifest['entries'] if e.get('destination')}
        for key in sorted(keys - mapped):
            errors.append(f'{key}: no manifest entry maps a source door to this gate')
    return sorted(set(errors))


def validate_storylines(quests_doc, gates_doc, progress_doc):
    """Missions, stage values and the progress tracks of storyline quests; claim links are checked by validate()."""
    schema = json.loads((ROOT / 'quest_content.schema.json').read_text())
    errors = [f'{"/".join(map(str, e.absolute_path))}: {e.message}'
              for e in jsonschema.Draft202012Validator(schema).iter_errors(quests_doc)]
    if errors:
        return errors
    gate_keys = {g['identity']['key'] for g in gates_doc['gates']}
    tracks = {t['key']: t for t in progress_doc['progress']}
    quest_keys = {q['identity']['key'] for q in quests_doc['quests']}
    for track in progress_doc['progress']:
        if 'auxiliary_of' in track:
            if track['missions'] or track['start_of']:
                errors.append(f'{track["key"]}: an auxiliary track is no mission or start track')
            for owner in sorted(set(track['auxiliary_of']) - quest_keys):
                errors.append(f'{track["key"]}: auxiliary of unknown quest {owner}')
    for quest in quests_doc['quests']:
        key = quest['identity']['key']
        for gate_ref in quest.get('gates', []):
            if gate_ref['family'] != 'Gate' or gate_ref['key'] not in gate_keys:
                errors.append(f'{key}: unknown gate {gate_ref["key"]}')
        if quest['kind'] != 'storyline':
            continue
        start = quest['start']
        if start and key not in tracks.get(start['progress'], {}).get('start_of', []):
            errors.append(f'{key}: start track {start["progress"]} does not name this quest')
        seen = set()
        for mission in quest['missions']:
            where = f'{key}#{mission["key"]}'
            if mission['key'] in seen:
                errors.append(f'{where}: duplicate mission key')
            seen.add(mission['key'])
            if mission['start_value'] > mission['end_value']:
                errors.append(f'{where}: start value above end value')
            if mission['journal']['kind'] == 'per_stage':
                values = [stage['value'] for stage in mission['journal']['stages']]
                if values != sorted(set(values)):
                    errors.append(f'{where}: stage values are not unique and ascending')
                if any(v < mission['start_value'] or v > mission['end_value'] for v in values):
                    errors.append(f'{where}: a stage lies outside the mission range')
            track = tracks.get(mission['progress'], {})
            if where not in track.get('missions', []):
                errors.append(f'{where}: progress track {mission["progress"]} does not list this mission')
            for transition in mission['transitions']:
                if ('requested_by' in transition) != (transition['owner'] == 'npc'):
                    errors.append(f'{where}: transition {transition["key"]} names NPC dialogue without an NPC owner or the reverse')
            keys = [t['key'] for t in mission['transitions']]
            if len(keys) != len(set(keys)):
                errors.append(f'{where}: duplicate transition key')
            evidence = {t['key'] for t in track.get('transitions', [])}
            for transition in sorted(set(keys) - evidence):
                errors.append(f'{where}: transition {transition} has no source evidence on its progress track')
    return sorted(set(errors))


def rule_leaves(rules):
    """(children, leaf conditions) of a rule tree."""
    children, conditions = [], []

    def leaves(condition):
        if 'all' in condition or 'any' in condition:
            for term in condition.get('all', condition.get('any')):
                leaves(term)
        else:
            conditions.append(condition)

    def walk(items):
        for rule in items:
            if 'branch' in rule:
                for branch in rule['branch']:
                    leaves(branch['when'])
                    walk(branch['then'])
                walk(rule.get('otherwise', []))
            else:
                children.append(rule)
    walk(rules)
    return children, conditions


def validate_interactions(interactions_doc, manifest, quests_doc, progress_doc):
    """Interaction definitions (D36): anchors, blocked owners, named quest transitions and manifest coverage."""
    schema = json.loads((ROOT / 'interaction.schema.json').read_text())
    errors = [f'{"/".join(map(str, e.absolute_path))}: {e.message}'
              for e in jsonschema.Draft202012Validator(schema).iter_errors(interactions_doc)]
    if errors:
        return errors
    tracks = {t['key'] for t in progress_doc['progress']}
    missions = {f'{q["identity"]["key"]}#{m["key"]}': m for q in quests_doc['quests'] for m in q.get('missions', [])}
    unresolved, seen, undeclared = {}, set(), set()
    for interaction in interactions_doc['interactions']:
        key = interaction['identity']['key']
        if key in seen:
            errors.append(f'{key}: duplicate interaction key')
        seen.add(key)
        anchors = [a['key'] for a in interaction['anchors']]
        positions = [tuple(a['source_position'].values()) for a in interaction['anchors']]
        if len(anchors) != len(set(anchors)) or len(positions) != len(set(positions)):
            errors.append(f'{key}: anchor keys and positions must be unique')
        children, conditions = rule_leaves(interaction['rules'])
        used = {c.get('anchor') for c in children}
        used |= {c['target'].get('anchor') for c in children
                if c.get('owner') == 'Movement' and c.get('request') == 'relocate' and c['target']['kind'] == 'anchor'}
        used -= {None}
        for anchor in sorted(used - set(anchors)):
            errors.append(f'{key}: unknown anchor {anchor}')
        # an anchor with no current consumer is not an error: anchors are transcription evidence (a
        # source position kept for the re-run that binds it), and a child that once referenced one can
        # be reclassified as blocked under a stricter typing rule without that evidence being deleted.
        for child in children:
            if child.get('status') == 'blocked' and child['owner'] in BLOCKED_REASONS and child['reason'] not in BLOCKED_REASONS[child['owner']]:
                errors.append(f'{key}: {child["owner"]} child is blocked for an unknown reason')
            if child['owner'] == 'Quest' and child['request'] == 'set_progress':
                if child['progress'] not in tracks:
                    undeclared.add(child['progress'])
                if 'transition' in child:
                    mission_key, transition = child['transition'].rsplit(':', 1)
                    mission = missions.get(mission_key)
                    if not mission or transition not in {t['key'] for t in mission['transitions']}:
                        errors.append(f'{key}: unknown transition {child["transition"]}')
                    elif mission['progress'] != child['progress']:
                        errors.append(f'{key}: transition {child["transition"]} moves another progress track')
        lines = [u['line'] for u in interaction['unresolved']]
        if len(lines) != len(set(lines)):
            errors.append(f'{key}: an unresolved line is listed twice')
        unresolved[key] = bool(lines) or any('unresolved' in c for c in conditions)
    if undeclared != set(manifest.get('undeclared_progress_tracks', [])):
        errors.append('manifest: undeclared_progress_tracks does not list exactly the tracks outside the catalogue')
    covered = set()
    for entry in manifest['entries']:
        destination, status = entry['destination'], entry['status']
        if status not in MANIFEST_STATUS:
            errors.append(f'manifest: unknown status {status}')
        if destination not in unresolved:
            errors.append(f'manifest: destination {destination} is not an interaction')
            continue
        if destination in covered:
            errors.append(f'manifest: {destination} is listed twice')
        covered.add(destination)
        if status == 'mapped' and unresolved[destination]:
            errors.append(f'manifest: {destination} is mapped but keeps unresolved lines or conditions')
        if status == 'conflict' and len({s['source'] for s in entry['sources']}) < 2:
            errors.append(f'manifest: {destination} is a conflict with one source')
    for key in sorted(set(unresolved) - covered):
        errors.append(f'{key}: no manifest entry maps a source script to this interaction')
    return sorted(set(errors))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('claims', type=Path)
    parser.add_argument('quests', type=Path)
    parser.add_argument('--catalog', type=Path)
    parser.add_argument('--manifest', type=Path)
    parser.add_argument('--gates', type=Path)
    parser.add_argument('--gates-manifest', type=Path)
    parser.add_argument('--progress', type=Path)
    parser.add_argument('--interactions', type=Path)
    parser.add_argument('--interactions-manifest', type=Path)
    args = parser.parse_args()
    if args.progress and not args.gates:
        parser.error('--progress needs --gates')
    if args.interactions and not (args.progress and args.interactions_manifest):
        parser.error('--interactions needs --progress and --interactions-manifest')
    load = lambda p: json.loads(p.read_text()) if p else None
    errors = validate(load(args.claims), load(args.quests), load(args.catalog), load(args.manifest))
    if args.gates:
        errors += validate_gates(load(args.gates), load(args.claims), load(args.gates_manifest))
    if args.progress:
        errors += validate_storylines(load(args.quests), load(args.gates), load(args.progress))
    if args.interactions:
        errors += validate_interactions(load(args.interactions), load(args.interactions_manifest), load(args.quests),
                                        load(args.progress))
    print(json.dumps({'valid': not errors, 'errors': errors[:50], 'error_count': len(errors)}, indent=2))
    sys.exit(1 if errors else 0)


if __name__ == '__main__':
    main()
