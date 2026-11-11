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
| 7861 | A character must deal actual boss damage to qualify; support-only participation is insufficient. | Superseded by the standing donor-parity rule: Crystal rewards any nonzero contribution, including support-only. |

The pinned reward script does not contain the news-era resilience adjustment or damage prerequisite.
The standing donor-parity instruction governs eligibility and loot scaling: retain the three
contribution dimensions and the pinned reward script, rather than inferring a damage prerequisite
or a minimum score from later news. Resilience remains an independently supplied encounter parameter.

## Donor eligibility reconciliation

The earlier positive-damage and 10% fair-share deviation is superseded by the standing
owner instruction to follow Crystal 201189f8. `data/scripts/systems/reward_chest.lua:54-69`
computes `(damageOut / totalDamageOut + damageIn / totalDamageIn + healing / totalHealing) / 3`
and rewards every nonzero score. The totals start at 0.1 (line 28), so damage-only,
tanking-only, and healing-only contributions all qualify; an all-zero participant does not.
There is no minimum fair-share threshold and no Soul War-specific eligibility exception.
Healing of other players is recorded at lines 156-162, and damage taken at lines 163-165.

The existing loot-factor calculation continues to use the same normalized score and complete
frozen census. Resilience remains an independently supplied encounter parameter.
