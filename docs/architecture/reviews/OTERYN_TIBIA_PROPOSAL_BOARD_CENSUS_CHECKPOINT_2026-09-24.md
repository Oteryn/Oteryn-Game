# Tibia Proposal Board: partial whole-board census and follow-up reading

- Research date: 2026-09-24.
- Source: public [Proposal Board](https://www.tibia.com/forum/?action=board&boardid=10), **All Threads** filter.
- Relation: continuation of the [24-thread qualitative review](OTERYN_TIBIA_PROPOSAL_BOARD_RESEARCH_2026-09-24.md), the [PR #571 product audit](OTERYN_EVOLVED_PR571_PRODUCT_AUDIT_2026-09-11.md) and its [192-entry register](OTERYN_EVOLVED_PR571_DECISION_REGISTER_2026-09-11.json).
- Status: **PARTIAL RESEARCH CHECKPOINT**. `IMPLEMENTATION_AUTHORITY: NONE`; `PRODUCTION_AUTHORITY: NONE`. This document does not approve changes to PR #571 or to the accepted Reference target.

## What was actually covered

The board initially displayed **67,444 topics on 2,249 pages**. During the attempt it displayed 67,445, then 67,449 and 67,450 topics. These are observations of a live list, not one frozen snapshot. Each ordinary page had 30 topic rows; the last page, read when the count was 67,445, had five.

| Measure | Verified result |
|---|---:|
| Board pages recorded | **1,631 / 2,249 (72.5%)** |
| Topic rows recorded | **48,905** |
| Distinct thread IDs in recorded rows | **48,888** |
| IDs repeated across page boundaries | **17** |
| Recorded page ranges | **1–817; 1131–1198; 1504–2249** |
| Unrecorded page ranges | **818–1130; 1199–1503** (618 pages) |

Every recorded page has the expected row count and unique IDs *within that page*. The 17 repeated IDs occur across pages, including five newly observed across pages 807–808 when access resumed. Changes in a live pagination order prevent a claim that all initially visible threads were captured. The index records titles, authors and displayed activity/update metadata; it does **not** contain the opening posts or replies for 48,888 threads. Title matches are a queue for reading, not evidence of the content or popularity of an idea.

Six thematic readers nominated threads from the saved index while page enumeration continued. Ten additional, distinct thread IDs were opened and read at least in part; none duplicates an ID in the earlier 24-thread report. Some candidate pages shifted while the board was live. Earlier requests stopped when the site displayed **429 Too Many Requests**; ordinary access later returned and indexing resumed. The other shortlisted threads remain unread.

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
| [Find Stash Items Usable for Weekly Tasks](https://www.tibia.com/forum/?action=thread&threadid=4999703), 1/1 | The author asks for a Stash search option for items usable in weekly tasks. There were no replies. | Direct functional match to `B09` as a discovery filter; show item availability separately from reservations. The post says nothing about changing tasks, rewards or ownership. |
| [Adjustments to the Stash and Market systems and filters](https://www.tibia.com/forum/?action=thread&threadid=4997734), 1/1, 15 posts | The author groups Stash↔Market navigation with preserved filters, NPC versus Market value cues, and personal item categories. A later author reply asks to show lowest sell and highest buy offers alongside an average. Several replies affirm the navigation issue; no substantive feature-level rebuttal was visible. | `D01` state preservation is a UI concern; live price cues need freshness and fee context, and a “do not sell” category needs one consistent reservation rule across sale paths. `B09/B10`, `A13/D05` and `D12/D13` are thematic overlaps only, not proven source attribution or acceptance. |

The economy reader reached a 429 page on the first attempt, then read two nominated posts after ordinary access returned. The progression reader did not open any nominated post; its queue remains title-only. Several candidates for `B/C/D` are only superficially similar, as the two false matches above demonstrate. `A01–A18` remain the only batch traced to one exact source thread in the earlier review.

## Continuation and decision boundary

1. If this research is resumed, continue the two missing ranges from their first page. Check row counts, page numbers and cross-page duplicates, then compare the live board count; a page census alone cannot certify a fixed-time topic population on this changing board.
2. An offline queue nominates 78 distinct, still-unread thread IDs across eight **social and community** clusters from the **partial** title index; other thematic notes have separate queues. Read opening posts and dissenting/qualifying replies before extending product findings. Record the exact thread ID, pages inspected and uncertainty. The queues are provisional and must be revisited after the missing pages are indexed. Do not extrapolate an opinion poll from replies or views.
3. Compare only evidence-supported proposals with Oteryn's accepted Reference cut and Evolved design boundaries. In particular, do not equate presentation-only UI work with changes to action timing, PvP, XP, access or World reward supply.

The public forum imposed a visible rate limit during an earlier part of this attempt, and access later returned. Work stopped at the user's requested checkpoint after page 817. The remaining 618 pages and unread candidate posts are outstanding. This checkpoint must not be described as a complete forum audit or as an accepted implementation backlog.
