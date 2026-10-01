#!/usr/bin/env python3
"""Quest interaction gap triage: classify every unresolved interaction line and unresolved
condition of `samples/interactions/interactions.json` into exactly one bucket.

- `owner_pending`: the unresolved item already names a missing runtime owner (D36's own five
  reasons: kv write, condition, boss cooldown, creature removal, delayed callback/scheduler), or
  is the same statement/condition shape under a different reason label (a custom KV wrapper's
  `:set`/`:get` instead of `player:kv()`; `canFightBoss`/`setBossCooldown` under a reason other
  than "boss cooldown"; `addCondition`/`removeCondition` likewise) -- resolves mechanically once
  that owner exists, with no new engine mechanism to design.
- `shared_mechanism`: a recurring multi-script pattern, mined from the pinned sources and accepted
  only at >=3 quests with an explicit, non-guessing call-shape rule (`MECHANISMS` below); each
  names the engine primitive already used identically across those scripts.
- `bespoke`: everything else. A line is never forced into 1 or 2.

Reads the committed samples plus the pinned Canary/CrystalServer checkouts (for the line text the
manifest's `unresolved` entries reference; the manifest's own source order already prefers Canary
when an interaction exists in both -- `sources[0]` always matches the interaction's own key
prefix). Quests are joined to interactions the same way `ots_readiness.py` does (imported, not
reimplemented). Deterministic; no narrative text from the sources is ever written out -- only line
numbers and the call names the rules themselves match on.

Usage: python ots_gap_triage.py --canary <opentibiabr/canary at 04b83b51> --crystal <zimbadev/crystalserver at 9f5a72c6>
"""
import argparse
import collections
import json
import pathlib
import re

import ots_readiness as readiness

HERE = pathlib.Path(__file__).resolve().parent
SAMPLES = HERE / 'samples'
OUT = SAMPLES / 'gap-triage' / 'triage.json'

# The five D36 reasons that already name a missing runtime owner (ots_interactions.py's own
# vocabulary; kept in sync with ots_readiness.py's REASON_FEATURE).
REASON_OWNER = {
    'kv write without an accepted owner': 'kv_state',
    'condition without an accepted owner': 'condition',
    'boss cooldown without an accepted owner': 'boss_cooldown',
    'creature removal without an accepted owner': 'creature_removal',
    'delayed callback (addEvent) without a scheduler owner': 'scheduler',
}

# Same-owner extensions: a statement or condition matching one of these call shapes needs the
# same owner as an already-named reason above, even though the item itself carries a different
# (or no) reason -- because it is the identical action under a spelling the D36 regexes for that
# reason do not happen to cover. Tried in order; each entry is (owner, compiled regex). Verified
# against samples/interactions before being accepted (see the task report).
OWNER_EXTENSIONS = [
    # a custom per-quest KV wrapper (`CakeQuest.KV:set(...)`, `SoulWarQuest.kvSoulWar:get(...)`,
    # `soulWarKV:set(...)`) -- same persistent per-player/global state need as `player:kv()`,
    # which ots_interactions.py's KV_WRITE only matches on the literal identifier `kv`.
    ('kv_state', re.compile(r':kv\(\)|\w*[Kk][Vv]\w*[.:](get|set|remove)\(')),
    # `canFightBoss` is the read side of the same boss-cooldown system `setBossCooldown` writes;
    # ots_interactions.py's BOSS_COOLDOWN only matches the write.
    ('boss_cooldown', re.compile(r':canFightBoss\(|:setBossCooldown\(')),
    # `addCondition`/`removeCondition` under a reason other than "condition without an accepted
    # owner" (a loop or function-literal body the top-level reason label does not reach).
    ('condition', re.compile(r':(addCondition|removeCondition)\(')),
]

# Shared-mechanism candidates: each rule matches an already-shared engine primitive (a global
# library function or class both servers define once) used identically across >=3 quests. Every
# candidate below was checked against the actual Lua definitions and call sites on the pinned
# checkouts before acceptance; rejected candidates are recorded in `REJECTED_MECHANISMS` with why.
MECHANISMS = {
    'boss_portal_spectator_gate': {
        'definition': ('A step-in mini-boss portal: the `Spectators()` builder screens the room '
                        '(setOnlyPlayer/setCheckPosition/check), gates entry on cooldown and level, '
                        'teleports the player in, spawns the boss, and later removes players and '
                        'monsters (removeMonsters/removePlayers/clearCreaturesCache) once the fight '
                        'window (addEvent) elapses.'),
        'rule': r'\bSpectators\(\)|:(setOnlyPlayer|setRemoveDestination|setCheckPosition|check|'
                r'removeMonsters|removePlayers|clearCreaturesCache)\(',
        'owner_lane': ('encounter/boss-room entry -- not yet in '
                        'OTERYN_ENCOUNTER_AUTHORING_FORMAT_V1.md SS1-3; GAME-AI-01 covers the spawn '
                        'half only'),
    },
    'kill_reward_fanout': {
        'definition': ('A creature-death callback delegates to the shared `onDeathForDamagingPlayers`'
                        '/`onDeathForParty` library helper, which the engine already fans out to every '
                        'damaging player or the whole party; only the per-player callback body is '
                        "quest-specific (it is transcribed on its own, statement by statement)."),
        'rule': r'\bon(DeathForDamagingPlayers|DeathForParty)\(',
        'owner_lane': ('no owner yet fans a single kill out to multiple credited players/party '
                        'members; sits above the existing per-effect owners (Quest/Item/Experience/'
                        'Achievement), which the per-player body still delegates to once expanded'),
    },
    'boss_room_entry_gate': {
        'definition': ('The shared `roomIsOccupied`/`Player:doCheckBossRoom` library checks (both '
                        'servers define each once in data/libs/functions/functions.lua) gate a lever '
                        'or portal on room occupancy/an active boss before the encounter opens.'),
        'rule': r'\b(roomIsOccupied|doCheckBossRoom)\(',
        'owner_lane': 'encounter/boss-room entry -- same open lane as boss_portal_spectator_gate',
    },
}
REJECTED_MECHANISMS = [
    {'name': 'boss-room lever tile-scan + kickerPlayerRoomAfterMin',
     'reason': ('the exact shape the task named (players on N tiles scanned in a loop, teleported, '
                'kicked after a timer via the shared `kickerPlayerRoomAfterMin` library function) is '
                'real (cults_of_tibia/actions_bosses_levers.lua, repeated per boss lever) but occurs '
                'in exactly 1 quest on the pinned sources; below the >=3-quest bar.')},
    {'name': 'area cleanup on exit (isPlayer -> teleport, isMonster -> remove, in a bounded room)',
     'reason': ('real in heart_of_destruction (`clearArea`, 3 lever scripts, all reached from the '
                'transcribed callback), but the similarly-named helpers in hero_of_rathleton, '
                'the_order_of_lion and a_pirates_tail_quest either do something else entirely '
                '(item-only cleanup) or their call site is never reached by the converter at all -- '
                'no explicit call-shape rule reaches a second quest without guessing.')},
    {'name': 'generic getSpectators()/isInRange() room-occupancy loops',
     'reason': ('used in 60+ quest scripts for wildly different purposes (damage tallies, area '
                'combat, teleport screening, puzzle triggers); no single call shape distinguishes '
                '"room occupancy check" from the rest without guessing at intent.')},
    {'name': 'combat damage application (doTargetCombatHealth/doAreaCombatHealth)',
     'reason': ('recurring (~15 quests) but each call carries its own formula/area/target selection; '
                'there is no accepted owner for scripted combat damage to delegate to, so this is a '
                'new-owner question for a later task, not a shared mechanism -- stays bespoke.')},
    {'name': 'random reward/position picks (`list[math.random(#list)]`)',
     'reason': 'only 2 quests on the pinned sources; below the >=3-quest bar.'},
    {'name': 'timed world-object revert (revert_after_ms)',
     'reason': 'already decided (D38 SS7); not a triage candidate.'},
]


def load(rel, key):
    return json.loads((SAMPLES / rel).read_text())[key]


def source_lines(roots, server, path, cache={}):
    key = (server, path)
    if key not in cache:
        cache[key] = (roots[server] / path).read_text(errors='replace').splitlines()
    return cache[key]


def line_text(roots, manifest_by_key, interaction_key, lineno):
    entry = manifest_by_key[interaction_key]
    src = entry['sources'][0]
    lines = source_lines(roots, src['source'], src['path'])
    return lines[lineno - 1] if 1 <= lineno <= len(lines) else None


def classify(text, reason):
    """(bucket, owner_or_mechanism) for one unresolved item. `reason` is None for a condition."""
    if reason in REASON_OWNER:
        return 'owner_pending', REASON_OWNER[reason]
    if text:
        for owner, pattern in OWNER_EXTENSIONS:
            if pattern.search(text):
                return 'owner_pending', owner
        for name, spec in MECHANISMS.items():
            if re.search(spec['rule'], text):
                return 'shared_mechanism', name
    return 'bespoke', None


def condition_lines(rules):
    """Yield every condition-position unresolved line (recurses `then` and `otherwise`)."""
    for kind, condition in readiness.walk_rules(rules):
        if kind == 'cond' and 'unresolved' in condition:
            yield condition['unresolved']['line']


def build_quest_join():
    """Interaction key -> set of catalogue quest keys, exactly as ots_readiness.py joins them."""
    quests = load('questlog/quests.json', 'quests')
    progress = load('questlog/progress.json', 'progress')
    interactions = load('interactions/interactions.json', 'interactions')
    quest_by_key = {q['identity']['key']: q for q in quests}
    joined = readiness.join_interactions(quests, progress, interactions)
    joined_by_interaction = {key: owners for key, owners in joined.items() if owners}
    unlinked = [key for key, owners in joined.items() if not owners]
    return joined_by_interaction, unlinked, quest_by_key


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--canary', required=True, type=pathlib.Path)
    parser.add_argument('--crystal', required=True, type=pathlib.Path)
    parser.add_argument('--out', type=pathlib.Path, default=OUT)
    args = parser.parse_args()
    roots = {'canary': args.canary, 'crystalserver': args.crystal}

    interactions = load('interactions/interactions.json', 'interactions')
    manifest = load('interactions/manifest.json', 'entries')
    manifest_by_key = {e['destination']: e for e in manifest}
    joined_by_interaction, unlinked, quest_by_key = build_quest_join()

    per_quest = {key: {'kind': q['kind'], 'owner_pending': 0, 'shared_mechanism': 0, 'bespoke': 0,
                       'owners': collections.Counter(), 'mechanisms': collections.Counter()}
                 for key, q in quest_by_key.items()}
    totals = {'owner_pending': 0, 'shared_mechanism': 0, 'bespoke': 0}
    owner_totals, mechanism_totals = collections.Counter(), collections.Counter()
    owner_quests, mechanism_quests = collections.defaultdict(set), collections.defaultdict(set)
    owner_interactions, mechanism_interactions = collections.defaultdict(set), collections.defaultdict(set)

    def record(interaction_key, bucket, tag):
        totals[bucket] += 1
        quests = joined_by_interaction.get(interaction_key, set())
        for quest_key in quests:
            row = per_quest[quest_key]
            row[bucket] += 1
            if tag:
                (row['owners'] if bucket == 'owner_pending' else row['mechanisms'])[tag] += 1
        if bucket == 'owner_pending':
            owner_totals[tag] += 1
            owner_quests[tag] |= quests
            owner_interactions[tag].add(interaction_key)
        elif bucket == 'shared_mechanism':
            mechanism_totals[tag] += 1
            mechanism_quests[tag] |= quests
            mechanism_interactions[tag].add(interaction_key)

    condition_total = blocked_total = reference_total = 0
    declared = {t['key'] for t in load('questlog/progress.json', 'progress')}
    for inter in interactions:
        key = inter['identity']['key']
        for item in inter['unresolved']:
            text = line_text(roots, manifest_by_key, key, item['line'])
            bucket, tag = classify(text, item['reason'])
            record(key, bucket, tag)
        for lineno in condition_lines(inter['rules']):
            condition_total += 1
            text = line_text(roots, manifest_by_key, key, lineno)
            bucket, tag = classify(text, None)
            record(key, bucket, tag)

        # Blocked typed children and absent predicates are transcription gaps,
        # not missing runtime owners. They have no unresolved source-line record.
        for kind, value in readiness.walk_rules(inter['rules']):
            if kind == 'child' and value.get('status') == 'blocked':
                blocked_total += 1
                record(key, 'bespoke', None)
            stage = value.get('quest_stage') if kind == 'cond' else None
            track = stage['progress'] if stage else value.get('progress') if kind == 'child' else None
            if track and track not in declared:
                reference_total += 1
                record(key, 'bespoke', None)

    unresolved_lines_total = sum(len(i['unresolved']) for i in interactions)
    assert sum(totals.values()) == unresolved_lines_total + condition_total + blocked_total + reference_total

    quests_rows = []
    for key in sorted(per_quest):
        row = per_quest[key]
        to_finish = sorted(f'owner:{o}' for o in row['owners']) + sorted(f'mechanism:{m}' for m in row['mechanisms'])
        if row['bespoke']:
            to_finish.append(f"bespoke:{row['bespoke']}")
        quests_rows.append({
            'quest': key, 'kind': row['kind'],
            'counts': {'owner_pending': row['owner_pending'], 'shared_mechanism': row['shared_mechanism'],
                       'bespoke': row['bespoke']},
            'owners_needed': sorted(row['owners']), 'mechanisms_needed': sorted(row['mechanisms']),
            'to_finish': to_finish,
        })

    owners_ranked = [{'owner': o, 'lines': owner_totals[o], 'interactions': len(owner_interactions[o]),
                      'quests': len(owner_quests[o])}
                     for o in sorted(owner_totals, key=lambda o: (-len(owner_quests[o]), o))]
    mechanisms_ranked = [{'mechanism': m, 'lines': mechanism_totals[m], 'interactions': len(mechanism_interactions[m]),
                          'quests': len(mechanism_quests[m]), 'definition': MECHANISMS[m]['definition'],
                          'rule': MECHANISMS[m]['rule'], 'owner_lane': MECHANISMS[m]['owner_lane']}
                         for m in sorted(mechanism_totals, key=lambda m: (-len(mechanism_quests[m]), m))]

    # Greedy unlock order over owners+mechanisms only (bespoke gaps never close this way; tracked
    # per step as the quests that would still have bespoke work left).
    need = {}
    for row in quests_rows:
        tokens = {f'owner:{o}' for o in row['owners_needed']} | {f'mechanism:{m}' for m in row['mechanisms_needed']}
        if tokens:
            need[row['quest']] = tokens
    bespoke_flag = {row['quest']: bool(row['counts']['bespoke']) for row in quests_rows}
    built, order = set(), []
    while any(need[q] - built for q in need):
        remaining = sorted({t for q in need for t in need[q] - built})

        def gain(token):
            return sum(1 for q in need if need[q] - built and need[q] <= built | {token})
        best = max(remaining, key=lambda t: (gain(t), sum(t in need[q] for q in need), t))
        built.add(best)
        complete = [q for q in need if need[q] <= built]
        order.append({'adds': best,
                      'quests_owner_mechanism_complete': len(complete),
                      'of_which_still_have_bespoke_gaps': sum(1 for q in complete if bespoke_flag[q])})

    report = {
        'classification': 'DERIVED from committed samples and the pinned Canary/CrystalServer checkouts',
        'source_revisions': {'canary': 'opentibiabr/canary@04b83b512114bfd888000d6e1433ed8ecaec7c5b',
                             'crystalserver': 'zimbadev/crystalserver@9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d'},
        'rules': {
            'owner_pending': {
                'named_reasons': REASON_OWNER,
                'same_owner_extensions': {owner: pattern.pattern for owner, pattern in OWNER_EXTENSIONS},
            },
            'shared_mechanism': {name: {k: v for k, v in spec.items()} for name, spec in MECHANISMS.items()},
            'rejected_mechanisms': REJECTED_MECHANISMS,
        },
        'counts': {
            'unresolved_lines': unresolved_lines_total,
            'unresolved_conditions': condition_total,
            'blocked_children': blocked_total,
            'reference_gaps': reference_total,
            'total_unresolved_items': unresolved_lines_total + condition_total + blocked_total + reference_total,
            'by_bucket': totals,
            'quests_triaged': len(quests_rows),
            'quests_with_bespoke_work': sum(1 for r in quests_rows if r['counts']['bespoke']),
            'quests_fully_covered_by_owners_and_mechanisms':
                sum(1 for r in quests_rows if not r['counts']['bespoke']
                    and (r['counts']['owner_pending'] or r['counts']['shared_mechanism'])),
            'interactions_unlinked_to_a_quest': len(unlinked),
        },
        'owners_ranked_by_quests': owners_ranked,
        'mechanisms_ranked_by_quests': mechanisms_ranked,
        'unlock_order': order,
        'quests': quests_rows,
        'interactions_unlinked': unlinked,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=1, ensure_ascii=False, sort_keys=False) + '\n')
    print(json.dumps(report['counts'], indent=1))
    print(json.dumps(owners_ranked, indent=1))
    print(json.dumps([{k: v for k, v in m.items() if k in ('mechanism', 'lines', 'interactions', 'quests')}
                      for m in mechanisms_ranked], indent=1))


if __name__ == '__main__':
    main()
