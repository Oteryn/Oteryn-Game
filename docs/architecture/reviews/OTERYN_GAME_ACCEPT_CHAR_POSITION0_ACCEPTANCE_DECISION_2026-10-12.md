# ACCEPT-CHAR-POSITION-0 Acceptance of the durable logout position decision

- Decision: `ACCEPT-CHAR-POSITION0-V1`
- Status: **ACCEPTED BY THE CONTROL PLANE 2026-10-12**. CHAR-POSITION-0 reads ACCEPTED. It adds no
  rule text and changes no code, wire, migration or content.
- Accepts: `reviews/OTERYN_GAME_CHAR_POSITION0_LOGOUT_POSITION_DECISION_2026-09-30.md`
  (`CHAR-POSITION0-LOGOUT-POSITION-V1`), following ACCEPT-CHAT-ATTACK-DEATH-0.
- Answers: CHAR-POSITION-1 (PR #2006) implements the decision while it still reads CANDIDATE.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What becomes effective

- The CHAR-POSITION-0 status line reads ACCEPTED, pointing here. No other line changes.
- Amendments that other decisions wrote as "pending on acceptance" of CHAR-POSITION-0 keep their
  wording. Each takes effect only when its own source decision is accepted, as
  ACCEPT-SOCIAL-MAP-0 §1 rules.
- The protocol registry and the cross-repository lock carry no status for this decision, so
  neither changes.

## 2. Not done here

- The persistence implementation (CHAR-POSITION-1, PR #2006) keeps its own exact-head validation,
  independent persistence review and protected integration.
- HP and mana persistence stay undecided; CHAR-POSITION-0 covers position only.
