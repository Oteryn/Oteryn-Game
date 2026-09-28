# Tibia manual notes: starting

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=starting>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `cd3c109062f9f5c925b59f684ce5f5c05aaee63f58bf6d83ad0e2a1da474558c`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Client / UI (with the server rules behind it)
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §starting <heading>, capture 2026-09-28`.

## 2 Setting Up the Game
- [both] Top-level intro only; no additional mechanics beyond child sections.

## 2.1 Entering and Selecting Account Data
- [server] Account creation requires a unique email address; one account per email, one email per account at a time.
- [client] Account can be created via website or client; steps are equivalent, some fields website-only (marked in child headings).
- [server] Email is the account recovery key: losing access to an unregistered account's email means permanent loss of account.
- [both] Password entry requires confirmation (typed twice).
- [client] Client-based creation requires reading/accepting terms as part of the same flow.

## 2.2 Creating a Character
- [server] Character requires: name, gender, game world.
- [server] Max 25 active characters per account, distributable across one or multiple game worlds.

### 2.2.1 Choosing a Name
- [server] Name constraints: max 29 characters; no digits, no special characters; certain words/letter-combinations disallowed; must comply with Tibia Rules.
- [server] Renaming an existing character later requires a fee.
- [client] Client offers a "Suggest Name" auto-generator, repeatable until player accepts a suggestion.

## 2.2.2 Choosing Your Character's Sex
- [server] Gender determines outfit and can affect some situational content; otherwise both sexes are mechanically equal.
- [server] Sex change is possible later for a fee (see character section).

## 2.2.3 Selecting the Game World
- [server] World Category: "Regular" (fully tested, default/recommended) vs "Experimental" (used for CipSoft testing).
  - [server] Experimental worlds: character creation requires an existing character on the account already; worlds are transfer-locked (can only move between other locked worlds); may be reset to a former state at any time; not BattlEye-protected.
- [server] World Location: server selected by physical region (NA/EU/Asia); advisory only, any world playable from anywhere.
- [server] World Type options and their PvP rules:
  - Optional PvP: attacks only with mutual consent.
  - Open PvP: free attacks allowed; excessive killing triggers skull marks and sanctions. Special case: world "Premia" is Open PvP but Premium-account-only.
  - Retro Open PvP: same attack rules as Open PvP (retro ruleset variant).
  - Hardcore PvP: PvP encouraged; characters can only walk through each other inside protection zones or on special fields.
  - Hardcore Retro PvP: attacking/killing tolerated and encouraged (retro ruleset variant).
- [client] "Play Tutorial" checkbox (website only): unchecked skips tutorial; characters without it start at level 2 in Newhaven and pick a vocation immediately, if account already has a character on the main continent.

## 2.3 Reading and Accepting Terms
- [server] Must accept all 3 documents (Service Agreement, Tibia Rules, Privacy Policy) to be permitted to play; all 3 checkboxes required before "Submit" is enabled/accepted.
- [server] Account created only after password, email, world, character name/sex, and all 3 term acceptances are submitted.
- [server] Account must subsequently be confirmed via emailed link; unconfirmed accounts can request the email again from the account page.

## 2.4 Getting Started
- [client] Playable via Windows/Mac/Linux client; local storage footprint is small since gameplay logic lives on the game server; client keeps a persistent connection to the game server.

### 2.4.1 Download and Installation
- [client] Windows: download via Tibia Launcher installer; initial install ≈350 MB, can grow up to +100 MB in user data/settings once the world map is explored; installer download itself ≈40 MB.
- [client] Mac: distributed as `Tibia.app.tar.gz`; must be unpacked into a location with read/write permission (e.g. local Applications folder) for the launcher to self-update.
- [client] Linux: distributed as tarball (`tar xfz tibia1000.tgz`); must be run from its own extracted folder (binary `Tibia`) — will not locate required files if launched elsewhere; initial folder ≈325 MB, plus up to +100 MB in `~/.tibia` after full map exploration.
- [server] Login queue: if too many players are online for the selected world, client shows queue position and auto-retries login after the wait period; server rejects any login attempt submitted before that wait period elapses. [client] shown as a position indicator.

### 2.4.2 Starting the Game
- [client] Launcher checks client version at start; outdated client shows an update screen (button blue = full download needed, orange = update available, turns green once done) before reaching the title screen.
- [server] Login requires email + password; if the account has an authenticator enabled, a valid token is also required.
- [client] Successful login shows a character list (name + game world per character); player selects one and confirms; same login-queue behavior as above applies per selected world.
- [client] Character list shows a "Get Premium" link if the account is free or Premium Time expires within 14 days; leads to account management purchase flow (login required there too if not already authenticated).
- [client] "Remember email" checkbox persists the email for next login — flagged as unsafe on shared computers.
- [client] "Forgot password" leads to the Lost Account Interface; a separate recovery-key-based flow exists for recovering a lost email address.
- [client] Title screen also exposes: Create new account, Manage Account (opens web account management), Manage Clients (download/select client version), Options (gear icon; "Show Advanced Options" reveals full settings, first-time view shows only basic settings), Quit.
- [client] Title screen extras: online player count and streaming links, Random Hint box, Event Schedule (opens a 12-month events calendar), Boosted creature of the day.
- [server] Boosted creature: one per day, gives increased experience, increased loot quantity, and faster respawn for that creature.

## Open questions for Oteryn
- Exact disallowed-word/character list for character names is not specified (only "certain words or combinations of letters").
- Rename and sex-change fee amounts are not given here (deferred to other sections not in scope).
- Exact login-queue wait-time formula/algorithm is unspecified.
- Precise mechanics of "reset to a former state" for experimental worlds (scope, frequency, notice) are unspecified.
- No numeric definition of "excessive killing" threshold for skull marks on Open/Hardcore worlds (deferred to combat/PvP sections).
- Authenticator token format/length and retry/lockout behavior on failed login are unspecified.
- Whether account can have characters simultaneously on both experimental and regular worlds is not explicitly stated (only says an existing character is required to create on experimental).
- No specification of what data exactly is stored client-side (session vs persistent state) beyond "small" footprint claim.
