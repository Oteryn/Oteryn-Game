# Canonical quest reward variant data registration

status: completed (authoring; integration pending)
owner: QuestDATA root; D277/D284 #162, owner-directed full catalogue continuation
branch: codex/quest-canonical-variants-20261002
parent: #1518,b8cdbde9e58f41fe7ad7dd56226921ad6552fcfd
PR/head: publication and FREEZE_SHA entry on #162

Register all105 existing typed source reward variants in the same canonical DATA
family, retaining key/text/random/container/cooldown/achievement payloads and
exact Item/provenance references. Three plain shards231definitions remain byte
equivalent (219ready/12held). Total336definitions; variants92waiting_implementation
and13waiting_data, never Native-ready. Medusa null text carrier, source/default
charge conflicts and Item holds remain explicit. This closes all77 missing data
references in the110 reward-only Quest definitions while retaining execution holds.
Native profile lowering is not implemented by this data task.

Source immutable handoff75245cb2b469c31632eaf5332b4e25186e69ccf508e45f3b7fa7bc7e5ede3633.
Independent source reviewbc6290f4d63d0b982d6c40128e47ae174e136c4771e3fe813ba710980accffc3
PASS:55tests/9negativecontrols, exact105source payloads and231plain records.
Root minimal Quest consumer distinguishes native/source/item holds; migration
counts derive independently from complete source claim inventory. New regression
proves registration cannot turn a held variant into ready Quest fields.

Root qualification:286offline Quest regressions+13variant regressions,262schema
cases PASS; additional consumer hold regression PASS. Full bundle/content check,
contentmigration validator and migration regressions PASS.36governance tests and
governance/repository policy PASS. Initial migration count wrapper mismatch was
corrected before freeze; final gates passed. Native Quest field readiness stays44/66.

Production/source runtime activation is unchanged. Control plane owns required
exact-head external review, parent-first retarget and protected MQ.
