# M02 — ordered Ground admission and Nomads window

SRV7 contributes the native Ground prerequisite and framed-use regression.
ENGNPC2 supplies the shared rule owner; SRVWLD supplies the two-cell durable
readset and source window binding. No SRV7 migration or protocol number is used.

Checked game intent: `content/quests/definitions/quests-00300-00351.json`,
`oteryn:quest.authored.nomads_land_quest`, recipe stage `s4`, explicitly requires
obstructing the hideout window with crates and an amphora before the cabinet.
Its recipe labels this a chosen Oteryn approximation. The test definitions and
coordinates are fixtures, not promoted source item/placement identities.

The existing World callback reader only admits an immovable map appearance.
This slice adds an optional bounded `ground_stack` to that same prerequisite:
canonical Item keys and quantities, in native overlay order, with distinct
nonzero custody identities. Unknown/volatile roots and unresolved movable base
roots fail closed. It grants no Item, personal progress or entitlement, and
cannot be composed with the existing forbidden personal/value effects.

Native custody evidence: `ChannelOverlay::add_ground` checks World, scope, map
revision, decoded cell and duplicate Item identity before admitting a record.
The predicate reads that owner through `LoopGroundCache`, whose production
projection sorts durable `stack_ordinal` before constructing the overlay. A
preflight uses the existing Ground reader; execution rechecks the non-invalidated
cache against the live scope, generation, frame and base digest. It does not read
the empty static bundle overlay or manufacture a Ground identity from an appearance. It is evaluated by the existing callback execute path under the
Channel turn. The existing pickup/movement projection remains the only writer.

External evidence checked on 2026-11-08: Crystal
`201189f871d23597b82caf88fd611454a2ddefb8`,
`data-global/npc/muhad.lua`, contains Arito's predecessor dialogue but no window
stack rule; no crate/amphora ordinal was inferred from it.
https://raw.githubusercontent.com/zimbadev/crystalserver/201189f871d23597b82caf88fd611454a2ddefb8/data-global/npc/muhad.lua

NEED-FETCH (wiki-pending; exact requests refused):
- https://tibia.fandom.com/api.php?action=query&prop=revisions&revids=1002600&rvprop=content&format=json — HTTP 402.
- https://tibiawiki.com.br/api.php?action=parse&page=Nomads_Land_Quest&prop=wikitext&format=json — HTTP 403.

The retained recipe source references revision 1002600 (spoiler) and 842864
(base). Remaining consumer qualification: exact source window/cabinet placement,
crate/amphora identities and arrangement, plus the production quest binding.

## Qmech merge reconciliation

The 20:25 UTC fetch advanced qmech to `3c06a2442`, whose updated INDEX assigns
M02 to ENGNPC2 and includes the shared `quest::ground_stack` rule owner. This
lane releases full M02 consumer work to that owner. The native callback adapter
now delegates ordered comparison to that shared owner, including exact definition
revision and positive ordered observation ordinals; no second comparison rule is
retained. All three SRV7 regression behaviours remain tested. Ground predicates
are explicitly value-free even after M07 admits *static* personal trial predicates:
a mutable read-only projection cannot authorize a personal durable write.

The subsequent qmech merge adds ENGNPC2's fenced durable Ground participant and
assigns M02 jointly to ENGNPC2+SRV7. The public `{item,count}` wire shape is retained;
both native and durable readers now delegate to the shared comparison rule with
actual definition revisions and append ordinals. Personal progress requires world
admission and the durable participant; an unfenced read-only declaration still
fails parsing. Source quest window/cabinet placement qualification remains separate.

## Source window qualification received with qmech 0326dc2e5

`SRVWLD-M02.md` records the fetched Arito's Task Quest source, lines 115–117,
including the body SHA-256 and donor Item identities. The source specifies two
piles, each with two crates topped by a large amphora (or three crates). The
registered `oteryn:quest-callback/m02/nomads-land/window` uses crate 2471 and
large amphora 2904 beside the committed sandstone window 1744 at
(33216,32855,-9). Its coordinates are an explicitly recorded Oteryn placement.
This supersedes the earlier outstanding source-placement questions above.

The merged native reader preserves current Item-definition revisions and actual
persisted append ordinals. The durable callback checks all required cells once,
locking Item identities before the existing Ground owner lock, while retaining
the recovery/scope transaction proof. The source window has no personal reward
or fabricated quest progress. Focused qualification includes
`m02_nomads_window_postgres_stateless_observer_requires_the_real_stack`.
