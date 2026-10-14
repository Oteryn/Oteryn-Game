# ACCEPT-SPELL-PRESENT-0 Acceptance packet for the spell and combat presentation decision

- Decision: `ACCEPT-SPELL-PRESENT0-V1`
- Status: **ACCEPTED, CONDITIONAL**. The owner confirmed the acceptance directly to the control plane
  (session `session_01XYC239givrMonrYoBwwECV`) in the 2026-10-14 conversation ("2. przekaz ze
  akceptuje", "tak zlecam" and the answer "all,PREMIUM_ACTIVATION,EQUIP-0,CHAT-0,CHAR-VITALS-0"),
  recorded in the control plane STATE comment
  https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-5969258663; no worker report is the source. It takes effect only when the
  PR carrying this status passes exact-head independent review (protocol, combat and determinism)
  and CI on its frozen head and is integrated through the protected merge path; until then
  SPELL-PRESENT-0 is not effective as ACCEPTED, whatever the status line says on an unmerged
  branch. It adds no rule text beyond the one dependency name correction of §2 point 1 (ATTACK-1 to ATTACK-1b) and changes no code, wire, migration, content or sound file.
- Packet for: `reviews/OTERYN_GAME_SPELL_PRESENT0_SPELL_AND_COMBAT_PRESENTATION_DECISION_2026-09-30.md`
  (`SPELLPRES0-SPELL-AND-COMBAT-PRESENTATION-V1`), following ACCEPT-SPELL-NPC-MAP-0 (PR #2084)
  and ACCEPT-CHAR-POSITION-0.
- Answers: control plane request of 2026-10-13 (MAGIC-CAST-1 entry gate, option B).
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What acceptance would make effective

- The CANDIDATE gate lifts for PRESENT-CONTENT-1, PRESENT-WIRE-1, SPELL-PRESENT-1,
  COMBAT-PRESENT-1 and PRESENT-CLIENT-1. Each keeps its own #162 allocation and other
  dependencies (VIS-2, SPELL-D4 composition, ATTACK-1b, COND-1 for ticks). PRESENT-WIRE-1 still
  needs the protocol numbers (capability `PRESENTATION_V1`, domains `WORLD_PRESENTATION` and
  `ACTOR_COOLDOWNS`, rows `SPELLPRES0-RL-01..07`) reserved on #162 before any registry change.
- **Amendment rule (one rule, both packets).** An amendment is approved when its source decision
  is accepted, and its wording is then final: the follow-up changes the source's status line only,
  and the amended documents keep their "pending on acceptance" wording (ACCEPT-CHAT-ATTACK-DEATH-0
  §1). It becomes part of the target's effective text when the target is ACCEPTED: at once if the
  target already is, otherwise when the target is accepted (ACCEPT-SOCIAL-MAP-0 §1: "takes effect
  in that target when the target is accepted"). Accepting a source never accepts its CANDIDATE
  targets.
- Applied: SPELL-D1 §10 (pointer and the result `detail`, §8) and ATTACK-0 §3 (pointer) have
  accepted targets, so they are effective at the acceptance of SPELL-PRESENT-0. CHAT-0 §3
  (pointer) has a CANDIDATE target (CHAT-0 was reopened by ACCEPT-CHAT-ATTACK-DEATH-0's 2026-10-12
  review), so it is approved and final but not effective until CHAT-0 is accepted.
- Owner question P1 is already answered (2026-09-30, #162, option a): the 15.30 client sound
  files are included. Acceptance does not re-ask it.

## 2. Open points the reviewers must settle before the status flips

1. **Brief names ATTACK-1.** The implementation brief listed COMBAT-PRESENT-1's dependency as
   "ATTACK-1"; the mechanics plan (2026-10-13) names ATTACK-1b. **Resolved in the follow-up PR:**
   ATTACK-1b is meant (the plan's COMBAT-PRESENT-1 row names it; "ATTACK-1" is the plan's alias
   for the whole ATTACK-1 split), so the decision's brief row and its §5 block row now read
   ATTACK-1b. The reviewers confirm the correction.
2. **CHAT-0 pending amendment.** The CHAT-0 §3 pointer (spell words to spectators, range ±8 × ±6)
   stays pending on CHAT-0. The reviewers confirm SPELL-PRESENT-1's spell-words emission (§6) does
   not depend on the pointer taking effect, or name what must wait for CHAT-0.
3. **Sound files.** The rights confirmation of 2026-09-30 (P1a) lives in #162. The reviewers
   confirm the quote is recorded where PRESENT-CONTENT-1 can cite it as provenance.
4. **Bounds.** The §13 rows (1,024 events per session per sync unit, 128 bytes per event, drop on
   overflow, never queue) are accepted as stated; PRESENT-WIRE-1 measures them. The reviewers
   confirm no accepted performance requirement is lowered.

## 3. Procedure after the owner confirms

1. The owner confirms the acceptance directly to the control plane (not through a worker report).
2. A follow-up PR changes the Status line of the decision to ACCEPTED, pointing here, corrects
   the dependency name ATTACK-1 to ATTACK-1b in the brief and §5 (§2 point 1), and
   replaces this packet's Status with the conditional wording used by ACCEPT-CHAR-POSITION-0.
3. Exact-head independent review, CI and protected integration, then the control plane lifts the
   MAGIC-CAST-1 gate line for the children of §1.

## 4. Not done here

- The follow-up PR changes only the decision status line and the dependency name ATTACK-1b. SPELL-NPC-MAP-0 is a separate packet (PR #2084).
- No child is allocated and no code, content, sound, migration or protocol-registry change is made.
