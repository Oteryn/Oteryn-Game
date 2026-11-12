# CLX-C — client progression and world panels

Base: `e91bce557728e52f5d61c38f8fad7dba3c1ee1d3`.
Client-only task: no production server changes, migrations, protocol numbers,
capabilities, command IDs, generated files, pins or goldens.

## Row disposition

| Row | Disposition | Evidence / boundary |
| --- | --- | --- |
| P18 | DUPLICATE-ON-BRANCH | Owner-supplied CL5 head `5b4376f89722e7ba2cedebfa6c43e55164e755c4` reserves Forge reply decoding, state, view, native panel and play tests. Excluded paths left untouched; implementation and test results on that head were not independently reviewed here. |
| P05 | DUPLICATE-ON-BRANCH | Same CL5 head reserves Task Board/talisman protocol, native panel and play tests, including `protocol/talisman.rs`. Excluded paths left untouched. |
| P04 | DUPLICATE-ON-BRANCH | Same CL5 head reserves Task Board input, protocol, state, view, native panel and play tests. This is an overlap disposition based on the owner's supplied file list, not an independent claim that weekly/delivery/shop acceptance passed there. |
| W09 | STOPPED — server projection needed | The existing Cyclopedia Misc wire exposes only `WealthDuplexSeconds`; `OtherConcoctions` is explicitly unavailable (`crates/protocol-oteryn/src/cyclopedia_character/combat.rs:134-137`). The production query reads only active `wealth_duplex` expiry (`crates/gameplay/durability/src/durability/cyclopedia_character/combat.rs:68-98`). There is no full active Drome potion/duration projection to decode. Showing all durations requires a server contract/producer change, outside this task. The existing Cyclopedia native paths are also reserved by CL5. |
| P08 | STOPPED — server projection needed | Capability 195 exposes selection/modification commands. `SelectPerkReply` contains the result and selections (`crates/protocol-oteryn/src/weapon_proficiency.rs:87`); `ModificationReply` contains the result and commanded slot (`:287`). Neither carries experience or the complete proficiency catalogue/tracks; no query route supplies them. Do not infer XP or copy a static donor catalogue into the client. |
| P19 | STOPPED — server balance projection needed | Capability 104 already carries `Binding.forge_stacks` (`crates/protocol-oteryn/src/appearance.rs:88`), but no read-only current dust-balance projection exists. Forge history is a list of retained operation receipts, not the current wallet (`docs/contracts/protocol-oteryn/v1/item-forge-v1.md`, Reply). A creature kill can change dust after the last operation, and a character can have no Forge history. A history receipt must not be presented as live balance. The full row needs a server-owned current balance read/snapshot. Per the task boundary, this row stops before implementing an incomplete UI. |
| W06 | DONE (production play/model test; Windows UI not executed) | A native click on a volatile handled portal was swallowed as an object selection. Portal appearance 54133 now passes through to the existing walking input, which reaches server step-in activation. The client regression also exercises portal removal and scroll/catalyst reward entries in the existing native container model. No local reward or encounter rules are introduced. |

The supplied SRV3 head is `c97858c3f440fd26676d3bc9be6d6a35b2c711ff`;
none of its reserved files are edited. The owner reports that QPKG3 does not
exist remotely; quest content remains excluded. Casting S17 is untouched.

## W06 source parity and production callers

| Rule / presentation | Evidence | Disposition |
| --- | --- | --- |
| Portal appears as an encounter object and expires under the server owner | Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, `data-global/lib/core/echo_warden.lua:203-224`, recorded in `docs/reference/echo-outcome-families/SV16.md` | Existing server behavior retained. Donor class: `OTS_HYPOTHESIS_ONLY`. |
| Warden basic scroll reward | Same Crystal pin, `data-global/scripts/custom/echo_warden/ondroploot_echo_warden.lua:10,14-24`, recorded in the same reference | Existing durable reward/corpse owner retained. The PG case proves actual scroll and catalyst custody and replay; compact client definition references are not donor item IDs. |
| Portal activation is movement, not item USE | `crates/gameplay/dispatch/src/gameplay_transport/echo_warden.rs:77`; existing `step_in` production caller and `append_portals` / `portal_entry` projection | Existing accepted native implementation retained. No new mechanic decision. |
| Echo portal appearance identifies a step-in object | `crates/gameplay/dispatch/src/gameplay_transport/echo_warden.rs:195-217` emits `appearance_id("oteryn:item.tibia.i54133", None)`; `crates/gameplay/world/src/map/view.rs:189-204` maps that key to 54133 | Keep rendering the portal, omit only this appearance from item selection, and let the existing click route walk onto its tile. Other handled and base-ordinal items remain selectable. |

Production client caller: `PlayView::set_live_map` calls `map_targets`; scene
selection feeds the existing `PlayView::click` walking route. Native container
windows read `ContainerPanel::current().entries`, the same model asserted by
the play regression. Native Windows execution was not available in this Linux
workspace; the cross-platform production play/model path is tested here.

## Red/green and review

`cargo test --offline --locked -p oteryn-client echo_portal_click_walks_and_reward -- --nocapture`
ran red before the production fix: the click queued `None` instead of
`Some(TargetWalk(ActorPosition { x: 101, y: 200, floor: 0 }))`.
It then ran green after the fix. The final case also retains two reward entries
(scroll and catalyst) and clears the container model at session end.

The existing PostgreSQL encounter scenario is extended with CLX-C assertions
before and after committed portal entry, for both unfinished and completed
Bestiary paths. They call the production `observe_world_map` route and strict
wire decoder, resolve the portal using the active Item generation, and verify
that its count is one, origin is a session handle, appearance is 54133, and it disappears on entry.
The same scenario executes real deaths, reward custody and replay checks.

Self-review scope: all changed files. No authority, value-moving transaction,
session fence, bound, receipt, codec, capability offer, generated file or existing
assertion is weakened. No excluded path is changed. The production fix only
removes the step-in portal appearance from the actionable target list; rendering still
uses the complete map snapshot. Server changes are test assertions only.

The first PG projection assertion exposed an incorrect test assumption: the actual
portal has `MapOrigin::Handle`, not `DisplayOnly`. The client fixture was corrected
to that production shape and run red again before the portal-specific fix.
The PG check now asserts the actual origin variant plus exact appearance and count;
no original assertion was removed or weakened.

## Final validation

All Cargo/PG shells source `/workspace/oteryn-env/env.sh`. Dependencies stayed
offline; no tools were installed. The tested Rust sources are the final sources.

| Command / exact filter | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS. |
| `cargo clippy --offline --locked -p oteryn-client -p oteryn-gameplay-dispatch --all-features --all-targets --quiet -- -D warnings` | PASS. Existing dependency warnings do not change the successful exit. |
| `cargo test --offline --locked -p oteryn-client --quiet` (no filter) | PASS: 281 unit tests and 17 integration tests; unit duration 17.05 s. Includes `echo_portal_click_walks_and_reward_remains_in_native_container`. |
| `env -u OTERYN_TEST_POSTGRES_ADMIN_URL cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features echo_warden --quiet` | PASS: 3 tests. Route-less run; this is not live PG execution evidence. |
| `cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features echo_warden_all_damagers_first_species_replay_and_real_scroll_custody_postgres -- --nocapture` | PASS: 1 test, 95.18 s; scenario timer 95.173596913 s. PostgreSQL 18.6 route present. Four new `CLXC_ECHO_PORTAL_WIRE POSTGRES_ASSERTION_EXECUTED` markers (`true`, `false`, `true`, `false`) prove the added assertions ran. Existing scroll/catalyst custody, replay and Charm reward checks also completed. |
| `CARGO_NET_OFFLINE=true oteryn-architecture-check workspace .` | PASS: `workspace-boundaries: PASS`. Used the prepared source-freshness wrapper. |
| `git diff --check` | PASS. |

An initially started unfiltered route-less dispatch suite was interrupted to
keep dispatch validation focused on Echo. It is not counted as passing.
No full nextest or full PostgreSQL suite was run. No commit or push was made.
No capability bits, command IDs, protocol messages or migrations were allocated.
