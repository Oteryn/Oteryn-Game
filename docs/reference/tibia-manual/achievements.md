# Tibia manual notes: achievements

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=achievements>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `36587f31da0e76324fa59bb8cc0b9144b97ae5694e7cf05c37c06c3e8f99dbb2`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Server mechanics
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §achievements <heading>, capture 2026-09-28`.

## §achievements 5.6.1 What Are Achievements?

- Achievements are optional goals; unlike quests they grant no XP or items — only reputation value (points, see below) and bragging rights. [server]
- Two visibility classes:
  - Regular achievements: listed publicly on the website Library and in-client Cyclopedia's Character section. [server][client][platform]
  - Secret achievements: not documented anywhere officially; players must discover conditions themselves (in-game hints, community/forum discussion). [server]
- On completion, the client shows a pop-up notification naming the achievement. [client]

## §achievements 5.6.2 Achievement Grades and Points

- Achievements are bucketed into 4 difficulty grades, each with a point range:
  - Grade 1: 1-3 points [server]
  - Grade 2: 4-6 points [server]
  - Grade 3: 7-9 points [server]
  - Grade 4: 10 points (fixed, not a range) [server]
- Per-world leaderboard: Highscores list the top 300 characters by total achievement points on each game world. [server][platform]
- A character's own total achievement points is visible on its Characters (profile) page. [platform]

## §achievements 5.6.3 How to Display Achievements?

- Each character may showcase up to 5 earned achievements on its public character page. [platform]
- Selection is done account-side: open the Characters section, click "Edit" for the target character, and tick checkboxes next to eligible achievements at the bottom of the edit page. [platform]
- The edit page lists all regular achievements (earned ones active, unearned ones greyed out) plus any secret achievements already discovered by any character on the account. [platform]
- Cross-character sharing: a character can display achievements that were actually earned by a *different* character on the same account, not just its own. [platform]
- The 5 displayed achievements can be set independently per character (not account-wide). [platform]

## Open questions for Oteryn

- Full canonical achievement list (names, grade, points, unlock condition) is not in this manual section — needs a separate content/data source (Cyclopedia export or dedicated data-mining task).
- Secret achievement discovery mechanics (how server flags "discovered" state, and whether discovery requires the responsible character or is account-wide immediately) is unspecified.
- Whether achievement points affect anything beyond Highscores ranking and guild auto-promotion criteria (guild manual section references achievement-point-based rank triggers) needs cross-check with guilds notes.
- Point award granularity within a grade's range (e.g., what determines 1 vs 3 points within Grade 1) is not explained — may just be a fixed per-achievement value chosen at content-design time, not a formula.
- Whether "up to 5 displayed achievements" has any ordering/priority control or is a simple set.
