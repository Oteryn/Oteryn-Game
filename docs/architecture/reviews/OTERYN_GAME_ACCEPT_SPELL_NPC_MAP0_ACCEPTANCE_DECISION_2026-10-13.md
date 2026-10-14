# ACCEPT-SPELL-NPC-MAP-0 Acceptance packet for the spell and NPC handoff mapping decision

- Decision: `ACCEPT-SPELL-NPC-MAP0-V1`
- Status: **ACCEPTED, CONDITIONAL**. The owner confirmed the acceptance directly to the control plane
  (session `session_01XYC239givrMonrYoBwwECV`) in the 2026-10-14 conversation ("2. przekaz ze
  akceptuje", "tak zlecam" and the answer "all,PREMIUM_ACTIVATION,EQUIP-0,CHAT-0,CHAR-VITALS-0"),
  recorded in the control plane STATE comment
  https://github.com/Oteryn/Oteryn-Game/issues/1622#issuecomment-5969258663; no worker report is the source. It takes effect only when the
  PR carrying this status passes exact-head independent review (spell and content) and CI on its
  frozen head and is integrated through the protected merge path; until then SPELL-NPC-MAP-0 is
  not effective as ACCEPTED, whatever the status line says on an unmerged branch. It adds no rule
  text beyond the §3 step 2 correction of the two M3 references, and changes no code, wire,
  migration or content.
- Packet for: `reviews/OTERYN_GAME_SPELL_NPC_MAP0_SPELL_AND_NPC_HANDOFF_MAPPING_DECISION_2026-10-04.md`
  (`SPELL-NPC-MAP0-V1`), following ACCEPT-CHAR-POSITION-0 and ACCEPT-SOCIAL-MAP-0.
- Answers: control plane request of 2026-10-13 (MAGIC-CAST-1 entry gate, option B). Every
  SPELL-RT-* child of MAGIC-CAST-1 waits for this decision.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. What acceptance would make effective

- The CANDIDATE gate lifts for SPELL-RT-SCHED-1, SPELL-RT-MONK-1, SPELL-RT-MONKAI-1,
  SPELL-RT-STANCE-1, SPELL-RT-GUARD-1, SPELL-RT-TARGET-1, SPELL-RT-CHAIN-1, SPELL-RT-EQUIP-1,
  SPELL-RT-PARTY-1 and SPELL-RT-FIELDBUFF-1. Each keeps its own #162 allocation and its other
  dependencies (the S7 reader, ATTACK-1b, COND-1, PARTY-1, SKILLS-0, ITEM-SEM-2b, WORLDINT-ADMIT-1).
  SPELL-RT-PREM-1 still waits for PREMIUM-1.
- **Amendment rule (one rule, both packets).** An amendment is approved when its source decision
  is accepted, and its wording is then final: the follow-up changes the source's status line (and, for SPELL-NPC-MAP-0, the two M3 references of §3 step 2),
  and the amended documents keep their "pending on acceptance" wording (ACCEPT-CHAT-ATTACK-DEATH-0
  §1). It becomes part of the target's effective text when the target is ACCEPTED: at once if the
  target already is, otherwise when the target is accepted (ACCEPT-SOCIAL-MAP-0 §1: "takes effect
  in that target when the target is accepted"). Accepting a source never accepts its CANDIDATE
  targets.
- Applied: the TRAVEL-0 §4 amendment (the optional `departure_text` route field, M8) and the
  native-behaviours candidate D.5.3 test 4 amendment (the Paralyze caster effect, M4) both have
  CANDIDATE targets, so each is approved and final but not effective until its target is accepted.
  This packet accepts neither target.
- Scope of the acceptance: rulings M1, M2, M4-M9. The source decision cites "M1-M3" (header and
  decision test) but has no M3 section: it numbers M1 (§2), M2 (§3), then M4 (§4). This packet
  does not count M3 as a ruling; the numbering gap is open point 5 of §2. M1, M2 and M5-M7, M9
  apply accepted decisions and add no contract; M4 and M8 are the only contract rulings, and M4
  stays `PARITY_PENDING` under S24.
- Every local batch that §9 of the source gates (the spell batches r59-r62 and any of r28-r58 not on main, the NPC combined patch, the travel data and the item candidates) may merge as a data PR; each stays held and activates nothing.

## 2. Open points the reviewers must settle before the status flips

1. **Premium row.** §3 of the decision lists the Premium queue owner as "PREMIUM-ACTIVATION
   (merged)". The mechanics plan (2026-10-13) records PREMIUM_ACTIVATION as CANDIDATE and PREMIUM-1
   as open. The word "merged" must be read as "PREMIUM_ACTIVATION0 packets", or corrected; either
   way no second Premium runtime follows.
2. **Evidence level of M4.** The Paralyze ruling rests on a donor source (S24 hypothesis). The
   reviewers confirm `PARITY_PENDING` is the right level to accept at.
3. **Amendments.** The reviewers confirm TRAVEL-0 §4 and D.5.3 test 4 read correctly once their
   targets are accepted, that no other accepted text contradicts M4 or M8, and whether the targets
   (TRAVEL-0, the native-behaviours candidate) are accepted separately or later.
4. **Dependencies named as accepted.** M1 and M2 rely on S7, S23-S27, CONDITIONS-0 (CANDIDATE),
   CREATURE-AI-0 §8 (CANDIDATE), FAMILIARS-0 (CANDIDATE) and STANCE-0 (CANDIDATE). The reviewers
   confirm the rulings only cite shapes those documents accepted into S27 or the merged code, and
   that neither M1 nor M2 becomes effective ahead of its source.

5. **M3 numbering.** The source cites M1-M3 but has no M3. The reviewers confirm no ruling is missing and the source header and decision test are corrected in the follow-up PR before or with the status flip (§3 step 2), not here.

## 3. Procedure after the owner confirms

1. The owner confirms the acceptance directly to the control plane (not through a worker report).
2. A follow-up PR changes the Status line of the decision to ACCEPTED, pointing here, and corrects the two references to the nonexistent M3 (the header line "Rulings M1-M3 and M9" becomes "M1, M2 and M9"; the decision test line "M1-M3 and M5-M7, M9" becomes "M1, M2, M5-M7 and M9"), with no other edit to the decision, and
   replaces this packet's Status with the conditional wording used by ACCEPT-CHAR-POSITION-0.
3. Exact-head independent review (spell and content), CI and protected integration, then the
   control plane lifts the MAGIC-CAST-1 gate line for the children of §1.

## 4. Not done here

- The follow-up PR changes only the decision status line and the M3 references. SPELL-PRESENT-0 (the other CANDIDATE gate of MAGIC-CAST-1) is a
  separate packet.
- No child is allocated and no code, content, migration or protocol-registry change is made.
