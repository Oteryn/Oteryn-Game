"""Exact Tibia Global corrections of plain donor chest rewards, not donor rewrites.

Owner rule (2026-11-17): where a Canary/Crystal donor differs from Tibia Global,
Global wins. Each entry pins the donor fact it replaces (pilot claim, Crystal
UID, project position and the exact source reward) so a donor change fails
closed instead of being silently re-corrected. The donor fact stays the
evidence in the pinned pilot; only the canonical reward changes.

| pilot claim | TibiaWiki evidence | donor | Global |
|---|---|---|---|
| captain_iglues | Captain Iglues Treasure Quest/Spoiler rev 1013321: the right chest is a Chest (Quest) holding 2 Salmon; the book and 12 salmon are the left, daily-respawn chest | 1 salmon (its own comment says 2x) | 2 salmon |
"""
import copy

# Crystal UID -> (pilot claim key, project position, donor reward, Global reward).
CORRECTIONS = {
    6117: ('canary:reward-claim/kv/captain_iglues', {'x': 32039, 'y': 32121, 'z': 13},
           [[3579, 1]], [[3579, 2]]),
}
BY_PILOT = {pilot: (unique, position, source, target)
            for unique, (pilot, position, source, target) in CORRECTIONS.items()}


def source_reward(unique, position, pairs):
    """Global reward of one Crystal chest table entry; other entries pass through."""
    if unique not in CORRECTIONS:
        return pairs
    _, expected_position, source, target = CORRECTIONS[unique]
    if position != expected_position or [list(pair) for pair in pairs] != source:
        raise ValueError(f'GLOBAL_CORRECTION_SOURCE_DRIFT {unique}')
    return copy.deepcopy(target)


def pilot_items(pilot, position, items):
    """Global reward items of one pilot placement, in the pilot's own item-reference shape."""
    if pilot not in BY_PILOT:
        return items
    _, expected_position, source, target = BY_PILOT[pilot]
    pairs = [[int(entry['item']['key'].rsplit('/', 1)[1]), entry['count']] for entry in items]
    if position != expected_position or pairs != source:
        raise ValueError(f'GLOBAL_CORRECTION_PILOT_DRIFT {pilot}')
    template = items[0]['item']
    return [{'item': {**template, 'key': f"{template['key'].rsplit('/', 1)[0]}/{item}"}, 'count': count}
            for item, count in target]
