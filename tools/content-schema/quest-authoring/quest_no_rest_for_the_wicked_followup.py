"""Project the two verified Global counts without changing the frozen donor view."""

QUEST = 'oteryn:quest.no_rest_for_the_wicked_quest'
SPOILER = 'https://tibia.fandom.com/?curid=107634&oldid=1177415'
COUNTS = {
    's2': (1, 100, 'Evil to the core'),
    's7': (10, 25, 'Imp-ossible Mission'),
}


def apply(records):
    definition, = (row['definition'] for row in records
                   if row['definition']['identity']['key'] == QUEST)
    supplement = definition['oteryn_recipe']
    if supplement['runtime_enabled']:
        raise ValueError('No Rest for the Wicked followup requires an inactive recipe')
    recipe = supplement['payload']['recipe']
    stages = {stage['key']: stage for stage in recipe['stages']}
    if stages['s7']['targets'] != ['Imp']:
        raise ValueError('No Rest for the Wicked historical Imp target differs')
    for key, (old, new, heading) in COUNTS.items():
        stage = stages[key]
        if stage['count'] != old:
            raise ValueError('No Rest for the Wicked historical count differs: ' + key)
        stage['count'] = new
        recipe['source_notes'].append(f'{key}: {heading} ({SPOILER})')
    stages['s2']['objective'] = 'Collect 100 Demonic Core Essences.'
    stages['s7']['targets'] = ['Charged Imp']
    stages['s7']['objective'] = 'Kill 25 Charged Imps before the Powercollector reaches 25.'
    recipe['adaptations'] = [
        'The fixed route omits per-boss cooldowns and variable team mechanics. '
        'It does not automatically grant Doomforge equipment.'
    ]


def correct_candidate_intent(stage, intent):
    old, new, _ = COUNTS[stage['key']]
    if (intent['count'], stage['count']) != (old, new):
        raise ValueError('No Rest for the Wicked event correction differs')
    corrected = dict(intent, count=new)
    if stage['key'] == 's7':
        target, = intent['targets']
        if target['target'] != 'Imp' or target['refs']:
            raise ValueError('No Rest for the Wicked historical event target differs')
        corrected['targets'] = [dict(target, target='Charged Imp')]
    return corrected
