# Hazard V1

Shared encounter capability 117 gates command 169. Identity comes only from the admitted session.

## Request

An empty payload queries the character's hazard levels. One byte in 1..=12 selects an already unlocked level. All other payloads are rejected. Clients cannot unlock levels.

## Result

Two bytes: unlocked level, selected level. Both are in 1..=12; selected cannot exceed unlocked. A refused command has no result payload. Selection is an absolute idempotent assignment under current session, lease, scope and recovery fences. A changed selection advances the global Character revision once, with an immutable command receipt. Queries and same-state selections do not advance it. A receipt replay supplies its original reply and separately reads the current projection; historical selection never overwrites current state. Numeric defaults are provisional Canary values under owner decision 22.
