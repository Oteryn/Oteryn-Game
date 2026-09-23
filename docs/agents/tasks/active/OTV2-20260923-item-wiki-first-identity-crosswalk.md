# OTV2-20260923-item-wiki-first-identity-crosswalk

\`\`\`yaml
task_id: OTV2-20260923-item-wiki-first-identity-crosswalk
title: Wiki-first TibiaWiki Item identity crosswalk
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/item-wiki-first-identity-crosswalk-20260923
pr: 807
base_sha: c07240d50473b8697cbe10641028cd0e7eb2d1e4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous wiki-first Item identity crosswalk writer"
created_at: 2026-09-23T23:19:00+02:00
updated_at: 2026-09-23T23:41:16+02:00
execution_policy: continuous_progress
owned_paths:
  - tools/reference-world-corridor-census/item_wiki_first_identity_crosswalk.py
  - tools/reference-world-corridor-census/item_wiki_first_identity_crosswalk_self_test.py
  - .github/workflows/item-wiki-first-identity-crosswalk.yml
  - docs/agents/evidence/OTV2-20260923-item-wiki-first-identity-crosswalk.json
  - docs/agents/tasks/active/OTV2-20260923-item-wiki-first-identity-crosswalk.md
public_contracts: []
depends_on:
  - "PR #803 protected wiki-first Item census"
  - "PR #763 protected Oteryn/Crystal Item identity closure"
blocks:
  - WIKI_FIRST_ITEM_FIELD_VERIFICATION
cross_repository_coordination_id: null
external_repositories:
  - "zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a (read-only pinned evidence)"
\`\`\`

## Outcome

Map each of the 6,918 protected wiki-first TibiaWiki Item pages to the existing protected Oteryn Item identity space when evidence is sufficient, with one explicit disposition per page: \`EXACT_MATCH\`, \`PROBABLE_MATCH\`, \`AMBIGUOUS\`, \`CONFLICT\`, \`NO_MATCH\`, or \`ALIAS_OR_DUPLICATE\`.

The full 6,918-row crosswalk remains workflow/scratch evidence. The repository retains only the deterministic compiler, synthetic tests, qualification workflow and compact evidence manifest.

## Architecture and source of truth

- **PROVEN:** PR #803 merged as protected \`main@c07240d50473b8697cbe10641028cd0e7eb2d1e4\`; its wiki-first source population is 6,918 pages with stable digest \`389875abd364aa9bcb0b09a591989c82ece5098d63b3c23376274048f6ac2f5a\`.
- **PROVEN:** protected #763 classification crosswalk full scratch SHA-256 is \`004948eeda07afb20d5560ec583eaa2a32397f19f891a7d8749962bc32fa0f8d\` and binds the complete 38,157 protected Item identities.
- **PROVEN:** exact Crystal source is \`zimbadev/crystalserver@ff7ede593c69d4c658b382c97443e8155926924a:data/items/items.xml\`, SHA-256 \`c847293e980b40ec146e2b7f68a62366513a1c0566d16b7c3a011136087021eb\`, classified \`OTS_HYPOTHESIS_ONLY\`.
- **PROVEN:** TibiaWiki remains \`STRUCTURED_REFERENCE_DATA\`, not automatic gameplay truth.
- **DERIVED:** wiki page title / Infobox name / explicit alias may discover exact-name Crystal candidates, but name-only evidence is insufficient for \`EXACT_MATCH\`.
- **DERIVED:** only already-established structurally equivalent integer fields are admitted as independent corroboration in this generation; conflicting donor observations are ignored rather than resolved by source order.

## High-risk authority/recovery qualification

\`\`\`yaml
applicable: false
reason: deterministic read-only identity evidence compilation; no production/runtime/durable mutation authority
\`\`\`

## Acceptance criteria

- [x] Discovery starts from the 6,918 protected wiki-first pages, not the 38,157 Crystal identities.
- [x] Protected census stable digest mismatch fails closed.
- [x] Protected 38,157 identity closure digest mismatch fails closed.
- [x] Exact pinned Crystal Item catalogue digest mismatch fails closed.
- [x] Name-only evidence never yields \`EXACT_MATCH\`.
- [x] Exact name plus at least one independent matching structural signal can yield \`EXACT_MATCH\`.
- [x] Strong structural contradiction yields \`CONFLICT\`.
- [x] Multiple plausible candidates remain \`AMBIGUOUS\` unless deterministic contradiction elimination leaves one structurally corroborated survivor.
- [x] Zero candidates yields \`NO_MATCH\`.
- [x] Duplicate-target wiki pages are protected by \`ALIAS_OR_DUPLICATE\` handling.
- [x] \`NO_INFOBOX_ITEM\` records remain in the population.
- [x] \`INFOBOX_ITEM_PARSE_ERROR\` records never use the rejected conflicting field as identity proof.
- [x] Output ordering and compilation are deterministic.
- [x] The disposition partition must close exactly 6,918 source pages.
- [x] No new Oteryn Item identity can be minted.
- [x] No semantic field promotion occurs.
- [x] Full hosted wiki-first crosswalk completes successfully and emits a compact deterministic manifest.
- [ ] Retained manifest is committed and reproduced byte-for-byte on the exact final head.
- [ ] Repository-required exact-head qualification is green.
- [ ] Integration uses only the protected Merge Queue route.

## Excluded scope

No balance changes, gameplay-stat mutation, Reference semantic promotion, new Item identities, new Item schema/model, long-form wiki prose, runtime/client/protocol/persistence changes, quest/NPC/map work, or creatures/spells expansion.

## Implementation / findings

The compiler inverts the predecessor direction:

\`6,918 wiki pages -> exact-name candidate discovery -> protected Crystal source IDs -> protected Oteryn native keys -> independent structural corroboration/contradiction -> deterministic disposition\`.

It reuses the protected classification crosswalk and wiki-first census instead of introducing a second identity registry. Structural comparison is deliberately bounded to fields whose cross-source meaning is already explicit: attack, defense, extra defense, range, hit chance, armor, charges and container capacity.

Local synthetic qualification before repository publication: 16/16 tests PASS.

PR #805 remains open and owns only the predecessor census lifecycle path; this task does not modify that branch or archived predecessor path.

## Validation

### Focused

- command/run: \`python tools/reference-world-corridor-census/item_wiki_first_identity_crosswalk_self_test.py\`
- result: PASS 16/16 before publication; hosted exact-head run pending

### Component/integration

- command/run: PR #807 run `35923485090`, job `107392876571`: protected native-map export + #763 classification crosswalk reproduction + live #803 wiki-first census + exact pinned Crystal catalogue + full crosswalk twice
- result: PASS — `EXACT_MATCH=1129`, `PROBABLE_MATCH=2734`, `AMBIGUOUS=482`, `CONFLICT=141`, `NO_MATCH=2432`, `ALIAS_OR_DUPLICATE=0`; exact coverage `16.319746%`; exact+probable coverage `55.839838%`; full output SHA-256 `f5bb9724bff51b99c35789dea986635e73fa9bdc6df77bc7f4e4a105768bed80`; manifest SHA-256 `713e35e411a37c22e7950611277120f8c6e92310a8559fbcbd8ac132d65152fd`; artifact `10777848349`

### E2E

- scenario: identity evidence compilation only; gameplay/runtime E2E is not applicable
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending final evidence commit/freeze
- trigger source: pull_request
- workflow/run/job: first full hosted qualification `35923485090` / `107392876571` PASS on pre-evidence head `02ef01daf0c3f69e63b536b5af1b5748f3964b47`; final exact-head rerun pending
- runner assignment: GitHub-hosted Ubuntu 24.04 for focused/full crosswalk workflow
- classification: content evidence tooling plus read-only Actions workflow
- result: full crosswalk PASS; final aggregate exact-head qualification pending after retained evidence commit

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: first workflow generation had an expression-escaping defect and was repaired before retained evidence; current full-run generation has no matcher/source invariant failure
- verdict: pre-freeze self-review PASS; final whole-diff exact-head review pending after evidence commit

## Independent review

- required: YES — bound META AI review policy classifies material GitHub Actions workflow changes as high-risk/control-plane review surface; one deep review is required after deterministic final-head qualification
- exact head: pending
- method/auditor: pending or NOT_APPLICABLE
- material findings: pending or NOT_APPLICABLE
- verdict: pending or NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #763 #767 #770 #803; #805 is path-disjoint predecessor closeout
- protected auto-merge: forbidden substitute; governed Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

\`\`\`yaml
last_progress: PR #807 full hosted run 35923485090 / job 107392876571 PASS; deterministic 6918-row disposition manifest captured for retained evidence
status: validating
branch: agent/item-wiki-first-identity-crosswalk-20260923
head_sha: 02ef01daf0c3f69e63b536b5af1b5748f3964b47
pr: 807
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: pre-evidence-02ef01d
ci_checks_for_current_head: 3
ci_run_ids: [35923485090, 35923485080, 35923485028, 35923485061]
ci_job_ids: [107392876571, 107392876948, 107392877080]
runner_assignment_state: active
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: retain exact manifest from successful run as compact repository evidence, freeze returned successor head, then perform fresh exact-head qualification and required independent deep review
\`\`\`
