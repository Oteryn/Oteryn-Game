# NX-8 S13c: party member effects

The party query now publishes the actual condition-owner remaining durations of
local party members. It does not grant or change a spell, range, cost or cast
rule. Capability 383 negotiates optional member field 5 on existing query 46;
nonselecting peers retain the original encoding. No command or migration added.

| Rule | Evidence | Implementation |
| --- | --- | --- |
| Protect Party is a buff condition, shield +3, 120 seconds | Canary `47dfd51f45280a59a1d3e50ba7edd573d7234446`, `data/scripts/spells/party/protect_party.lua:6-10`; Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, same path/lines; fetched 2026-11-05, OTS_HYPOTHESIS_ONLY | Existing cast/condition owner is unchanged; `party_command.rs::member_party_spell_durations` reads its active instance and deadline. |
| Caster and party members within range receive the condition | Both pinned donors, same path `:26-38,64-65` | Existing production cast applies it; projection matches current durable membership IDs to current exact actor/session leases. The PostgreSQL case drives real casts and party query and observes two affected members and one distant unaffected member. |
| Party effect names/order are Train, Protect, Enchant, Heal, Enlighten | Existing accepted authored condition keys and self projection `party_effects.rs`; donor party spell scripts above for Protect | One bounded five-element member duration vector uses those existing production keys, with zero for expired/removed conditions. |
| Removed conditions must disappear; timers must expire | Condition-owner active-at and remaining-duration reads, not inferred membership/history | Query replaces the whole member view. Native party window refreshes every second, and elapsed monotonic time counts down between responses. |

The World party owner supplies membership. The Channel condition owner supplies
only current locally owned actor conditions. A missing duration field means an
unobserved remote/offline member (or extension not selected), displayed as
“effects unavailable”; an observed five-zero vector means no active party
spells. No remote condition state is fabricated. The server does not persist or
replay a stale effect snapshot across reconnects. The native party panel uses a
scrollable display so all members remain accessible.

Validation filters: `party_member_effect_durations_decode_and_replace_on_refresh`
(protocol red before production changes, captured by lead),
`party_member_effects_real_query_durations_and_legacy_gate_postgres` (real
PostgreSQL/production casts/query; existing harness prints POSTGRES_EXECUTED with
duration), `member_effects_replace_and_count_down` (UI), and
`party_member_effects_server_result_reaches_native_panel_and_removes` (native
mailbox/query/result play path). Tests also cover legacy omission and a stale
acting lease fence. No wiki fetch was attempted: **wiki-pending** for official
party tracker presentation and the five current spell effect descriptions.
