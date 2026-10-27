# Boss resilience and participation reference

Task: `NEWS-HIST-BOSS-RESILIENCE-1`. News sources: 7609, 7719 and 7861.

## Pinned server evidence

- Canary is examined at `47dfd51f45280a59a1d3e50ba7edd573d7234446`; Crystal is examined at
  `201189f871d23597b82caf88fd611454a2ddefb8`.
- Both pinned `data/scripts/systems/reward_chest.lua` implementations collect three participation
  dimensions: damage dealt, damage received and healing of another player. They sum each dimension
  independently, divide each contribution by its dimension total, then divide the sum by **3**
  (Canary lines 27–59; Crystal lines 27–59).
- Both pinned implementations initialise each dimension total to **0.1** to avoid division by zero,
  and treat a nonzero resulting score as participation (both files, lines 27–29 and 52–59).
- Both pinned implementations record healing only when the healer is a player other than the healed
  character. Damage received is recorded only when the attacker is the boss (both files, lines
  136–167).
- Both pinned `src/creatures/monsters/monster.cpp` implementations refuse target-distance changes for
  reward bosses (Canary lines 3488–3506; Crystal lines 2616–2634). This is the only pinned generic
  reward-boss resilience rule found in the native creature owner.

## News reconciliation

| News | Evidence | Disposition |
|---|---|---|
| 7609 | On spawn, selected bosses take less damage and the defence gradually returns to normal. | The exact boss list, initial percentage, duration and curve are **UNKNOWN** in both pinned implementations; no numbers are guessed. |
| 7719 | Participation other than damage dealt is reduced by the percentage of current bonus boss resilience. | Implemented as a parameterised integer percentage; damage dealt is unchanged. |
| 7861 | A character must deal actual boss damage to qualify; support-only participation is insufficient. | Implemented as a positive-damage gate at reward dispatch. |

The pinned reward script does not contain the news-era resilience adjustment or damage prerequisite.
Oteryn therefore keeps the source-backed three dimensions but applies the later news rules in the
native encounter owner. It does not reproduce the pinned Lua loot-factor formula: NEWS 7609 does not
supply its missing resilience numbers, and floating-point loot generation remains outside this task.
