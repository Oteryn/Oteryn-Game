# PUBPROJ contract notes

Owner authorization: 2026-11-07, “1 tak, 2 zgoda”. Base: `d34807f5b`.
Branch: `claude/happy-davinci-ijz3wy-pubproj`; migration allocation: 1350–1359;
new numeric identifiers use next free on the base plus 50. None allocated yet.

## Part A

The parser already has `PublicProjectionsConfig`, default-disabled gating,
purpose-separated credential/fence validation, and a production caller in
`node/serve/supervision.rs`. Install the disabled table in the Synology template
and node boot fixture. No gameplay rule changes or donor rules in this step.

## Part B reconciliation

Read Platform main's
`docs/contracts/OTERYN_V2_PUBLIC_GAME_FAMILIES_EXTENSION_CONTRACT_PROPOSAL.md`
(revision 2). Its V2 paths replace the three candidate V1 families; the existing
V1 watermark enum stays closed. The owner additionally requires stable killer
identity: player CharacterId and native creature content key, historical names
for display only. This amendment must preserve §7 privacy and requires durable
cause capture before a creature identity can be emitted. Producers, acceptance
text, fixtures and execution evidence are still pending in this checkpoint.
