# ACCEPT-CHAR-VITALS-0 Acceptance of the durable health and mana decision

- Decision: `ACCEPT-CHARVITALS0-V1`
- Status: **ACCEPTED BY THE OWNER 2026-10-13 VIA THE CONTROL PLANE**. Owner `blakinio` accepted CHAR-VITALS-0 (owner accepted via control plane). The acceptance takes effect only when this PR passes exact-head independent review and CI on its frozen head and is integrated through the protected merge path. It adds no rule text and changes no code, wire, migration or content.
- Accepts: `reviews/OTERYN_GAME_CHAR_VITALS0_DURABLE_VITALS_DECISION_2026-10-13.md` (`CHAR-VITALS0-DURABLE-VITALS`), following ACCEPT-CHAR-POSITION-0.
- Accepted version (immutable pin): the file as it lands on `main` through PR #2045, git blob `b3ebdfea95d2a15d122740231d856df299dd0c4a`, SHA-256 `ab5aa2afe9e9dfdc38f4387289dcb823f56de1cd81859fe0a0d8296fcdfca760`. The acceptance covers exactly that content. If the file differs from this blob when it lands on `main`, this acceptance does not cover the difference and the owner must accept it again.
- Pin amendment (PR #2045, 2026-10-14): the original pin was the file on the head of PR #2045 at commit `14f335e67ae4b9ade923867154d137cdc9800648`, git blob `13f8ebea3b7401f8ec0894b3e9ead0a1012fa94d`, SHA-256 `6d75ed6823f439e8c335b0c519ba131529fd9575052c3dd6a38348a295685367`. The new blob differs from it only in the status line, which now reads ACCEPTED pointing here, as §2 directs; no rule text changed. The owner reconfirmed the acceptance to the control plane on 2026-10-14.
- Answers: CHAR-VITALS-1 (PR #2045) cites the decision while it still reads CANDIDATE.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What becomes effective

- Once the condition in the status line is met, the CHAR-VITALS-0 status line reads ACCEPTED, pointing here. No other rule line changes.
- Amendments that other decisions wrote as "pending on acceptance" of CHAR-VITALS-0 keep their wording. Each takes effect only when its own source decision is accepted.

## 2. Acceptance record

- Owner: `blakinio`. Date: 2026-10-13. Basis: owner accepted via control plane.
- Review history in the decision stays as written. Implementation children keep their own exact-head validation, independent review and protected integration.
- The decision file lives on the branch of PR #2045 and is not on `main` yet. This acceptance does not copy it. Its status line reads ACCEPTED, pointing here, in the PR that lands the file (#2045) or in a follow-up once both are on `main`.
