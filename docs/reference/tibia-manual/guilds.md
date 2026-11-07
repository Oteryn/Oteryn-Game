# Tibia manual notes: guilds

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=guilds>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `180b07c975f68f3bbee3d9dff9a3bca60a83597f9d01c8a559ce0f9758df5688`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Server mechanics
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §guilds <heading>, capture 2026-09-28`.

## §guilds 5.8.1 What Are Guilds?

- A character can belong to **at most one guild at a time**. [server]
- Membership benefits/features:
  - **Guild Rank**: per-member title shown on the character's account page and the guild page; visible to other players looking at the character. [server][platform]
  - **Guild Chat Channel**: auto-joined on login for all members; leaders/vices can invite non-members into the channel or remove them via the channel's context menu; leader/vice names render in a distinct chat font color. [client][server]
  - **Guild Message**: MOTD-style text shown to all members in the guild channel at login; editable only by leader/vice via "Edit the Guild Message" context action. [server][client]
  - **Guild Board**: private forum-style board for members to read/post; leader/vices can close/delete threads and pin ("sticky") their own threads. [platform]
    - Auto-close rule: a thread with no post in the **last 30 days** is closed. [server][platform]
    - A guild board with no active threads is deleted shortly after; deleted-thread content is **not recoverable**; leader can create a fresh board afterward. [server][platform]
  - **Guild Event**: leader/vices can announce events, guild-only or public. [platform]
  - **Guild Bank Account**: separate from personal bank accounts; funds it (e.g., for guildhall purchase or guild fees); any member can deposit; only leader/vices can withdraw; current balance visible to all logged-in members on the website. [server][platform]
  - **Guild Activity Log**: rolling **30-day** history visible to all members — join/apply dates, promotions/demotions, guild-bank deposits/withdrawals. [server][platform]
  - **Access to Guildhalls**: only the guild leader can rent one; leader invites members/others; leader can restrict interior areas via per-door rights (same door-rights mechanic as regular houses). Gives even free-account members effectively a home via invitation. [server]
    - Every guildhall includes a depot locker; any invitee can access their own depot chest, inbox, and the Market from it. [server]
    - Guildhall bid funding: system first checks the **guild bank account**; if insufficient, it falls back to drawing from the **guild leader's personal bank account**. [server]

## §guilds 5.8.2 Guild Structure

- Baseline hierarchy: exactly 3 conceptual tiers — leader, vice leaders, regular members — though a guild can define up to **20 named ranks** with custom titles layered on top. [server]
- **Leader**: exactly one per guild at all times; the founding character is auto-assigned this rank. [server]
- **Vice leaders**: a guild must maintain **at least 4**.
  - New guild: must recruit ≥4 vice leaders within **3 days** of founding, or the guild is auto-disbanded. [server]
  - If an existing vice leader departs and the count drops below 4, the leader has **2 weeks** to find a replacement. [server]
- **Premium requirement**: only Premium accounts can hold leader or vice-leader rank. [platform][server]
  - Only one leader-or-vice-leader position allowed **per account** at a time (can't be leader on one world and vice on another simultaneously). [server]
  - If a leader/vice's Premium lapses, they **keep the rank and full permissions** (no auto-demotion for lapsed Premium). [server][platform]
  - However, if the guild has fewer than **5 Premium players** among its leader+vice ranks combined, the guild is disbanded after **2 weeks**. [server][platform]
- **Regular members**: unlimited count; join by leader/vice invite or by leader/vice accepting an application. [server]
- Client display: small banner icons under character nameplates distinguish "member of your guild" vs "member of another guild"; looking at a guild member also surfaces total member count and currently-online count for that guild. [client]

## §guilds 5.8.3 Administrating Guilds

- All guild administration (found/edit/join) happens on the tibia.com website, not in-client (the in-game Cyclopedia guild view appears read/limited per other sections, but edit actions are web-based). [platform]
- Viewing a guild ("View" on its info page) shows motto, logo, member list with level/vocation/online-status — available to anyone, logged in or not. [platform]
- Editing/administrative data (guild bank gold, applications, autorank flags) requires being logged into the account. [platform]
- Changes to guild data take effect **only on next login** of affected members (e.g., an online member excluded from the guild remains a de facto member until they log out). [server]
- **Found Guild**: requires Premium; blocked if any character on the account already holds leader/vice rank elsewhere; requires guild name, world, leader character name, account password confirmation. [platform][server]
  - Guild name subject to standard character-name rules plus a **max 29-letter** length cap; offensive names are a Tibia Rules violation. [server][platform]
  - Guild names are **immutable after creation** — an invalid name effectively dooms the guild to disbandment (no rename path). [server][platform]
- **Join Guild**: button shown only if logged in with an account holding an invited character for that guild; select the character, submit. [platform]
- **Apply to this Guild**: available when the target guild is open for applications; pick a character, write an application text; applicant may withdraw the application later. [platform]
  - Rejected applications carry a **30-day cooldown** before re-applying to the same guild. [server][platform]
- **View Applications**: visible to accounts with no further eligible characters to apply, but that have applied with at least one character within 30 days. [platform]
- **Applications** (leader/vice only toggle): "Deny Applications" closes the guild to new applicants until "Allow Applications" is used; all members can read incoming applications, but only leader/vices can accept/reject. [platform]
  - Applications auto-expire after **30 days**; also vanish if the applicant joins a different guild first. [server][platform]
- **Edit Description** (leader only): free-text description up to **500 characters**, plus a homepage link and guild logo upload; offensive content punishable. [platform]
- **Create/Delete/Manage Board** (leader only to create/delete): standard board rules apply (30-day inactivity close, deletion after no active threads, no thread recovery); board also selectable from the general forum section. [platform]
- **Disband Guild** (leader only): requires password confirmation; takes effect immediately.
  - Any rented guildhall is auto-cleared on the **next server save**; portable stored items move to the (former) leader's inbox; non-portable furniture (chairs, tables) stays behind in the guildhall. [server]
- **Edit Ranks** (leader only):
  - Configure up to **20 ranks** with custom names; identical names on adjacent ranks can visually look like a single merged rank (cosmetic only — there's always exactly one true leader, always the first entry in rank order). [server][client]
  - Optional **"Autoprogress Trigger"** per rank: promotion criteria of days-in-guild, character level, or achievement points; to reach a higher rank a member must satisfy the criteria for **every lower rank too** (cumulative gating). [server]
  - No auto-demotion: a member who stops meeting a rank's criteria is not automatically downgraded; only manual demotion via "Edit Members" removes rank. [server]
  - Auto-promotion **cannot target rank 1 or rank 2** (i.e., cannot auto-promote into leader or the top sub-leader tier programmatically). [server]
  - Auto-promotion eligibility is evaluated **once per day** by the system. [server]
- **Edit Members**:
  - "Set title to": leader-only, assigns a custom individual title to a member. [platform]
  - "Exclude from guild": leader or vice can kick a member out. [server][platform]
  - "Set rank to": promote/demote; **any member can promote/demote members ranked strictly below them**, but cannot promote someone up to their own rank (must be done by someone at least one rank higher than the target's destination). [server]
  - Leader can also toggle a member's inclusion/exclusion from the auto-rank-progression system per member. [server]
- **Guild Events**: leader/vice create with name, description, date/time; a "private" flag restricts visibility to guild members only, otherwise the event is public to all characters on that game world. [server][platform]
- **Resign From Leadership** (leader only): leader nominates a successor; transfer takes effect at the **next server save**; any rented guildhall auto-transfers to the new leader; the incoming leader's account must not already hold another leader/vice position. [server]
- **Invite Player**: leader or vice can send invites; invited character must accept via a button on the guild info page. [server][platform]
- **Exclude Player**: leader or vice, via "Edit Members"; an excluded member who is online at the time stays a member until they log out. [server]
- **Leave Guild**: vice leaders and regular members may leave freely at will; if online at the moment of leaving, remain a member until logout. [server]
- **Choose New Leader**: surfaces to vice leaders only when the guild has no leader (e.g., leader account scheduled for/undergoing deletion).
  - All current vice leaders are candidates; election closes once all vices have voted, or the guild is about to be deleted. [server]
  - Highest-vote vice becomes leader; ties broken by **longest guild membership tenure**. [server]
  - Votes from vices who lost Premium status or are themselves scheduled for deletion are **discarded**. [server]
  - If zero votes are cast, the guild is deleted on the scheduled date; if the original leader's deletion is revoked mid-election, the election is cancelled. [server]

## §guilds 5.8.4 Guild Wars

- Eligibility to participate in any war: guild must have 1 leader + at least 4 vice leaders, must not be mid-formation (still recruiting initial vices) or mid-dissolution, must not already be at-war-with-or-have-declared-on more than **5 guilds** simultaneously, and cannot be at war with the *same* opposing guild more than once concurrently. [server]
- a) Declare War:
  - Only the guild leader can initiate, via "Guild War" on own guild page -> "Declare War" form. [platform]
  - Form fields and bounds:
    - Opponent: selected from a dropdown of eligible guilds on the same world. [platform]
    - Duration: **7-180 days**. [server]
    - Kills to Win: **10-1,000 kills**. [server]
    - Fee for Your Guild (paid to opponent if you lose): **0 - 2,000,000,000 gold**. [server]
    - Fee for Opposing Guild (paid to you if you win): **0 - 2,000,000,000 gold**. [server]
    - Comment: free text message to the other guild. [platform]
  - Fixed **war charge of 1,000 gold** is required IN ADDITION to the declared fee, in each guild's bank account, at the moment the war actually starts (both sides independently need fee+1,000 present). [server]
  - Opposing leader can **Accept**, **Reject**, or **Edit** (counter-propose) the terms; only fields "Duration" through "Comment" are editable in a counter-proposal (Opponent presumably fixed). [platform][server]
  - On rejection: the two guilds are locked out from declaring war on each other again for **7 days**. [server]
  - If the opposing guild takes no action within **7 days**, the declaration auto-deletes. [server]
  - Once both sides accept, war starts at the **next server save**, contingent on both guilds actually holding fee+1,000 gold at that moment; if either is short, war start is **delayed** (implies a recheck loop until funded). [server]
  - The declaring leader can cancel the declaration unilaterally any time before the war actually starts. [server]
- b) At War — nameplate banner color coding: green = own guild, red = opposing guild you're at war with, blue = a guild at war with someone else (uninvolved to you). [client]
  - Attacking an opposing member always triggers a **protection-zone block** for the attacker (can't flee into PZ after attacking) — except the edge case of attacking only own guildmates, which still permits PZ entry. [server]
  - During an active war: leaving the guild or kicking a member is still allowed; **new invites are blocked** once war is declared; however, characters invited *before* the war started may still join after war start. [server]
  - Kill-score cap: killing the **same** opposing member counts toward the war kill score **up to 5 times per 24 hours**; further kills of that same victim within the window don't add score. [server]
  - Live war state (current kill score, start date, max duration) is viewable on the guild wars page. [platform]
  - Assistance restriction: characters not involved in the war cannot heal or party-buff/strengthen a participant who took damage from an opposing-guild member within the **last 60 seconds**. [server]
- c) The End of a Guild War:
  - Early end: **surrender**, available to either guild leader via a button on the guild wars page; surrendering guild automatically loses, forfeiting the set fees to the winner. [server][platform]
  - Normal end conditions: reaching the agreed kill target, or the set duration elapsing.
    - Reaching kill target = automatic win, full fee transfer. [server]
    - Duration expires with neither side at target: guild with **more kills** wins; winner gets their own fee refunded plus a **proportional share** of the loser's fee equal to the winner's *kill-count margin* over the loser as a fraction of the winner's own kills (per worked example: winner's excess-kill fraction of winner's own fee-set amount, computed from the winner's kills, applied against loser's declared fee). [server]
    - Exact tie in kills at duration end: **no winner**, both guilds get their own fee back in full. [server]
  - If the opposing guild is disbanded while a war is ongoing, the remaining guild automatically wins and collects all fees. [server]

## Open questions for Oteryn

- The worked kill-proportion payout example (guild A: 1000 gold/12 kills vs guild B: 600 gold/4 kills -> A gets 1000 back + 400 from B) needs to be reverse-engineered into an exact formula; manual only gives one example, not the general formula — needs derivation/verification before server implementation (appears to be `winner_share_of_loser_fee = loser_fee * (winner_kills - loser_kills) / winner_kills`, unconfirmed).
- Autorank criteria types (days, level, achievement points) — need exact operators (>=, >) and whether multiple criteria on one rank are AND'd or is it single-criterion per rank only.
- Whether "up to 20 ranks" is a hard schema limit or configurable per world/server build.
- Precise semantics of "cannot promote to own rank" — confirm off-by-one: promoting FROM rank N+1 TO rank N requires actor at rank < N (i.e., actor must be at least one rank above the destination), matching the manual's rank-3-promotes-to-rank-4 example.
- Guild disbandment auto-triggers (< 4 vices past grace period, < 5 Premium leader/vice) — need to confirm whether these are checked on a daily job like autorank, or event-driven.
- Interaction between guild war "no new invites" rule and the "already-invited-before-war can still join" rule — need an exact invite-timestamp cutoff definition.
- Whether guildhall depot locker access for invitees persists after they're kicked/uninvited (presumably revoked immediately, unconfirmed).
- Guild-vs-guild concurrent war cap of 5 — confirm it counts pending declarations (not yet started) or only active wars.
