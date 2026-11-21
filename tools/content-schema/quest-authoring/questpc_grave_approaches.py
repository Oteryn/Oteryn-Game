"""Twelve chosen grave visits: seven sanctification sites and five tomb antechambers."""
QUEST = 'oteryn:quest.grave_danger_quest'
SOURCE_QUEST = 'canary:quest/grave_danger_quest'
SANCTIFY = 'actions_grave_sanctify.lua'
# Exact pinned script positions followed by explicitly chosen clear native ground.
ROWS = (
    ('dark-cathedral', SANCTIFY, (32644, 32394, -8), (32645, 32394, -8, 410)),
    ('femor-hills', SANCTIFY, (32542, 31846, -6), (32542, 31845, -6, 11417)),
    ('ankrahmun', SANCTIFY, (33376, 32806, -8), (33376, 32805, -8, 418)),
    ('vengoth', SANCTIFY, (32959, 31534, -7), (32960, 31534, -7, 7062)),
    ('orclands', SANCTIFY, (32776, 31817, -8), (32777, 31817, -8, 4409)),
    ('ice-islands', SANCTIFY, (32012, 31558, -7), (32012, 31557, -7, 436)),
    ('kilmaresh', SANCTIFY, (33813, 31624, -9), (33813, 31628, -9, 469)),
    ('duke-krule', 'actions_duke_krule.lua', (33455, 31493, -13), (33456, 31493, -13, 429)),
    ('count-vlarkorth', 'actions_count_vlarkorth_.lua', (33455, 31413, -13), (33456, 31413, -13, 429)),
    ('earl-osam', 'actions_earl_osam.lua', (33516, 31444, -13), (33517, 31444, -13, 429)),
    ('lord-azaram', 'actions_lord_azaram.lua', (33422, 31493, -13), (33423, 31493, -13, 429)),
    ('baeloc-nictros', 'actions_baeloc_nictros.lua', (33424, 31413, -13), (33425, 31413, -13, 429)),
)
SITES = {'grave-danger/' + name:
         (QUEST, SOURCE_QUEST, 'grave_danger_quest/' + script, origin, approach)
         for name, script, origin, approach in ROWS}
# The Kilmaresh tomb's immediate ring contains retained pillars/tomb pieces.
# Use the clear southern outer approach; never weaken native obstacle admission.
DISTANCES = {'grave-danger/kilmaresh': 4}
