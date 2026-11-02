# RAIDS-GLOBAL

Status: AUTHORING — catalogue and source rule foundation published; live integration in progress.
Branch: `chatgpt/raids`
Base: `38641fecd4558a4a14e6b15b3874110ffa2dff19`
Owner decision: 2026-11-01 attached PROMPT_22_raids.md.

## Outcome

Generate the complete evidenced raid catalogue and connect the existing node scheduler,
durable raid runner, live monster owner and player message owner. Unknown source fields or
unbound areas stay explicit inactive records. Capability149 remains unoffered.

## Catalogue checkpoint

The deterministic generator verifies every staged apply-plan SHA256, the pinned Crystal capture,
and the existing native world coordinate binding. The generated catalogue has239records:
135active and104inactive. See [the full catalogue and exact inactive reasons](RAIDS-GLOBAL-CATALOGUE.md).
All166 captured Crystal source files are pinned to201189f871d23597b82caf88fd611454a2ddefb8.
Source maxima require500 creatures,50waves and512 catalogue/candidate records; existing
resource declarations0652/0653/0655 carry those sourced bounds.

## Implementation sequence

A. Catalogue, provenance, typed loader and deterministic source rule foundation.
B. Realize durable wave identities through the existing live monster owner.
C. Deliver ordered announcements through the existing connected-player message path and client.
D. Complete, clean up and fence old-generation runs; preserve sourced end rules.
E. Drive active catalogue schedules from current World and player activity evidence.

The node replacement is integrated with the fenced scheduler/effect contracts in the next
implementation commits; this checkpoint does not claim the production loop is complete.

## Evidence and decisions

Primary donor: Crystal201189f871d23597b82caf88fd611454a2ddefb8. Accepted BOSS-RAID-0 remains
the source for native durability and corrected interval probability. Withdrawn generic
raid-content-1 schedules stay withdrawn. Separately registered numbered variants preserve their
source schedule identities and carry a family link, pending the coordinator answer.
Questions and answers are on draftPR2799. Only the task lead publishes this branch.

## Checks on this catalogue checkpoint

- `python3 -m unittest -v test_raid_catalogue` in tools/content-migration:15passed.
- `python3 raid_catalogue.py --check` in tools/content-migration:passed.
- `cargo test -p oteryn-gameplay-encounter-rules --lib raid::`:35passed,0failed,0ignored.
- `cargo clippy -p oteryn-gameplay-encounter-rules --all-targets --all-features -- -D warnings`:passed.
- `cargo fmt --all -- --check`:passed.
- `git diff --check`:passed.
- `python3 .claude/skills/oteryn-dev/scripts/next-free-numbers.py`:reserved block free;
  exit1 due to an unrelated704–707 clash betweenPR2782 andPR2793.

PostgreSQL execution, full pre-push validation, regeneration and CI belong to the coordinator.
No PostgreSQL-backed execution or full-workspace tests are claimed.

## Remaining

Live integration, client broadcast rendering, source failure/end handling, World aggregate
activity gates, focused checks on those changes, and final merge of latest main remain in progress.
