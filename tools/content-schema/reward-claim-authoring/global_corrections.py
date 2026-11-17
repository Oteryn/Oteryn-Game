"""Use exact TibiaWiki Global bodies after checking the pinned Crystal bodies.

The Dreammaster (Book) rev 841657, Book of the Lost Heroes (Book) rev 1114576,
and The Holy Tible (Book) rev 1022683 are captured in the QA source commit.
"""
import hashlib

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
