# M16: Sterling's personal treasure marker

The consumer is the Targuna Long Lost Treasure dialogue, not a claim that the
whole Targuna quest is qualified. The source is [Crystal Sterling at
9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d](https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/npc/sterling.lua).
Its retained blob SHA-256 is
`3069d6f8265d9b414aedc6c50460268d5c3e98c0a65257cc48c54750e44516a9`.
The authoring tool checks those bytes in the repository's donor corpus before
emitting the three mounted callback, offer and greeting documents.

The source's treasure position `(31948, 31924, 7)` is native floor `-7`.
At mission 3, `mission` opens topic 2; `yes` requires Morla's completed work,
advances mission to 4, records `treasure_marked = 1`, and adds the Cross marker
labelled `Treasure`. The source does not remove an inventory map on this branch;
the callback therefore charges no item. Marker and progress use one durable
reward transaction and the marker is delivered only through the admitted
Character's personal map snapshot. Client markers are session memory, separate
from editable World-scoped local markers.

The PostgreSQL dialogue test uses the canonical mounted callback and a
stage-ready Character fixture (mission 3, Morla done, marker absent). It talks to
Sterling through `hi`, `mission`, `yes`, then reads the actual persisted marker
through the admitted transport adapter. It also checks unselected capability,
stale connection, stale lease and read-only revision behavior. The separate Root
case covers refusal, atomicity, replay and immutable source identity.
