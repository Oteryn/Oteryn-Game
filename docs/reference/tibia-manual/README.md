# Tibia manual reference notes

These are Oteryn-written notes on the official Tibia manual
(<https://www.tibia.com/gameguides/?subtopic=manual>), all 19 sections. Agents who reconstruct game
mechanics or the client read these files **instead of fetching tibia.com**. tibia.com blocks the cloud
containers, GitHub runners and web fetch tools with Cloudflare, so an attempt to fetch it wastes tokens.

- Capture: the owner, 2026-09-28, on a local machine.
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL` (see
  `docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`).
- Full text: private, in Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`, with a per-section
  SHA-256 manifest. This repository is public and the manual is copyright CipSoft GmbH, so the text itself
  is not committed here. The notes restate the rules in Oteryn's own words.

## Which file to read

| Consumer | Files |
|---|---|
| Client / UI | [interface](interface.md), [controls](controls.md), [controls_communication](controls_communication.md), [controls_trading](controls_trading.md), [starting](starting.md) |
| Server mechanics | [characters](characters.md), [combat](combat.md), [magic](magic.md), [world](world.md), [quests](quests.md), [achievements](achievements.md), [houses](houses.md), [guilds](guilds.md) |
| Content catalogue (Store) | [store](store.md), [products](products.md) |
| Platform / web (not Game runtime) | [accounts](accounts.md), [support](support.md), [forum](forum.md), [introduction](introduction.md) |

## How to use the notes

- **Headings.** Each file follows the manual's own numbered headings, so a rule is cited as
  `tibia.com manual §controls 4.1.6, capture 2026-09-28`.
- **Tags.** Bullets are tagged `[client]`, `[server]`, `[both]` or `[platform]`.
- **Open questions.** Each file ends with `Open questions for Oteryn`: behaviour the manual leaves open.
- **Behaviour, not numbers.** The manual describes behaviour and states only some numbers. Exact values and
  formulas come from the wikis at the target date (`docs/agents/programs/OTERYN_TARGET_DATE_20260927_DECISION.md`),
  and Canary/Crystal stay `OTS_HYPOTHESIS_ONLY`.
- **Unconfirmed values.** A note marked unconfirmed, such as a formula reverse-engineered from a worked example,
  is not a stated rule. Verify it before implementation.
- **Refresh.** A new capture adds a new dated attachment to `KAN-33` and updates these notes in one change.
  Never fetch tibia.com ad hoc from an agent session.
