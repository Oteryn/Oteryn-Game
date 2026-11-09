# Maris Mistrock Beregar clue execution

The existing source-qualified Beregar quest rule already owns Maris's `mistrock` callback. The new isolated PostgreSQL test uses the native activated-content fixture, real NPC_TALK owner, canonical quest catalogue and durable NPC-program reconciliation; no production quest data, handler or guard changed.

It verifies an unrelated `mission` keyword leaves WayToBeregar at zero, `mistrock` returns both exact source lines and sets the track to one, replay returns the recorded response with exactly one change, and another `mistrock` repeats the clue without adding a second quest receipt. Source: Crystal 201189f871d23597b82caf88fd611454a2ddefb8, data-global/npc/maris_mistrock.lua:67, as captured by the existing quest authoring tool and packet.

The isolated real PostgreSQL case passed: one test, zero failures or ignored cases, 121.34 seconds. Marker: `POSTGRES_CASE_COMPLETED NPCFIX maris_beregar_clue_once success=true duration_seconds=120.922055829`. This verifies existing behavior; no artificial failing baseline or new production fix is claimed. Focused cargo check, strict Clippy and formatting verification are running. The whole S10 slice and held travel routes remain unqualified by this clue-only case.
