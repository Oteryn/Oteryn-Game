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
| ghostsilver_lantern_quest | Ghostsilver Lantern rev 1148076: the Spider Web at 128.56/123.176/9 (32824,31664,9) gives the lit Ghostsilver Lantern, item 23738; 23734 is the unlit variant | 1 unlit lantern 23734 | 1 lit lantern 23738 |
| Koshei platinum cache | Koshei the Deathless Quest rev 842829 and the chosen Oteryn recipe | 100 platinum coins | 50 platinum coins |

Written book texts (TEXTS) follow the same rule. The Global text is the TibiaWiki
book page text as the wiki renders it (`<br>` is a line break; a plain source
newline renders as one space); talesoftibia libsearch carries the same words.

| pilot claim | TibiaWiki evidence | donor | Global |
|---|---|---|---|
| powerbolt2 | The Dreammaster (Book) rev 841657 | reworded text ("dream master", "can't", one sentence per line) | the wiki text |
| dreamers_challenge/chest_book | Book of the Lost Heroes (Book) rev 1114576 | the six names only | heading line "Book of the Lost Heroes" before the names |
| the_pits_of_inferno/chest_tible | The Holy Tible (Book) rev 1022683 | no heading, "honour" | heading "The Holy Tible:" and a blank line, "honor" |
"""
import copy
import hashlib

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
    6241: ('canary:reward-claim/quest/u8_1/koshei_the_deathless/koshei_the_deathless_platinum',
           {'x': 33261, 'y': 32448, 'z': 12}, [[3035, 100]], [[3035, 50]]),
    # Crystal 201189f8 chest.lua:2916-2922 gives 23734; actions_lantern.lua:1-4
    # and items.xml:47292-47296 identify 23738 as the lit form.
    6310: ('canary:reward-claim/kv/ghostsilver_lantern_quest', {'x': 32825, 'y': 31664, 'z': 9},
           [[23734, 1]], [[23738, 1]]),
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

DREAMMASTER = (
    "... the dreammaster retreated to the world behind the curtains of awareness, I cant reach him, "
    "now that the last hall of dreams is lost to the forces of evil. I sealed Goshnar's grave so noone "
    "can enter the pits without knowledge of our secret. I will try to retreat to knightwatch tower and "
    "wait for a dreamer in possession of the key, so we can travel one of the dreampaths to a safer place "
    "to regroup and plan a counterattack. I fear we have to recruit new members and there is only little "
    "time left to train them. I hope Taciror will not waste our last forces in a futile attack on the "
    "ruthless seven. Our order has never truly recovered from the losses in our war against Goshnar and "
    "his undead hordes. Now with the death of our leaders and best warriors in the attack of the demonic "
    "forces we don't stand a chance against our enemies. Our only hope is to gather new forces and retake "
    "the chamber of dreams. Of couse I know the right method to distract hugo long enough to get past him. "
    "The dreammaster is important to teach our recruits in the old ways and the art of dreamwalking. We "
    "need a leader for our cause and we need him bad. Headless we will fail and fall. It's already "
    "uncertain who took the nightmare chronicles out of the pits and I have no idea where they hid them. "
    "They are fighting about power and influence but unity is the key to success. Our whole order is "
    "centred about unity. All our rituals and procedures rooted on unity and sharing, they can't neglect it.")
# Crystal UID -> (pilot claim key, sha256 of the donor body, Global body).
TEXTS = {
    6112: ('canary:reward-claim/kv/powerbolt2',
           '9ea894f92d1b6e4cf60555cd08f5e552b8c5ce59c11c13eb6d35f7ff07abb036', DREAMMASTER),
    6196: ('canary:reward-claim/quest/u7_9/dreamers_challenge/chest_book',
           '50918d1f490444e7ecc11a36c2a48b6e1c6ee9229931abb59c69b1042df50b9f',
           'Book of the Lost Heroes\nAskuro\nMantar\nIldoni\nMaxmof\nXemodo\nOxanti'),
    6202: ('canary:reward-claim/quest/u7_9/the_pits_of_inferno/chest_tible',
           'cd5d63f500cf56283e976d96162ed57678dd8544fd2a174d32744747fe607e11',
           'The Holy Tible:\n\nBanor I praise your name.\nBe with me in the battle.\n'
           'Be my shield, let me be your sword.\nI will honor the godly spark in my soul.\n'
           'May it flourish and grow.'),
}
TEXTS_BY_PILOT = {pilot: (unique, donor, body) for unique, (pilot, donor, body) in TEXTS.items()}


def text_ref(body):
    return {'length': len(body), 'placeholders': [], 'sha256': hashlib.sha256(body.encode()).hexdigest()}


def source_text(unique, body):
    """Global text of one Crystal chest book; other books pass through."""
    if unique not in TEXTS:
        return body
    _, donor, target = TEXTS[unique]
    if hashlib.sha256(body.encode()).hexdigest() != donor:
        raise ValueError(f'GLOBAL_TEXT_SOURCE_DRIFT {unique}')
    return target


def correct_written_text(pilot, reward):
    """Global text reference of one canonical written-text variant; others pass through."""
    if pilot not in TEXTS_BY_PILOT:
        return
    _, donor, target = TEXTS_BY_PILOT[pilot]
    if reward.get('written_text', {}).get('text_ref', {}).get('sha256') != donor:
        raise ValueError(f'GLOBAL_TEXT_CANONICAL_DRIFT {pilot}')
    reward['written_text']['text_ref'] = text_ref(target)


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
