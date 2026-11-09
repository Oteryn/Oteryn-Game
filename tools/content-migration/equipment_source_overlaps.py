"""Compose finite source cohorts; runtime profile union sizes are not inputs."""
import json
import re

import elemental_spheres_profiles as spheres
import floor_assembly_profiles as floor
import qm50_elevator_profiles as elevator
import rafzane_elevator_profiles as rafzane
import qready1_craft_item_profiles as craft
import qengine_reward_item_profiles as engine
import engnpc_m05_item_profiles as npc
import m26_endgame_item_profiles as endgame


def source_keys(root, path, expected):
    rows = json.loads((root / path).read_bytes())['records']
    keys = [row['production_definition']['production_key'] for row in rows]
    if len(keys) != len(expected) or set(keys) != expected:
        raise ValueError('EQUIPMENT_OVERLAP_SOURCE_IDENTITIES:' + path)
    return set(keys)


def names(ids):
    return {f'oteryn:item.tibia.i{i}' for i in ids}


def refresh(source, root, qualified_boss_corpses):
    shared = source_keys(root, engine.PROOF, names(engine.IDS))
    floor_keys = source_keys(root, floor.PATH, names(floor.IDS))
    sphere_keys = source_keys(root, spheres.PATH, names(spheres.IDS))
    elevator_keys = source_keys(root, elevator.PATH, names(elevator.IDS))
    rafzane_keys = source_keys(root, rafzane.PATH, names(rafzane.IDS))
    craft_keys = source_keys(root, craft.PACKET, names(craft.IDS))
    npc_keys = source_keys(root, npc.PATH, npc.KEYS)
    endgame_keys = source_keys(root, endgame.PATH, names({33864,33865,33866,33867,33876,34111}))
    cohorts = [
        ('QMECH-M49', sphere_keys, floor_keys, names({7738,7739,7740,7741,7742,7743,7744,7745,7746,7748,8106,8139,8140,8141})),
        ('QMECH-M50', rafzane_keys, elevator_keys, names(elevator.IDS)),
        ('M26-ENDGAME', endgame_keys, qualified_boss_corpses, names({33864,33865,33866,33867,33876})),
        ('QREADY1-CRAFT', craft_keys, shared, names({21506,21507})),
        ('ENGNPC-M05', npc_keys, shared, names({27654})),
        ('M22-TAINTED-TALISMAN', names({22008}), shared, names({22008})),
        ('NPCFIX2-M22', names({22008}), shared, names({22008})),
    ]
    for name, owned, previous, overlap in cohorts:
        if owned & previous != overlap:
            raise ValueError('EQUIPMENT_OVERLAP_SOURCE_INTERSECTION:' + name)
        new = len(owned - overlap)
        pattern = rf'(\("{re.escape(name)}",\s*)([\d_]+)(\))'
        matches = list(re.finditer(pattern, source))
        if len(matches) != 1 or int(matches[0][2].replace('_','')) not in {len(owned), new}:
            raise ValueError('EQUIPMENT_OVERLAP_COHORT_DECLARATION:' + name)
        source = re.sub(pattern, lambda m: m[1] + str(new) + m[3], source)
    return source
