# Tibia manual notes: controls_communication

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=controls_communication>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `27283933d754ce23886ecf30aa445d2b5d01e28d5c307c89c8ae27b651c503b9`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Client / UI (with the server rules behind it)
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §controls_communication <heading>, capture 2026-09-28`.

## 4.2 Player Communication
- [server] Chat content is subject to Tibia Rules (bans on insulting, racist, sexual, drug-related, harassing, otherwise offensive content).
- [both] Channels are purpose-specific; players are expected to use the matching channel for their message type.

## 4.2.1 How to Write Messages
- [client] Composed in the console entry line, sent on Return. Shift+Left/Right move cursor; Shift+Up/Down recall previously sent lines; Home/End jump to line start/end.
- [client] Hotkeys: Options → Custom Hotkeys → "New Action" (spell/object/text), bound via a pen icon; text hotkeys can auto-send instead of requiring Enter. Action Bars give an equivalent quick-repeat mechanism.
- [server] Spam control (local chat, all public channels, private messages): sustained rate limit of 1 line/2.5s; a burst buffer of 4 lines sendable instantly, decrementing 1/line and refilling 1 per 2.5s up to 4. Exceeding it after the buffer empties triggers a temporary mute (text not broadcast, client shows a notice); muted characters cannot send PMs or cast spells. Mute duration = 5s × (times limit exceeded recently)², i.e. quadratic escalation; offense count is server-tracked, survives relog; buffer refills to full once the mute ends.
- [server] Separate limit: PMs to >20 distinct characters within 10 minutes triggers the same mute.
- [server] Purposeful spam-control evasion (e.g. formatting tricks) remains bannable even if it avoids the rate limiter.

## 4.2.2 Channels
- [client] Tabs atop console; switch via click or Tab; overflow collapses behind an arrow (turns red on an unseen message in a hidden channel). Open: file-card icon or Ctrl+O (typing a name there opens a PM channel directly). Close: "X" or Ctrl+E.
- [server] Free accounts are blocked from Yell, World Chat, English Chat, and Advertising until level 20.
- [client] CipSoft staff messages render in red font.
- Local Chat: [client] renders as a speech bubble above the character, audible to nearby visible characters. [both] Loudness "say"(default)/"yell"(long range)/"whisper"(very short range), via sound icon or prefixes `#s`/`#y`/`#w`; resets to "say" after each message. [server] yell has a 30s cooldown (max 2/min). [client] Alt+W (or bound hotkey) speeds up clearing old game-window text.
- Server Log: [client] receive-only (typed text redirects to local chat); can't be closed, can be reordered. [server] reports damage dealt/taken/by-summons (unless disabled), monster/party loot (unless a loot channel is open), raid notices, tutorial hints, look-at results, last-login time, server-save/update warnings.
- Loot Channel: [client] receive-only (same redirect behavior); shows player/party monster loot, optional value-tier coloring (set via Cyclopedia → Items). [server] folds into Server Log if not open.
- NPC Channel/Chat Window: [client] triggered by "hi"/"hello" to a nearby NPC in local or NPC channel; further conversation continues in the chat window. Some windows offer canned-phrase buttons (e.g. "deposit all", "trade", "sail"); otherwise keyword-driven (e.g. "name", "job", "help", "mission", "outfit", "addon"). [server] swearing at an NPC can provoke a punitive reaction.
- World Chat: [server] public, all players, any language, general topics under Tibia Rules.
- English Chat: [server] public, English-only; other languages go to World Chat.
- Advertising: [server] requires a vocation; Tibia-related ads only; rate limit 1 msg/2min; unavailable while muted.
- Advertising-Rookgaard: [server] separate ad channel for Rookgaard characters (can't trade cross-continent).
- Help Channel: [server] public, for controls/accounts/website/gameplay questions.
- Private Chat Channels: [server] Premium-only to create; auto-closes when the creator closes it or logs out. [client] invite via context menu ("Invite to private chat") or right-click the tab (also offers "Exclude player"); shortcuts `#i Name` invite, `#x Name` exclude; invitees must self-select the channel to join.
- Party Channels: [server] auto-joined on entering a party, but must still be selected to view; access lost immediately on leaving/disbanding, must rejoin if reinvited; current members only.
- Guild Channels: [server] private to members; leader/vice-leaders can invite non-members and can author a persistent login-shown guild message. [client] leader/vice-leader names render in a distinct color. [server] during a guild war, kill events (scorer + running total) and the war's end/outcome broadcast to both guilds' channels.

## 4.2.3 Messaging Other Players
- [server] Requires sender to be at least level 3 to open a private-message channel.
- [client] Opened via context menu "Message to [Name]" on a character, VIP entry, or channel participant. Tab shows the recipient's name; text renders blue. Visible only to the two participants.
- [client] Displays in both console and game window by default; console-only via unchecking "Show Private Messages in Game Window" (Options → Messages).
- [client] One-off send without a channel: `*Name* Text`.

## 4.2.4 Ignore List
- [client] Opened via the "No Entry" console icon or Ctrl+I (rebindable). Add via "Add" + name, or right-click → "Ignore [Name]".
- [server] While "Activate Ignore List" is checked, listed characters' messages are suppressed client-side; ignored players are never notified.
- [server] Entries auto-clear on next login unless marked permanent (checkbox; permanent entries render red).
- [client] "No Entry" icon flashes briefly when a message is actually filtered.
- [server] Extra toggles: "Ignore Yelling" (blocks all yells), "Ignore Private Messages" (blocks all PMs) — independent; whole feature can be disabled via "Activate Ignore List" without clearing the list.

## 4.2.5 Whitelist
- [client] Opposite panel to the ignore dialog; inverse-filter — only listed characters' messages get through, silencing everyone else.
- [client] Add via "Add" + name; entries clear on client close unless "Allow Permanently" is checked (name renders green).
- [server] "Allow VIPs to message you" bulk-whitelists the VIP list but is NOT permanent (resets on client close regardless). Feature independently toggled via "Activate Whitelist".

## 4.2.6 VIP List
- [client] Context-menu options per entry: Edit (one category icon at a time; free-text description; "Notify on login"; assign to custom groups); Remove; Message to [Name] (unavailable offline); Add New VIP (also via right-click "Add to VIP list" on any character); Add New Group (3 predefined — Enemies, Friends, Trading Partners — plus up to 5 custom); Sort (By Name/By Status—online first/By Type); Show/Hide Offline VIPs; Show/Hide Groups (empty groups hidden, unassigned under "No Group"); Report Name; Copy Name (paste via right-click or Ctrl+V).
- [server] Capacity: 20 (free), 100 (Premium).
- [client] Violet name = character is training with an exercise weapon (likely inactive).

## Open questions for Oteryn
- Exact parsing rules for NPC keyword dialogue (fuzzy match, case sensitivity) are unspecified.
- No numeric detail on the NPC-swearing punishment.
- No specification of max message length.
- No stated cap on simultaneous Premium private-chat channels or per-channel invite-list size.
- No stated persistence of PM channels/history across logout.
- Guild-war "scored a kill" broadcast: no definition of what counts as a war kill vs. unrelated PvP between the guilds.
- No specification of case sensitivity for `#i`/`#x`/`#s`/`#y`/`#w`/`*Name*` shortcut parsing.
