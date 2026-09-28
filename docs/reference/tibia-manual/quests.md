# Tibia manual notes: quests

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=quests>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `37528051f114d83892e5abd8631c5b7d332252af0d751754f02a9cd7f39cd0f8`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Server mechanics
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §quests <heading>, capture 2026-09-28`.

## §quests 5.5.1 What Are Quests?

- Any character can attempt a quest provided stated prerequisites (e.g., minimum level) are met. [server]
- Most quests are single-completion per character; the associated reward/treasure is likewise obtainable only once per character. [server]
- Quests are not time-boxed once started: a character can log out mid-quest and resume later with no expiry. [server]

## §quests 5.5.2 How to Get Quests

- Quests are obtained by talking to NPCs — greet, then ask about "quest" or "mission" as a dialogue keyword. [server]
- Some quests are discoverable purely by exploration (no NPC handoff), e.g. finding a hidden area or item. [server]
- A door that refuses entry with a "not worthy" style message signals a level gate — character must return once the required level is reached; no other unlock method implied. [server]
- Locked doors imply a key-based unlock: correct key must be found elsewhere and tried on the door. [server][client]

## §quests 5.5.3 Quest Log

- Client-side UI opened via a dedicated shortcut/button. [client]
- Started quests appear under a "Quest Lines" list; selecting a quest reveals its individual missions; selecting a mission shows task detail text. [client]
- Completed quests show a green-tick marker and a short summary of the final task; an NPC-given hint at quest end remains reviewable via the completed quest's detail view. [client][server]
- UI features: text search field, "Show completed" toggle, "Show hidden" toggle, "Sort by Name" alphabetical sort. [client]
- Per-quest "Show in quest tracker" checkbox adds it to a separate always-visible Quest Tracker window during play. [client]
- Exclusions from the quest log: quests that are just a one-off findable treasure (no log entry at all); outfit/addon quests are only listed while in progress (removed from log once solved). [server][client]

## §quests 5.5.4 A Word on Quest Spoiling

- Purely a design/community guidance note (no enforced mechanic): players are encouraged not to consult guides/veterans and to solve quests unaided. [server]
- No penalty or restriction is described for looking up spoilers — social/etiquette guidance only.

## §quests 5.5.5 World Quests

- World quests require participation from many players on a single game world (not soloable in the small-group sense of regular quests). [server]
- Two subtypes:
  - World events: seasonal, start simultaneously across all game worlds, run on a fixed real-world schedule (roughly annual per the text). [server]
  - World tasks: pre-defined, not date-bound, can be undertaken whenever a world's population fulfils them. [server]
- Unlike regular quests, most world quests are repeatable at intervals — same character can complete a given world quest multiple times across cycles. [server]
- World events require the character to be logged in while the event window is active; missing that window forfeits participation for that run — no makeup/catch-up. [server]
- Current/upcoming world quest status and past world-event winners are published externally (Tibia.com library "world quests" section) — a platform/website concern, not in-client. [platform]

## Open questions for Oteryn

- Exact quest-log data model: how "hidden" quests are flagged/unflagged, and what determines a quest's hidden state by default.
- Reward/treasure single-instance enforcement: is this per-character flag storage keyed by quest+chest, or per-quest completion flag only (affects whether a partially-looted quest chest can be re-emptied).
- Level-gate door messaging and the underlying check (character level vs. some other stat) needs exact server-side condition semantics.
- World task "not bound to a specific date" — need a concrete trigger/availability model (world state condition vs. cooldown) since this differs from world events.
- World event catch-up: confirm there is truly zero catch-up mechanism (no grace period) for players offline during the event window.
- Whether quest mission structure (quest -> missions -> tasks) maps to distinct save-state entries per mission or one blob per quest.
- Outfit/addon quest log visibility toggle (removed once solved) — need to confirm whether server still retains completion state for outfit-unlock purposes after log removal.
