# Consolidated reference audit coverage

Reconciled 2026-10-09 from existing written evidence. This is the continuation
index: consult it before reopening a reference screen. It preserves prior work;
it does not claim that historical screenshots were all re-inspected in this pass.

## Sources and meaning

- [A: prior audit](REFERENCE-AUDIT.md): settings, sidebar, mainland-character
  follow-up, analysers, containers, character pages and static discoveries.
- [B: corrections](UI-REFERENCE-OBSERVATIONS.md): HUD states and hotkeys.
- [C: settings detail](UI-REFERENCE-SETTINGS.md): later editor, dropdown and
  action-bar observations; also the 28-entry shortcut inventory.
- [D: continuation](UI-REFERENCE-MODULES.md): selected menu re-inspection and
  fresh interaction probes. D does not replace A's broader mainland coverage.
- [E: pinned OTClient source map](UI-REFERENCE-OTCLIENT.md): layouts, handlers
  and explicit implementation gaps at upstream revision `53c3878a`. Read this
  before a new live probe; source evidence is not official-client verification.

Observed means an existing record establishes visible controls or a stated
interaction. Restricted means only the restricted state is established for that
account. Neither means successful mutation, persistence, or Oteryn implementation.
The next-check column is a gap list, not a request to repeat the observed column.

## The 28 shortcut entries

| # | Entry | Preserve existing observation | Source | Targeted remaining check |
| --- | --- | --- | --- | --- |
| 1 | Skills | Statistics, visibility/context menu | A | Any unidentified menu entries; reset effects remain untested |
| 2 | Battle List | Filter strip, name and eight sort choices | A | Unidentified filter tooltips, secondary-list restrictions |
| 3 | Spell List | Search, details, vocation/group/type/account filters | A, D | Combined filters, aggregate toggles, Show More, saved state |
| 4 | VIP List | Add/sort/offline/groups menu, empty list | A, D | Dialog validation and populated groups/status updates |
| 5 | Lenshelp / help | Shortcut listed; generic help entry recorded | A, C | Resolve exact destination/name; do not infer from Settings Help |
| 6 | Quest Log | Quest/mission details, search, sorting, tracker controls | A | Content-dependent states and tracking effects |
| 7 | Compendium | Guide, features, information, updates, support | A | Missing functional routes only; do not reread all historical articles |
| 8 | Cyclopedia | Map, Items, Houses, Character, Bestiary, Charms, bosses, archive | A, D | Subpage gaps listed below |
| 9 | Highscores | World/vocation/category/PvP filters, ranks | A | Unrecorded dropdown choices and empty/error states |
| 10 | Player Guide | Visible Basics/Combat and Hunting; Compendium guide inspected | A, D | Dedicated panel navigation/unlock behavior |
| 11 | Manage Shortcuts | Displayed/available lists, full 28 entries | A, C | Order/visibility persistence if not evidenced elsewhere |
| 12 | Party List | Empty panel; name and eight sort choices | A, D | Edit Name meaning, populated rows and sort behavior |
| 13 | Wheel of Destiny | Beginner preview AND mainland actual wheel/presets; both workshops | A, D | Node dependencies, preset dialogs, populated gems; no resource spending |
| 14 | Quest Tracker | Header menu: remove all/completed, automatically track new/untrack completed | A, D | Populated-entry menu, automation effects and persistence |
| 15 | Unjustified Points | Open-points count, three threshold bars | A | Nonzero states and explanatory tooltips |
| 16 | Prey | Beginner restriction AND mainland two choice slots/rerolls/locked third | A, D | Populated bonus states and reroll/selection responses, not another initial opening |
| 17 | Kill Tracker | Prey-creature tracking and inactive state | A | Active tracking and context controls |
| 18 | Reward Wall | Mainland seven-day cycle, bonuses, streak/jokers, history | A | Available reward/claim states; do not claim rewards for inspection |
| 19 | Analytics Selector | All nine entries opened; menus, empty and Premium states | A, D | Populated calculations, session boundaries and remaining controls |
| 20 | Bosstiary | Grid, filters, progress, search/pagination through Cyclopedia | A, D | Shortcut destination and unlocked boss detail |
| 21 | Boss Slots | Locked slots, boosted boss, unlock conditions | A, D | Shortcut destination and unlocked selection flow |
| 22 | Bosstiary Tracker | Empty docked tracker/header | A | Populated entry and header menu |
| 23 | Bestiary Tracker | Creature, numeric progress and bar | A | Context controls and update/removal behavior |
| 24 | Imbuement Tracker | Equipment and imbuement slots | A | Active duration/expiry and menu controls |
| 25 | Weapon Proficiency | Catalogue/search/filters, XP, perk tree, requirement warning | A | Eligible-weapon details and perk interactions |
| 26 | Exaltation Forge | Fusion, Transfer, Conversion, History | A | Selection-dependent validation, confirmations and populated history |
| 27 | Social | Team finder/filter form; Premium restriction | A | Friends/invitations/config/search/badges are static discoveries, not verified interiors |
| 28 | Task Board | Bounty, Weekly, Hunting Task Shop, controls/rewards/upgrades | A | Selection-dependent details and responses; no reroll/claim/purchase |

This inventory has 28 entries, not 28 completed modules. All 28 identities were
recorded; depth varies. No honest completion percentage follows from this count.

## Settings coverage retained

A records inspection of all 20 visible settings pages. This is page coverage,
not proof of every hidden control or runtime effect. Do not restart those pages.

| Group | Established | Remaining targeted checks |
| --- | --- | --- |
| Basic / Controls | Page layouts; three mouse presets | Classic loot-button selector and dependent controls |
| General / Action Bar / Custom Hotkeys | Profiles, chat contexts, two binding columns, key capture/conflict, spell/object/text editors | Gap between general-action captures 02/03; auto-switch rules, profile removal confirmation, full spell-dependent fields |
| Interface / HUD / Console / Game Window | Pages inspected; 35 HUD conditions; target-marking and loot-colour choices | HUD ordering, master/child and aggregate semantics; Console/context details not explicitly enumerated |
| Action Bars / Shortcuts | All nine bars individually verified at 50 slots; all shortcut identities | Edge-master/retained-child behavior, clear confirmations and persistence |
| Graphics / Effects | Linux engines, antialiasing, lighting and opacity controls | Platform-specific availability and actual effect/dependency checks |
| Sound / Battle Sounds / UI Sounds | Volume/filter families and device selection | Audio consumers, unavailable-device handling, master/child behavior |
| Misc / Gameplay / Screenshots / Help | Controls and 18 screenshot triggers; help/import/export/reset routes | Trigger effects, validation and non-destructive dialog inspection |

## Nested and non-shortcut gaps

- Cyclopedia Character: A already records general/offence/defence/misc stats,
  deaths/PvP history, achievements, item summary, appearances and titles.
  Store Summary's lower scroll region remains explicitly unverified.
- Magical Archive: D adds Additional source/description and the conditional
  Rune Spell tab. Other conditional spell fields and assignment completion remain.
- Items/Manage Containers: A already records the Loot/Obtain category matrix,
  fallbacks, filters, list controls and restrictions. Remaining work is exact
  unrecorded choices, validation and behavior. D now records backpack sorting
  by four keys in both directions, nested controls and Obtain Container movement;
  filtering and recursive confirmations remain unverified.
- Chat: D now records Server Log's read-only-tab/save/clear menu. Participants,
  mute, player-channel/entry menus, secondary-chat routes and actual save behavior
  still need targeted checks; file vocabulary alone does not establish behavior.
- Boss difficulty, depot search, stash, market and inspection: A's semantic
  sweep identifies these extra dialogs. Their complete runtime layouts remain
  unverified; the 28 shortcuts do not bound the whole client.
- Cross-cutting: saving/restoring settings, reconnect state, errors, populated
  account data and server consumers are not established by static menu inspection.

## Reconciled contradictions and duplicate work

| Earlier statement | Resolution for continuation |
| --- | --- |
| HUD has six conditions | B records 35 after scrolling; six was an incomplete viewport |
| Only first action bar verified at 50 | C individually verifies all nine; do not repeat the capacity sweep |
| Object/text/profile editors still uninspected | C already records them and key conflicts; preserve those results |
| Character subpages pending in A's initial table | A's later Character stat subpages section supplies detailed coverage |
| Prey interior wholly unknown | A records a mainland-character interior; D's later beginner restriction is account-specific |
| Wheel known only in preview | A separately records actual mainland wheel/presets; D preview adds no universal restriction |
| Forge, Proficiency and Task Board not yet inspected | A's mainland follow-up explicitly records these modules |
| Supply menu / Drop Tracker destination unknown | A already records both; D's live checks corroborate them rather than discovering new scope |
| All nine analysers uninspected | A records all nine entry routes; populated behavior and Premium contents remain open |
| Modal filenames prove analyser content | D rejects three world-only captures; retain A's textual observations without promoting rejected images to evidence |

## Next work rule

Source-first order: E's mapped upstream layout/handler, A's installed-original
resource vocabulary, then live comparison of the remaining uncertainty. E now
narrows the Store Summary lower-region check to hirelings/house items and supplies
Quest Tracker persistence, console conditions, stash amount selection and preset
auto-switch references. Container weight sorting and Obtain movement are upstream
stubs: they must not be counted as implemented behavior.

For each new probe, name one unresolved cell above and append its result with
evidence to the detailed inventory. Reopen an established screen only to reach
that gap or resolve a concrete contradiction. Historical written observations
are retained with their provenance, not silently relabelled as fresh verification.
No current-account restriction erases an earlier observation on another account.
