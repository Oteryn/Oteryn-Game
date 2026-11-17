"""Source-qualified tree identities absent from the frozen legacy Item package."""


def extend_records(records, root):
    # These owners grant identity only; their native lowering remains separate.
    from boss_conditional_item_reference import extend_records as conditional
    from boss_movement_item_profiles import extend_records as movement
    from boss_corpse_terminal_reference import extend_records as corpse
    from qready1_cradle_item_profiles import extend_records as cradle
    from cake_bridge_reference import extend_records as cake
    for owner in (conditional, movement, corpse, cradle, cake):
        owner(records, root)
