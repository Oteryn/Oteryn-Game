# Pemaret — Marlin Trophy source packet

The shared npc-research/pemaret.md was absent; this task permits writes only in the worktree, so the packet stays here.

Source: already downloaded `tools/content-schema/quest-authoring/samples/donor-source/corpus.tar.gz` (SHA256f3fe990f2a72e2779fb360b9c8f98be9d4174b898fa486d4251aa00137c56277), manifest9823fd5e25796d5cfcd88a586530a60c0d7a9ca8a4f1130df25afbc2fb21b693. Crystal9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d `data-global/npc/pemaret.lua`, blobSHA2567860edb917cf36e43f2f675b9a2f9b8bbbc0821051ef20e4a9443cf04649489c. Cached NPC index links this source to canonical `oteryn:quest.marlin_trophy_quest`; no new external fetch.

Lines61–80: `marlin`, if MarlinTrophy storage<1 and carried marlin901>0, offers decoration and sets topic1. `yes` at topic1 and storage<1 removes exactly one901, gives one trophy902, sets MarlinTrophy1, then topic0. Missing fish says so and clears topic. `no` at topic1 declines and clears it. Completion is once per character. No gold or XP belongs to this exchange. The source does not require catching the fish oneself: a carried/traded marlin is accepted.

Other existing services retained: Edron travel20gold (postman discount), Eremo travel0, generic ship kick; fresh fish3578 BUY5, green perch7159 SELL100, marlin901 SELL800. Source greeting/farewell at105–107. Placement not inferred from those scripts; use existing normalized project placement data. The relevant901/902 native carrying profiles are currently absent. Full real-dispatch exchange qualification remains unfinished; do not promote this source packet.
