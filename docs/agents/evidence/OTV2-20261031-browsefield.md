# Browse field (I18a, I18b, I18c)

## Scope and ownership

- Task: `OTV2-20261031-browsefield`.
- Canonical branch: `chatgpt/browsefield`; one publishing lead, three path-disjoint workers.
- Base inspected: `80dca5fd1578d029f953f05690c6246a0236fd16`.
- Primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
- Deliverable: one draft pull request; no review request, heavy-tests label or merge.
- The user requests the first draft immediately and decisions through QUESTION/ANSWER comments.

## Verified starting point

The current container-view owner is command 21/domain 14. Command 9 reaches the existing
session-fenced durable item loop and its receipts. Browse fields are ephemeral views of tiles;
they must not create persisted virtual containers or a second item mover.

Crystal uses a 30-entry page (`container.cpp:55-70`), same-floor adjacency and walking followed
by retry (`game.cpp:5021-5100`), and validates aligned page seeks (`game.cpp:5175-5191`).
Ground-container Up reaches the tile (`game.cpp:4473-4503`).

## Planned numbers and validation

The assigned block is capabilities 230-231, commands 440-443, mechanics/offers 580-583,
migrations 0640-0643. The intended minimal extension is capability 230 and command 440,
with existing Up/Close and durable command 9 retained. No migration is planned.
The main-only number scan has no occupation of this block; the open-PR scan is pending.

Focused checks will cover codec bounds, donor rules, real dispatch, durable readback/replay,
view refresh and range close. Rust and PostgreSQL validation will use an isolated checkout
on the user-authorized computer because the scratch executor lacks Rust and its Git proxy
is unavailable. Results are pending, not passed.

## Open decisions

Donor drop-into-view behavior and existing general item-mover limitations require explicit
QUESTION comments on the draft. Independent implementation continues while they are open.

## Publication

The user allocated this exact task branch to the lead. Local Git authoring is isolated.
The bound META policy 3.1.0 permits a new API candidate through the bounded connector Git
Data route when normal publication is unavailable: fresh predecessor, sole-parent successor,
one non-force update and readback. Candidate validation is rerun after publication.
