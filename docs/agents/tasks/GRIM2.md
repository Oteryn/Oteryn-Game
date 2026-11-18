# GRIM2 Grimvale engine audit

Base `871c1dec7d9963420d455210e4cddc0c9df80d26`. Compared the QCX67 Grimvale audit at `57645c54:docs/agents/tasks/QCX-grimvale.md` with Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.

## Paradise Lost timed NPC exchange

Sorry, not possible.

Crystal Tonar removes one shell and grants the ocarina in the same confirmation. The storage write records the exchange. [Crystal `data-global/npc/tonar.lua:66–82`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/tonar.lua#L66-L82)

## Full Moon mine-to-mark permissions

Sorry, not possible.

The mining action sets a retry timestamp and can grant a vial of liquid silver. Each mark checks and updates its own cooldown when the player steps onto it. Crystal has no mining prerequisite for either mark. [Crystal `data-global/scripts/lib/register_actions.lua:670–697`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/lib/register_actions.lua#L670-L697) · [Crystal `data-global/scripts/quests/grimvale/movements_strangely_glowing_mark.lua:13–28`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/scripts/quests/grimvale/movements_strangely_glowing_mark.lua#L13-L28)

The donor-only rule leaves both requested behaviors unsupported. This audit adds no engine caller, PostgreSQL case, migration, or authored content.
