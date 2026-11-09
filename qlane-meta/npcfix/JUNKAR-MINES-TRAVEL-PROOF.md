# Junkar Mines paid-travel execution

The existing production NPC_TALK PostgreSQL scenario now includes Junkar Mines and its three already admitted return routes. The fixture grants Grand Postman rank for the final Junkar cases, so the expected fares are Kazordoon 0, Robsons Isle 60 and Thais 80 gold. Production fares, route admission and guards are unchanged.

The expanded case first failed against real PostgreSQL in 51.53 seconds with `Format("sector floor outside the World")`, after emitting `POSTGRES_STARTED TRAVEL npc_paid_travel`. The fixture world now explicitly includes the three required underground floors (-15, -14, -13). The route key uses its admitted `robsons_isle` keyword.

A second real PostgreSQL run failed in 117.40 seconds at the zero-fare inventory-change assertion: mission six grants carpet discounts, but ships require Grand Postman rank. The fixture now seeds rank 3 for Junkar only, after all original passages. Passage quote assertions are exact, preventing `10 gold` from matching an expected `0 gold` substring. Production discount rules are unchanged.

Every added route retains the existing quote, injected arrival transaction rollback, reconciliation, arrival and durable receipt replay assertions. No existing assertion was removed or loosened. The final real PostgreSQL scenario passed: one test, zero failures or ignored cases, in 121.28 seconds. It emitted `POSTGRES_EXECUTED S10T junkar_mines` markers for Kazordoon fare=0 at (32660,31957,15), robsons_isle fare=60 at (32527,32036,14), and Thais fare=80 at (32370,32180,13), each confirming replay charged once. Formatting and diff checks passed. Final focused check and Clippy are running; this document does not qualify held Harlow/Kamil routes or the whole S10 mission slice.
