# Native key-door receiver slice

The existing `DoorBook::use_key_observed` supplies an opaque opening only after a
successful matching-key native transition. Its wrong-key, stale, closing and
concurrent-attempt cases remain the World owner's tests. Ordinary quest commands
remain forbidden for the reserved `oteryn:quest-door-use/` prefix.

The registered Root receiver binds a server-captured placement, native opening
revision, key instance/number, observed Character revision and quest definition.
It checks exact carried custody and refuses an opening captured before a later
mission start/reset. Existing native-death receivers retain their original
binding and admission logic behind the shared internal observation slot. Runtime
callers use the new revision-slot method; the writer registration includes it.

The PostgreSQL case uses a registered-receiver fixture and actually mints the
numbered copper keys through the native reward owner. It qualifies Root admission
and replay, not the physical Draconia route. Production dispatch now forwards only the opaque successful opening, after native
target/revision/reach admission, with the pre-action gameplay fence and the exact
used key instance. Runtime and spell-state locks are released before Root awaits.
The exact Draconia placement binding is still pending; this change enables no
production door binding by itself.

The bounded `world-authoring/m01c_key_doors.py` overlay now restores the six
startup key-number attributes in memory and refuses absent, duplicate or
conflicting native locked doors. Its normalized capture is
`tools/content-schema/world-authoring/samples/m01c-key-doors.json`.
The executable donor table is [Crystal door_key.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/startup/tables/door_key.lua):
lines 70–75 bind key 3012 at the two Draconia positions; lines 106–112 bind
3520 at three positions; lines 168–170 bind 5010 at one position.
[storages.lua](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-global/lib/core/storages.lua)
lines 344, 350 and 368 resolve those storage symbols to the same numbers.
The capture pins both source hashes. Table comments naming 4193 and 5009
disagree with the executable entries; the overlay follows the entries.
The generator now applies all six startup key numbers to the production World
and participates in from-scratch World conversion. Region hashes, action counts
and source accounting regenerate together; reapplication is idempotent.
Draconia's entry now binds the two real key-3012 doors and requires level 25.
Either opening advances the shared route track from zero to one; it cannot
complete the Quest or grant a reward. PostgreSQL exercises level 24 refusal,
level 25 acceptance, exact key custody, native-source admission and replay.
No whole-route or completion qualification is claimed.

The historical Ice Rapier SOURCE hold is retained. Nonreward progress may use
the independently qualified native container projection only when source owner,
claim revision, source digest, placement UID, root and every child/count match.
Missing, partial or foreign evidence remains held, as does completion. The
existing receipt tool proved every frozen SOURCE core unchanged after the
compiler-pin refresh; canonical Item qualification is not changed globally.

Locked Item profiles 5097 and 6248 use the existing native importer and pinned
Canary XML/appearance inputs. [items.xml](https://github.com/opentibiabr/canary/blob/99902524e052f37574194466c2949c576e4ab269/data/items/items.xml)
lines 14489 and 17136 identify the locked doors; the source importer supplies
their door classification and solid/projectile physics. Source hashes remain
in `key-door-item-profiles.json` and `canary-ground-proof.json`; no Item
definitions or physical flags are copied from neighbouring door identities.
