# R59 — complete typed target data for the remaining state lanes

The exact 43 remaining r55 Wheel/stance/chain source keys now have 41 concrete Spell/dependency target data bundles and two Sap Strength reference rows. Sap Strength stays reference-only under accepted S24, with its original current source header and provenance; no executable is invented for a removed spell.

This package uses a versioned private authoring schema, `urn:oteryn:source-state-native-extensions:1`. It exports closed `$defs.nativeBehavior` branches with a unique r59 `parameters.source_model` tag and `$defs.sourceStateContract` for the proposed `Spell.source_state_contract` field. Shared v1 schemas and Rust/runtime files remain unchanged. A private-schema valid bundle is **CANDIDATE_SCHEMA_VALID** as target DATA with **authoring_contract_extension_pending=true**, **source_consumer_implemented=false**, and runtime/native flags false. It is not currently admitted by the unchanged native reader's full-profile identity guard.

Concrete models reuse accepted S27 native parameter families for avatars, Wheel attacks, Mass Spirit Mend, Magic Shield, delayed Divine Grenade, owned Divine Empowerment fields, and the canonical Canary stance. These accepted canonical parameter values are separate from byte-exact current source headers. Current source equivalence is expressly false; S27/S5/S6/S21/S23 normalization is not a claim that every current donor value matches the canonical target. Current source programs remain in immutable r38 evidence.

The private extensions contain actual missing fields, rather than Lua or a generic program shell:

- Crystal stance slot, exact attribute parameters and effect IDs, player/non-player routes, condition-removal/clear/POFF order and set-before-Combat order. Concrete C++ helper data covers elemental conversion/bonuses, aura duration and damage modifiers, Divine Defiance dodge/specialized magic, Elemental Synthesis, and Shared Conservation's dependency on Nature's Embrace's secondary selector. Helper source files and accepted-model producers are SHA-pinned in the proof.
- Great Death Beam preserves the real Canary alias: all three grade entries point to the same Combat and therefore its final eight-row area. Crystal has independent six/seven/eight-row areas; its zero grade selects grade one and casts, rather than refusing. Its separately owned Beam Mastery flank matrices/factors, per-callback stage reads, primary-then-flank execution and returned primary result are explicit. Elemental retuning runs before the grade read.
- Lightning has a standard S23 sequential-chain Ability plus the concrete initial-selector extension: explicit target chains, direction-only casting uses a single target. Raw total three/radius five and accepted additional two/radius four are separate values.
- Optional Wheel area/damage/healing/cooldown parameters sit in separately owned augmentation data under S6, outside the plain base native body. Revelation-stage gates and values remain in their actual gated models.

Every raw `source-header.json` is copied exactly from r28. `receipt.json` retains the standard receipt shape; the separate `projection-receipt.json` carries private-contract, source-provenance and provider limits. Summary and lane audit include all 43 keys. No native profile alias, activation, canonical selection change, identity allocation or provider-equivalence claim occurs.

```sh
/workspace/spell-tools/bin/python tools/content-schema/spell-authoring/project_state_native_extensions.py --repo .
/workspace/spell-tools/bin/python -m unittest discover -s tools/content-schema/spell-authoring -p test_project_state_native_extensions.py -v
```

Ten tests pass, covering exact population/header retention, every closed private target shape, arbitrary-key/field refusal, source shared-Combat alias and zero-grade branches, exact stance conditions/order, canonical Magic Shield capacity/expiry/damage routing, Lightning's direction selector and separate optional Wheel augments. The two closure tests check both field Item references and immutable pinned helper bytes/scopes. These checks validate authoring data and do not execute the future consumer.
