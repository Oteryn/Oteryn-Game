"""Qualify the selected Atelier snapshot independently of its builder.

These reference facts do not admit runtime effects or certify Global RNG.
Semantic authoring can tune values; exact source qualification cannot.
"""
from wheel_authoring import ROOT, read

WIKI = 'https://tibia.fandom.com/wiki/Wheel_of_Destiny?oldid=1151969'
DECISION = 'docs/architecture/reviews/OTERYN_GAME_WHEEL_GEM0A_REFERENCE_VALUES_AMENDMENT_DECISION_2026-10-01.md'
LOOT_SOURCE = 'https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/data/libs/functions/gematelier.lua'
LOOT_SHA256 = '31c6fea70a6dd91d89bfe8ad9a82b92bffd1e56e36d2e177c11b0c496e28376f'


def validate_atelier_reference(candidate):
    def require(condition, code):
        if not condition:
            raise ValueError(code)

    gems = candidate['gems']
    require(gems['atelier']['operation_policy']['revealed_gem_limit'] == 225,
            'ATELIER_REFERENCE_CAP')
    require(gems['grade_costs'] == [
        {'target_grade': 1, 'basic': {'gold': 2000000, 'fragments': 5},
         'supreme': {'gold': 5000000, 'fragments': 5}},
        {'target_grade': 2, 'basic': {'gold': 5000000, 'fragments': 15},
         'supreme': {'gold': 12500000, 'fragments': 15}},
        {'target_grade': 3, 'basic': {'gold': 30000000, 'fragments': 30},
         'supreme': {'gold': 75000000, 'fragments': 30}},
    ], 'ATELIER_REFERENCE_GRADE_COSTS')
    loot = gems['loot_reference']
    require(loot['roll_denominator'] == 100000 and
            loot['category_precedence'] == ['influenced', 'fiendish', 'archfoe'] and
            loot['uniform_vocation_selection'] is True,
            'ATELIER_REFERENCE_LOOT_SELECTION')
    require(loot['per_quality'] == [
        {'quality': 'lesser', 'chance_by_category': {'influenced': 9000, 'fiendish': 3000, 'archfoe': 0},
         'maximum_trials': 2, 'stop_on_first_failure': True},
        {'quality': 'regular', 'chance_by_category': {'influenced': 0, 'fiendish': 3000, 'archfoe': 9000},
         'maximum_trials': 2, 'stop_on_first_failure': True},
        {'quality': 'greater', 'chance_by_category': {'influenced': 0, 'fiendish': 9000, 'archfoe': 3000},
         'maximum_trials': 1, 'stop_on_first_failure': True},
    ], 'ATELIER_REFERENCE_LOOT_TRIALS')


if __name__ == '__main__':
    validate_atelier_reference(read(ROOT / 'samples/wheel-candidate.json'))
    print('PASS: Atelier cap/costs and OTS stop-on-first-failure loot reference.')
