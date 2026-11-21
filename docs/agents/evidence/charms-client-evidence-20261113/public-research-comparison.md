# Bounded public Charms research

Compared against main `3e168ce6afe374485da20ee0ac13d3749558e2f8`, accessed
2026-11-13. The accompanying JSON matrix retains17 source locators, five topic
comparisons, nine rehashable Git-blob receipts and per-video timestamp status.
No runtime code, mechanic authority/status, raw video, image or transcript is
published. This extends the static/active packet; it does not qualify combat.

| Topic | Existing source / current consumer | Public evidence and remaining limit |
| --- | --- | --- |
| Parry armor / mana drain | `charm_combat::damage_amount` honors armor and resistance flags; health reflection calls it. Native incoming-condition reflection has an explicit `!mana` guard. | Venitze_king's own comparison playlist and Tibia Guides' test video are useful primary leads. A community armor assertion agrees with catalogue flags but supplies no isolated armor experiment. Mana-drain-only behavior and exact pre/post mitigation remain unqualified. |
| Dodge + Minor ordering | `mechanics.json` already records conflicting upstream orders. Rust separates `IncomingCreatureAttack` Dodge from `IncomingCreatureHit` effects. | No inspected modern same-hit controlled test settles order. Purchase-priority discussions and pre2024 videos do not answer the two-assignment question. This is an unresolved comparison, not a declaration that no evidence exists. |
| Cleanse Agony / reproc | Native owner already has11sec immunity. Candidate selection excludes Drown but has no explicit Agony exclusion; application separately bypasses immunity for Agony. Commit removes the selected retained instance and replaces matching immunity. | [Official manual](https://www.tibia.com/gameguides/?section=characters&subtopic=manual) says Agony cannot be cured, blocked or resisted. [Official25Aug2026 change](https://www.tibia.com/news/?id=8935&subtopic=newsarchive) explicitly adds Hex removal/immunity. Potential Agony selection conflict needs a complete eligible-instance trace; Hex/IntenseHex representation and new-proc refresh remain open. Receipt replay protection is not a new-proc test. |
| Leech AoE / rounding | Source metadata already retains unresolved AoE/rounding candidates. Current helper rounds positive per-hit arithmetic half-up and clamps; dispatch supplies actual HP loss, with no target-count input in that function. | [Alytreta creator video](https://www.youtube.com/watch?v=MxiCFl3OT_8) auto transcript10:46–15:28 describes collection and AoE dilution;21:48–23:16 reports test-server cave-rat overkill healing. This gives a historical divergence candidate for the inspected dispatch seam. It does not resolve current rounding or a complete upstream target-count callgraph. Old4%/2% amounts cannot qualify current stages. |
| Carnage chain / attribution | Current evaluator explicitly prevents CharmDamage reentry; dispatch targets four cardinal same-floor neighbours and settles child kills with the captured reward principal. | [TibiaQA geometry report](https://www.tibiaqa.com/36586/how-does-charm-carnage-range-work) links [Vda_pedro34:02](https://www.youtube.com/watch?v=1Bs8nL7D8UM&t=2042s). The page/video identity was verified, but that timestamp was not frame-qualified here. Geometry corroboration does not establish recursive kills or Global reward attribution. |

The leech creator transcript is Portuguese automatic captions. Its timestamps
are **transcript-read**, not independently verified frame arithmetic. The2022
Reddit interpretation has inconsistent printed denominators and is not a
rounding oracle. Carnage and the older Parry video had no exportable transcript;
their identity/links are retained without fabricated timestamps or outcomes.
Exact publication dates are null where not established; relative UI ages were
not converted into invented dates.

Three calculator observations from the lead are retained separately in the
matrix. TibiaPal's inspected JavaScript models Overpower/Overflux and elemental
damage but lacks current stage/rate/level qualification. TibiaMaps models
assignment optimization; ExevoPan compares expected damage with its displayed
assumptions. None resolves the five interaction topics above. A calculator is
evidence of its own model, not proof of Global execution.

Owner-supplied Discord locators are registered for future reading. The lead
verified Tibia Analysers and Kusnier server access; the publication worker did
not independently inspect those channels. A31Oct2026 Wojtek Agresja message
about LowBlow AoE crit correlation is retained as a community hypothesis only.
No private Discord body, author profile, raw transcript or media is copied.
One bounded `parry armor` search returning zero is not evidence of absence.

The next useful qualification requires evidence that distinguishes competing
rules: armor-isolated Parry and mana-only attacks; Dodge plus a visible Minor
on the same hit; an eligible Agony instance and repeat successful Cleanse proc;
integer-boundary AoE/overkill leech; Carnage child kills with credit/chain logs.
Do not repeat basic geometry or infer mechanics from a few statistical kills.

```sh
python docs/agents/evidence/charms-client-evidence-20261113/validate_public_research.py --repository .
python docs/agents/evidence/charms-client-evidence-20261113/validate_public_research.py --test
```
