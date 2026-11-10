# SRVGAP1108 — checked quest mechanic gaps

Checked against QMECH `aca848557` before the lane's watch change. This is a
bounded report of the quest steps inspected, not an audit of every quest.

| quest | missing mechanic (server or client) | evidence (quest file and step, plus missing hook/table/command) | blocks the quest fully or partly |
| --- | --- | --- | --- |
| Adventures of Galthen Quest | Server: qualify and execute the source boss exit-teleporter hook for Ahau | `content/quests/definitions/quests-00100-00199.json`, definition `oteryn:quest.adventures_of_galthen_quest`, chosen recipe step `s5` requires defeating Ahau. `content/encounters/bosses/crystal-lever-candidates.json`, record `data-global/scripts/quests/adventures_of_galthen/actions_ahau.lua`, has `complete_literal_layout=true`, `callbacks_known=false` and blocker `config.exitTeleporter`. `crates/gameplay/content/src/content/bosses/activity_template.rs::qualify_activity_template` returns `SourceHookUnavailable` for these facts; no fallback template is admitted. | Partly: this encounter step cannot use the boss activity path. Other quest steps also retain their own binding requirements. |
| Adventures of Galthen Quest | Server: prove the native callback closure before admitting Mitmah Vanguard's boss activity | Same quest definition, chosen recipe step `s8` requires defeating Mitmah Vanguard. The captured record `data-global/scripts/quests/adventures_of_galthen/actions_mitmah_vanguard.lua` has a complete literal layout and no listed blockers, but `callbacks_known=false`. `qualify_activity_template` rejects that flag as `SourceHookUnavailable`, even when the layout is complete. | Partly: this encounter step cannot use the boss activity path; this report does not claim the rest of the quest is wired. |

The QMECH package currently assigns boss/encounter work to `engenc` and
`engenc2`. These findings are offered to those lanes; this report is not a
claim on their work. The watch, seashell, clay and jewellery mechanics in the
SRVGAP1108 packet were not established by this inspection as quest blockers.
No client gap is claimed.
