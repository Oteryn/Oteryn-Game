# QCX-19 Sea of Light

Status **PARTIAL**. This slice ends when Spectulus sends the player to the Lost Mines.

| Stage | Native content |
| --- | --- |
| The Plans | Spectulus accepts the player on `mission` then `yes`. The beggar takes two separate one-gold donations and awards the plans after the story. Spectulus takes one set of plans and awards 400 experience. |
| The Collector briefing | The plan handoff records transaction time. A new mission dialogue opens after 1,800 seconds, and three confirmations lead to the Lost Mines briefing. |

The quest keeps its own phase and study timestamp. Dialogue can restart after a disconnect without charging a committed donation a second time. The existing NPC offer and reward owners handle dialogue and each fenced value change. [Quest rewards](https://tibia.fandom.com/index.php?curid=41820&oldid=791161) · [Walkthrough and dialogue](https://tibia.fandom.com/index.php?curid=41840&oldid=855981)

## Remaining world bindings

- The first missing hook is the native tool-on-placement binding for the large cyan crystal in the Lost Mines. Dispatch checks the exact tile, appearance, and stack ordinal at `crates/gameplay/dispatch/src/gameplay_transport/quest_callbacks.rs:264`. The packet locates the mine entrance and describes the crystal inside. The source-qualified crystal placement and NPC map marker still need to be established. [Lost Mines source](https://tibia.fandom.com/index.php?curid=41840&oldid=855981)
- Remaining stages require the Rare Crystal handoff for 500 experience, then the Vega Ice Cube use and cave destination, the Carrying Device on the Collector's corpse, and the Filled Carrying Device handoff for 1,000 experience, one Yellow Gem and ten Small Diamonds. The quest remains open until those bindings are implemented. [Walkthrough and dialogue](https://tibia.fandom.com/index.php?curid=41840&oldid=855981) · [Quest rewards](https://tibia.fandom.com/index.php?curid=41820&oldid=791161)

## Regression test

`qcx_sea_of_light` loads the production reward catalogue, checks both individual fees and the Plans delivery and handoff, and verifies the study cutoff immediately before and at 1,800 seconds. It runs without a PostgreSQL route.
