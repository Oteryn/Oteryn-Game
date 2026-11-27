"""Source-qualified tree identities absent from the frozen legacy Item package."""


def extend_records(records, root):
    # These owners grant identity only; their native lowering remains separate.
    from boss_conditional_item_reference import extend_records as conditional
    from boss_corpse_terminal_reference import extend_records as corpse
    from boss_movement_item_profiles import extend_records as movement
    from cake_bridge_reference import extend_records as cake
    from crystal_ordinary_items import extend_references as doors
    from nrwicked_profiles import extend_records as arbaziloth_fire
    from qready1_cradle_item_profiles import extend_records as cradle
    from writable_item_profiles import old_paper_definition


    for owner in (conditional, movement, corpse, cradle, cake, arbaziloth_fire, doors):
        owner(records, root)
    records.append(old_paper_definition(root))

    from fourth_quarter_corpses import extend_item_references as fourth_quarter
    from ordinary_corpse_cohort import extend_item_references as third_quarter
    from ordinary_corpse_profiles import extend_definitions as crystal_corpses
    from ordinary_corpse_slice2 import extend_references as second_quarter

    fourth_quarter(root, records)
    crystal_corpses(root, records)
    second_quarter(records, root)
    third_quarter(records, root)
