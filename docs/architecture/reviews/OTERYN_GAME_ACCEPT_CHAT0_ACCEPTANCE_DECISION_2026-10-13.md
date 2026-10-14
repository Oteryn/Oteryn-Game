# ACCEPT-CHAT-0 Acceptance of the player chat decision

- Decision: `ACCEPT-CHAT0-V1`
- Status: **ACCEPTED BY THE OWNER 2026-10-13 VIA THE CONTROL PLANE**. Owner `blakinio` accepted CHAT-0 (owner accepted via control plane). The acceptance takes effect only when this PR passes exact-head independent review and CI on its frozen head and is integrated through the protected merge path. It adds no rule text and changes no wire, migration or content. It also changes `ChatRuntime::pre_gate` in `apps/game-server/src/gameplay_transport/chat_intent.rs` and its test, so a session without chat state gets the early `CHAT_UNAVAILABLE` refusal (security re-review finding).
- Accepts: `reviews/OTERYN_GAME_CHAT0_PLAYER_CHAT_DECISION_2026-09-30.md` (`CHAT0-LOCAL-PRIVATE-WORLD-ROOMS-V1`), following ACCEPT-CHAR-POSITION-0.
- Answers: the CHAT work (PR #2076) cites the decision while it still reads CANDIDATE.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What becomes effective

- Once the condition in the status line is met, the CHAT-0 status line reads ACCEPTED, pointing here. No other rule line changes.
- Amendments that other decisions wrote as "pending on acceptance" of CHAT-0 keep their wording. Each takes effect only when its own source decision is accepted.

## 2. Acceptance record

- Owner: `blakinio`. Date: 2026-10-13. Basis: owner accepted via control plane.
- Review history in the decision stays as written. Implementation children keep their own exact-head validation, independent review and protected integration.
- Open re-review, stated plainly: the 2026-10-12 review of ACCEPT-CHAT-ATTACK-DEATH-0 (head `a1d2ee2d8b9f`) found protocol PASS, privacy PASS, security DEFECT. The decision text records both defects as resolved in code and text (CHAT-MUTE-CAST-1 #2003; the pre-lock gate by DEATH-WIRE-1 and §6) but waits for a re-review that has not happened. The owner accepts CHAT-0 with that re-review still open, knowingly and without a waiver of the security gate on the implementation children. On 2026-10-14 the security re-review on this PR found one more defect (an under-level yell bypassed the pre-lock gate); CHAT-YELL-PREGATE-1 (#2103) fixed it on `main`. The owner then ruled: no waiver, this PR waits for a security re-review that passes. It merges only after a security re-review of its head, which includes #2103, finds no defect.
