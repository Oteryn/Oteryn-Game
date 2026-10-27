# NEWS-HIST-REGEN-1

## Parity

| Evidence | Unfed regeneration | Fed (saturated) regeneration | Amounts and modifiers |
|---|---|---|---|
| Canary `47dfd51f` | Always on; HP 4 s, MP 6 s | HP 3 s, MP 5 s | Vocation amount per tick; resting/equipment effects unchanged |
| tibia.com news 8541/8543 | Always on; HP 4 s, MP 6 s | HP 3 s, MP 5 s | Existing per-tick amounts and modifiers unchanged |
| Oteryn before | Off | Vocation-specific legacy periods | Existing vocation amounts; existing resting/equipment effects |
| Oteryn after | Always on; HP 4 s, MP 6 s | HP 3 s, MP 5 s | Existing vocation amounts; existing resting/equipment effects |

## Implementation

The existing player-condition periodic dispatch owns the one-second clock and selects its cadence
from the existing food condition. No second regeneration owner, durable clock, or migration is
introduced.
