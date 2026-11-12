# MAINRED1107 repair evidence

Base: `4508fd2f1d31ba316792ec05a968f9101daa67f9`.

## Node boot provisioning

The native manifest pins cosmetics, starting-loadout item profiles and item
profile updates. Node boot and the deployment/test stagers omitted those
three providers, causing `NativeGameplayInput::from_manifest` to report
`native gameplay provisioning file` before the operator's expected grant
refusal. Their staging allowlists now include all three known file pins;
repository containment and exact SHA-256 checks remain mandatory.

Red: `python3 -m unittest test_qualification_tools.StagingTests` failed on
missing cosmetics; the new test executing the actual node-boot staging
here-document reproduced the same omission (0.137 s). Green: from
`tools/qualification/spells`, `python3 -m unittest test_qualification_tools`
passed all 16 tests in 0.718 s. `bash -n tools/qualification/node_boot/run.sh`
and `git diff --check` passed. Full node boot requires `sudo`, which is absent
in this environment; the actual staging code is covered by the regression.
No migrations or protocol numbers are allocated.

## Embedded reward claims

The three extra candidates come from ITEMHOLD-Q `ca5a91b59` (#3017):
`kv.ornamentedshield` and `kv.triangletower` gained qualified native container
recipes; `kv.powerring2` became a ready full-charge leaf. The embedded shards
are current. Their integration expectation was stale: 308 candidates became
311, and Power Ring leaving the waiting set changed 11 plain holds to 10.
The test now also names each new claim and requires exactly one unbound
placement and no left-out disposition for it. No source data or binding
predicate was relaxed.

Red: `cargo test --locked -p oteryn-game-server --test integration_rest
world_reward_claims_the_embedded_shards_decode -- --nocapture` reproduced
311 versus 308 (0.07 s); after the first assertion it also exposed 10 versus
11. Green: the same command passed (0.05 s; 3.660 s including Cargo).

## Native combat presentation fixture

`ffef24ad6` (#3011) made effect 1 render with its donor red colour, while the
older synthetic fixture still required a yellow effect. The fixture keeps
DRAWBLOOD (1), adds FIREWORK_YELLOW (29), and now checks exactly four red
border quads and eight yellow sparks as well as damage/mana/heal colours.
No renderer mapping or colour assertion was removed.

Red: `cargo test --locked -p oteryn-synthetic-client-harness --test
native_combat_presentation damage_heal_and_hit_effect_reach_the_play_scene
-- --nocapture` reproduced the missing yellow (0.00 s). The same command is
green (0.00 s; 36.501 s including Cargo). Clippy for all harness targets with
`-D warnings` passed in 19.934 s.

## Server Seam bootstrap ordering

MAINRED14 `9d0925238` assumed ambience always preceded the channel list.
The chat and visibility owners each use independent 250 ms timers, and
Serene can also publish ambience. A channel-first interleaving therefore
has valid contiguous server sequences and exact owner state but failed the
qualification's global ordering assumption. The local regression reproduced
the reported channel-list 1 / ambient 2 / party-icons 3 failure before the fix.

The qualifier now matches expected owners in their received wire order,
requiring exact event counts, payloads and per-domain revisions plus server
sequences 1..N. Missing, duplicate, extra and corrupt owner state is still
rejected. The regression checks all six interleavings and negative cases.
`cargo test --locked -p oteryn-game-server --lib initial_pushes_ -- --nocapture`
went from one failing/one passing case to two passing cases (0.00 s;
17.165 s including Cargo). Full local WP5 runs compiled the server but Docker
failed extracting its pinned PHP layer with `no space left on device`;
no full-topology pass is inferred from the unit regression.

## Full-manifest cosmetics dependency closure

The v6 canonical manifest pins `native-cosmetics.json`, which is a descriptor,
not an inline section. Its authoring, source manifest and source shards must
also be staged. The production reader consumes that closure in
`crates/gameplay/content/src/content/native_gameplay/native_cosmetics/provisioning.rs:44-106`.
The initial top-level pin census missed these transitive files. Node Boot and
the deployment stager now share the explicit closure selector. It preserves
repository-relative locators and enforces every dependency digest, the native
per-file/aggregate bounds, and repository containment before creating output.
Inline historical cosmetics packets keep their original bytes and behavior.

Red: the canonical staging regression failed for both consumers on missing
`content/cosmetics/outfits/authoring.json` (0.585 s). Running the existing
`actual_full_manifest_qualifies_source_world_and_all_owner_profiles` test with
`OTERYN_FULL_SPELL_TEST_MANIFEST` pointing at the old staged full manifest
reproduced `InvalidArtifact("native gameplay provisioning file")` (0.00 s;
13.600 s including Cargo). Green: all 18 provisioning tests pass in 1.020 s,
including both real stagers and digest, traversal and symlink refusal cases.
No manifests, content digests, migrations or protocol numbers changed.
The same production full-manifest qualification test passes against the new
staged directory in 166.43 s (166.757 s including Cargo).
