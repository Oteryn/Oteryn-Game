"""Pinned Wooden Stake priest sequence; carried evidence retains instance identity."""
import re

PRIESTS = ('quentin', 'tibra', 'maealil', 'yberius', 'isimov',
           'amanda', 'kasmir', 'rahkem', 'brewster', 'tyrias')


def append(add, state, callbacks, inputs):
    track = 'u7_8/friends_and_traders/the_blessed_stake'
    for before, npc in enumerate(PRIESTS, 1):
        text = inputs[f'data-global/npc/{npc}.lua'].decode()
        branches = list(re.finditer(
            r'stakeKeyword:addChildKeyword\(\{ "yes" \},[^\n]*text = "(So receive my prayer:[^"\n]*)"[^\n]*function\(player\)(.*?)\nend\)',
            text, re.S))
        matches = [m for m in branches if re.search(
            rf'TheBlessedStake, {before + 1}\)', m[2])]
        if len(matches) != 1 or 'getItemCount(5941)' not in text:
            raise ValueError('Pinned priest branch drift: ' + npc)
        branch = matches[0]
        if npc == 'tyrias':
            if not all(re.search(pattern, branch[2]) for pattern in (
                    r'player:removeItem\(5941,\s*1\)', r'player:addItem\(5942,\s*1\)')):
                raise ValueError('Pinned final stake exchange drift')
        elif 'removeItem(5941' in branch[2]:
            raise ValueError('Pinned prayer no longer retains the stake: ' + npc)
        prefix = text[:branch.start()]
        questions = list(re.finditer(
            r'local stakeKeyword = keywordHandler:addKeyword\(\{ "stake" \}.*?text = "([^"\n]+)"',
            prefix, re.S))
        if not questions:
            raise ValueError('Pinned priest offer drift: ' + npc)
        add('exchange-prayer-' + npc, 'friends_and_traders', npc, ['yes'],
            [state(track, before, before + 1)], branch[1], topic=1, source_text=branch[1],
            offer=(['stake'], questions[-1][1]),
            take=[(5941, 1)] if npc == 'tyrias' else (),
            give=(5942, 1) if npc == 'tyrias' else None)
        if npc != 'tyrias':
            callbacks[-1]['present'] = [{'item': 'oteryn:item.tibia.i5941', 'count': 1}]
