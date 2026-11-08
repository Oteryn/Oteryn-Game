"""Independently declare the one source-qualified map in the equipment cohort registry."""

def declare(source, proof):
    expected = {'oteryn:item.tibia.i53158'}
    keys = {row['production_definition']['production_key'] for row in proof['records']}
    if keys != expected or len(proof['records']) != 1:
        raise ValueError('TARGUNA_MAP_COHORT_IDENTITY')
    prefix, table = source.split('const PROFILE_ADDITIONS:', 1)
    body, suffix = table.split('];', 1)
    if '"QUESTPC5-TARGUNA-MAP"' not in body:
        body = body.rstrip().rstrip(',') + ',\n    ("QUESTPC5-TARGUNA-MAP", 1),\n'
    return prefix + 'const PROFILE_ADDITIONS:' + body + '];' + suffix
