# Tibia Proposal Board: partial whole-board census and follow-up reading

- Research date: 2026-09-24.
- Source: public [Proposal Board](https://www.tibia.com/forum/?action=board&boardid=10), **All Threads** filter.
- Relation: continuation of the [24-thread qualitative review](OTERYN_TIBIA_PROPOSAL_BOARD_RESEARCH_2026-09-24.md), the [PR #571 product audit](OTERYN_EVOLVED_PR571_PRODUCT_AUDIT_2026-09-11.md) and its [192-entry register](OTERYN_EVOLVED_PR571_DECISION_REGISTER_2026-09-11.json).
- Status: **PARTIAL RESEARCH CHECKPOINT**. `IMPLEMENTATION_AUTHORITY: NONE`; `PRODUCTION_AUTHORITY: NONE`. This document does not approve changes to PR #571 or to the accepted Reference target.

## What was actually covered

The board initially displayed **67,444 topics on 2,249 pages**. During the attempt it displayed 67,445, then 67,449 and 67,450 topics. These are observations of a live list, not one frozen snapshot. Each ordinary page had 30 topic rows; the last page, read when the count was 67,445, had five.

| Measure | Verified result |
|---|---:|
| Board pages recorded | **1,425 / 2,249 (63.4%)** |
| Topic rows recorded | **42,725** |
| Distinct thread IDs in recorded rows | **42,713** |
| IDs repeated across page boundaries | **12** |
| Recorded page ranges | **1–561; 758–807; 1131–1198; 1504–2249** |
| Unrecorded page ranges | **562–757; 808–1130; 1199–1503** (824 pages) |

Every recorded page has the expected row count and unique IDs *within that page*. The 12 repeated IDs occur across pages, including several at the boundary of readings on different occasions. Changes in a live pagination order prevent a claim that all initially visible threads were captured. The index records titles, authors and displayed activity/update metadata; it does **not** contain the opening posts or replies for 42,713 threads. Title matches are a queue for reading, not evidence of the content or popularity of an idea.

Six thematic readers then nominated threads from the saved index while page enumeration continued. Eight additional, distinct thread IDs were opened and read at least in part; none duplicates an ID in the earlier 24-thread report. Some candidate pages shifted while the board was live. Further requests stopped when the site displayed **429 Too Many Requests**. The other shortlisted threads remain unread.

## New directly read discussions

The table paraphrases opening posts and the indicated replies. “Oteryn consideration” is a design question, not an accepted feature or an estimate of player consensus.

| Official thread and pages read | Direct observation | Oteryn consideration and limit |
|---|---|---|
| [Client Side UI Scaling](https://www.tibia.com/forum/?action=thread&threadid=4864808), 1/1 | The author wants UI scale independent of OS DPI for limited panel space and legibility at different resolutions. One reply merely signs the request. | Investigate coherent client scale for text, icons, windows and hit targets without changing world zoom or information visibility. One use case and one brief endorsement provide weak demand evidence; thematic match to `B01`, not proven provenance. |
| [Experience and skill loss in current gaming era](https://www.tibia.com/forum/?action=thread&threadid=4999689), 1/1 | The author objects to losing time on death, including disconnect-related deaths, but specifies no replacement rule. No replies were visible. | Keep death penalty and disconnect attribution as explicit, separate policy questions. No numerical change can be inferred from this post. |
| [Tibia's PvP and Guild System — Bring Wars Back!](https://www.tibia.com/forum/?action=thread&threadid=4997084), **first of two pages** | The opening post proposes three PvP stances, damage/retaliation rules, guild consequences and monster-assisted kill attribution. One substantive reply discusses a different fit for Optional PvP; several entries are author continuations or brief endorsements. | High-impact PvP/death/guild policy requiring profile boundaries, abuse analysis and kill attribution. Page two was not read; reply count is not a vote. |
| [Rotten Blood / Soul War boss access rework](https://www.tibia.com/forum/?action=thread&threadid=4999234), 1/1 | The author requests a repeat-travel shortcut after the relevant quest, alternatively after a boss-specific Bosstiary stage. No replies were visible. | Distinguish shortening repeat travel from granting boss access or changing reward eligibility. Evidence is one proposal. |
| [Party Finder](https://www.tibia.com/forum/?action=thread&threadid=4497423), 1/1 | The author proposes a dedicated group-finding channel with status, vocation and XP-compatibility commands. Replies question channel usage and space, suggest automatic role context, and raise trust/loot-settlement concerns. | Test an opt-in party-discovery/lobby flow (`D08`) before a separate chat surface. Joining must not imply location disclosure or automatic loot transfer. Five visible posts cannot establish prevalence. |
| [full equipment sets!!](https://www.tibia.com/forum/?action=thread&threadid=2780780), 1/1 | The author asks for visually matching armor pieces, not saved or switched equipment presets. | A **false title match** for `C01` Equipment Presets; no source attribution. |
| [Diagonal Facing](https://www.tibia.com/forum/?action=thread&threadid=3132475), 1/1 | The author wants diagonal facing/animation; replies raise visual objections. It does not propose simultaneous WASD diagonal movement. | A **false title match** for `C02`; no source attribution. |
| [Show party members with blue dot on minimap](https://www.tibia.com/forum/?action=thread&threadid=3086838), 1/1 | The author proposes a party-member marker; replies discuss different floors and a possible up/down cue. | Strong *functional* match to `B03`/`A08`, with consent, freshness and floor semantics to settle. The PR inventory gives no original thread URL, so matching functionality does **not** prove that this is the source. |

Economy and progression readers reached a 429 page before opening any of their nominated posts. Their title queues are not added as substantive findings. Several candidates for `B/C/D` are only superficially similar, as the two false matches above demonstrate. `A01–A18` remain the only batch traced to one exact source thread in the earlier review.

## Continuation and decision boundary

1. When ordinary access returns, continue the three missing ranges from their first page. Check row counts, page numbers and cross-page duplicates, then compare the live board count; a page census alone cannot certify a fixed-time topic population on this changing board.
2. An offline queue nominates 78 distinct, still-unread thread IDs across eight product clusters from the **partial** title index. Read opening posts and dissenting/qualifying replies before extending product findings. Record the exact thread ID, pages inspected and uncertainty. The queue is provisional and must be revisited after the missing pages are indexed. Do not extrapolate an opinion poll from replies or views.
3. Compare only evidence-supported proposals with Oteryn's accepted Reference cut and Evolved design boundaries. In particular, do not equate presentation-only UI work with changes to action timing, PvP, XP, access or World reward supply.

The public forum imposed a visible rate limit during this attempt. The remaining 824 pages and unread candidate posts are outstanding. This checkpoint must not be described as a complete forum audit or as an accepted implementation backlog.
