"""Exact Tibia Global corrections of plain donor chest rewards, not donor rewrites.

Owner rule (2026-11-17): where a Canary/Crystal donor differs from Tibia Global,
Global wins. Each entry pins the donor fact it replaces (pilot claim, Crystal
UID, project position and the exact source reward) so a donor change fails
closed instead of being silently re-corrected. The donor fact stays the
evidence in the pinned pilot; only the canonical reward changes.

| pilot claim | TibiaWiki evidence | donor | Global |
|---|---|---|---|
| captain_iglues | Captain Iglues Treasure Quest/Spoiler rev 1013321: the right chest is a Chest (Quest) holding 2 Salmon; the book and 12 salmon are the left, daily-respawn chest | 1 salmon (its own comment says 2x) | 2 salmon |
| quest/key/id4502 | Alawar's Vault Quest rev 683774 and /Spoiler rev 1029814: the Key 4502 reward is a Dark Helmet, 4 Throwing Knives, a Blank Rune, 33gp and Key 4502 | 23 gold coins | 33 gold coins |
| fireaxe1 | Fire Axe Quest/Spoiler rev 855030: the chest holds a Ring of Healing, a Dragon Necklace and 7 Small Diamonds; the Fire Axe is the final reward of the Pile of Bones (fireaxe2) | the bag also holds a Fire Axe (its own comment omits it) | no Fire Axe in the bag |
| powerring1 | Power Ring Quest rev 683946: the rewards are a Power Ring and a Bronze Amulet (powerring2) | 1 gold coin (its own comment says power ring) | 1 Power Ring |
"""
import copy

# Crystal UID -> (pilot claim key, project position, donor reward, Global reward).
CORRECTIONS = {
    5008: ('canary:reward-claim/quest/key/id4502', {'x': 32201, 'y': 31571, 'z': 10},
           [[2970, 1], [3031, 23], [3147, 1], [3298, 4], [3384, 1]],
           [[2970, 1], [3031, 33], [3147, 1], [3298, 4], [3384, 1]]),
    6067: ('canary:reward-claim/kv/fireaxe1', {'x': 33078, 'y': 31656, 'z': 11},
           [[3098, 1], [3085, 200], [3028, 7], [3320, 1]],
           [[3098, 1], [3085, 200], [3028, 7]]),
    6117: ('canary:reward-claim/kv/captain_iglues', {'x': 32039, 'y': 32121, 'z': 13},
           [[3579, 1]], [[3579, 2]]),
    6151: ('canary:reward-claim/kv/powerring1', {'x': 32599, 'y': 31776, 'z': 9},
           [[3031, 1]], [[3050, 1]]),
}
BY_PILOT = {pilot: (unique, position, source, target)
            for unique, (pilot, position, source, target) in CORRECTIONS.items()}
# Canonical container contents (charges already normalized to one instance).
CANONICAL = {
    'canary:reward-claim/quest/key/id4502': (
        [[2970, 1], [3031, 23], [3147, 1], [3298, 4], [3384, 1]],
        [[2970, 1], [3031, 33], [3147, 1], [3298, 4], [3384, 1]]),
    'canary:reward-claim/kv/fireaxe1': (
        [[3098, 1], [3085, 1], [3028, 7], [3320, 1]],
        [[3098, 1], [3085, 1], [3028, 7]]),
}


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


def correct_canonical(pilot, position, reward):
    """Global contents of one canonical container variant; others pass through."""
    if pilot not in CANONICAL:
        return
    source, target = CANONICAL[pilot]
    pairs = [[int(q['item']['key'].rsplit('.i', 1)[1]), q['count']] for q in reward.get('items', [])]
    if position != BY_PILOT[pilot][1] or pairs != source or 'container' not in reward:
        raise ValueError(f'GLOBAL_CORRECTION_CANONICAL_DRIFT {pilot}')
    template = reward['items'][0]['item']
    reward['items'] = [{'item': {**template, 'key': f'oteryn:item.tibia.i{item}'}, 'count': count}
                       for item, count in target]
