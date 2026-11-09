# Imported native actor admission

Native USE/COLLECT/KILL-only bindings must account for the chosen recipe's
actor requirements. A level-only binding declares its exact minimum level;
boot rejects a different minimum, an omitted nonzero minimum, or unsupported
premium/prerequisite requirements. Existing free entry bindings retain their
serialized shape and definition hashes.

The durable owner checks the declared level against its already locked
`game_character_progression_state.level`, after historical receipt replay and
before any fresh quest write. It reuses the existing QENGINE2 reward-level
policy; the live client or cached runtime level is not an authority.

This slice handles level-only requirements. Premium and prerequisite quests
remain closed until their corresponding actor gates are implemented. It does
not qualify quest rewards or completion.
