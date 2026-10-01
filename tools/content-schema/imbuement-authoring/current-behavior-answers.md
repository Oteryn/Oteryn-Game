# Current imbuement answers —2026-10-01

The12 original question groups now have1 owner scope resolution and11 concrete
implementation answer groups across Canary and both Crystal branches. The
[validated evidence packet](samples/current-behavior-answers.json) records whole-file
SHA256 identities, exact code anchors, per-engine values and separate Global sources.
This is a data/research deliverable; runtime integration remains assigned through#162.

Engine sources were read through normal public HTTP at pinned revisions:
[Canary](https://github.com/opentibiabr/canary/tree/04b83b512114bfd888000d6e1433ed8ecaec7c5b),
[Crystal imbuements](https://github.com/zimbadev/crystalserver/tree/15593c28fd9adc2bb9739cf0fdb1a4289ebfe1e1),
[Crystal summer-update](https://github.com/zimbadev/crystalserver/tree/00ce02a57ca5a12e48f32a3476e37471167e4c3f).
Current [Formulae](https://tibia.fandom.com/wiki/Formulae),
[Life Leech](https://tibia.fandom.com/wiki/Life_Leech) and
[Vibrancy](https://tibia.fandom.com/wiki/Vibrancy) were read through Remote Desktop +
Chrome/CDP after ordinary HTTP402/403. Other retained official/wiki/forum sources
keep their individual access method, date and confidence in the Global/combat ledgers.

| Original question | Concrete answer | Current Global reference and limits |
|---|---|---|
| [Data target](samples/current-behavior-answers.json) | Active target is current Global research as of2026-10-01. The owner removed the historical July28 snapshot requirement. | Current-date selection is an owner scope decision, not an empirical hidden-server claim. |
| [Etcher consumption](samples/current-behavior-answers.json) | One Etcher on success, zero on ordinary pre-debit rejection. Internal-error ordering differs and is separately recorded. | Public references specify clear-all, Free-account use and worthy acquisition; exact consumed units are supplied by engine implementations. |
| [Equipped scroll target](samples/current-behavior-answers.json) | Equipped targets are explicitly accepted and stats updated by all three inspected C++ paths. | Official manual allows direct inventory use and CM Liamas says anywhere. The datedMinerva tutorial additionally showsPowerful Vampirism 20:00h/success withGhostChestplate in the equipped armor slot; this is a named public case. |
| [Timers](samples/current-behavior-answers.json) | All engines require equipped/online state. Aggressive categories require fight and outsidePZ; default utility categories run outside combat/PZ. Canary accounts elapsed seconds, Crystal debits1second per executed1000ms scan. | Current official manual supplies coarse equipped/hunting timing; utility speed/capacity/paralysis timers have dated community support. |
| [Combat and leech](samples/current-behavior-answers.json) | Current public mana formula uses per-target ceiling and counts overkill; all three code references use clipped realDamage and per-target rounding. Critical rolls, Wheel/charm shares and healing extension behavior are traced separately. | Current public Formulae1205374 describes Mana ONLY as sum of per-target ceilings: ceil(Dmg_i*share*(0.1*N+0.9)/N), critical damage included, DamagePrey excluded, overkill included. Current Life1101811 separately excludes DamagePrey. Unequal-Life/zero-hit counting/arbitrary native-Wheel-proficiency composition and complete mitigation ordering are not selected from this Mana reference. |
| [Vibrancy PvP](samples/current-behavior-answers.json) | Current family wording requires initial success. Canary reflects before recovery RNG and suppresses a failed additional PvP refresh when pvpDeflect applies. Both Crystal parsers ignore the XML Vibrancy effect type; generic deflect is a separate mechanism. | Full CURRENT public family1194726 explicitly says additional PvP paralysis is deflected IF INITIALLY SUCCESSFUL; it gives an already-paralyzed50% recovery example. The qualification is a concrete public statement. It does not define the initial-success state, lifetime/reset, or a universal all-tier50% probability. Archived884982 agrees. Historical live release4828 explicitly removed reflection, which conflicts with Canary reflecting current additional attacks; current proprietary implementation is not inferred. |
| [Payment sources/order](samples/current-behavior-answers.json) | Gold is paid before recipe components. Coins precede bank; inventory precedes Stash. Recipes follow XML order. Gold collection is breadth-first, material search depth-first; coin sort differs between engines. | Current Imbuing states inventory-first then Stash/bank. Exact hidden Global container and inter-resource order is not specified. |
| [Failure consumption/rollback](samples/current-behavior-answers.json) | Ordinary prechecks precede debits. Later inscription output/removal failures can follow gold/material/blank debits without compensation. Etcher mutation/consumption order differs; universal rollback is absent from inspected flows. | Global guarantees crafting success. This does not describe invalid requests, output insertion errors, malformed definitions or resource rollback. |
| [Native effect composition](samples/current-behavior-answers.json) | Native/equipment skills accumulate additively in code; percentage mitigation is iterative with engine-specific rounding and armor order. Current Formulae provides a bounded native equipment example, not the universal mixed composition. | Current native-equipment reference supplies iterative percentage reduction with floor remaining damage per item:200, ZaoanHelmet5% ->190, ProtectionAmulet6% ->178; then armor9 reduction4..7 ->171..174. This named native example does not select arbitrary imbuement/native/Wheel ordering. Historical two-Powerful-Void(16%) and two-Powerful-Vampirism(50%) pairs plus the2022 Void+Wheel8.5% example remain bounded historical references, rather than universal current stacking/caps. |
| [Ownership transfer/state](samples/current-behavior-answers.json) | Imbuement ID and remaining seconds are packed into per-item custom attributes, saved/loaded and moved intact in standard direct trade/mail. No ownership field/reset exists there; recipient equip resumes the retained duration. Market disallows active imbuements. | Community guide permits transferred imbued gear. Public listings and ordinary coin trade do not establish precise before/after duration preservation across every Global route. |
| [Etcher purchase/Premium](samples/current-behavior-answers.json) | No Premium gate is present in the complete traced NPC purchase paths. Worthy/Tomes gating differs between references. | Global manual sells Etcher to worthy characters. Free use and blank-scroll Premium requirements do not establish the Etcher acquisition Premium predicate. |
| [Filled-scroll consumption](samples/current-behavior-answers.json) | One filled scroll on successful application, zero on ordinary validation rejection; Crystal removes by matching inventory type, Canary removes the used object. | Official blank-scroll inscription consumption is1; that is a separate transaction from filled-scroll application. |

The reference algorithms disagree in ways that affect gameplay: current Mana
ceiling/overkill examples differ from engine rounding/clipped damage; Canary charges
older shrine fees; both Crystal loaders ignore the XML Vibrancy effect type. Canary
reflects additional PvP paralysis before its recovery roll, which conflicts with
the documented removal of reflection at the2018 live release. Timers and coin
ordering also differ. None of these differences is silently selected as Global parity.

Late insertion/debit failures have code paths without compensating refunds. The
architect/coordinator must choose an explicit transaction contract before workers
implement it; copying an OTS failure path is not an integrity guarantee. The
[public research closure](global-research-closure.md) now documents31 bounded
public facts separately from specific unconfirmed fields. The11 original groups
are not11 wholly missing data sets. Published public statements suffice for
literal scoped authoring facts; the27 unperformed captures are supplemental.
This correction does not establish full Global parity or resolve unsourced
consumption, Premium-purchase and numerical-transfer fields.

Video-event qualification: the121.000–122.350icon change is not identified as the namedVampirism application. Equipped permission is a bounded inference from before/after torso placement and the named20:00h/success result, corroborated by the pinned engines; direct equipment state at the application instant is unproved. No exact consumption value is selected.
