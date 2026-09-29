# Tibia manual notes: magic

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=magic>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `3a1ec9ea896216b77a2cf6f204d57f0fa443cb10eaf39e8deae3dfd360c8973c`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Server mechanics
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §magic <heading>, capture 2026-09-28`.

## 5.4 Magic (overview)

- [both] Spells range from utility (food, ammo, light) to offensive damage; spellcasters can also wield wands/rods and craft runes. Full spell list lives in the library section, not this manual page.

## 5.4.1 How to Learn Spells

- [server] Starting spells on Newhaven are vocation-dependent and intentionally weak/basic.
- [server] New spells unlock automatically as the character levels, whenever the vocation has a spell defined at that level threshold — no manual "learn" action needed.
- [server] A newly unlocked spell is immediately usable.
- [client] Newly learned spells auto-populate the action bar, provided that spell isn't already on the bar and the auto-add option is enabled in Options.
- [client] Spellbooks (purchasable in magic shops or from other players) record known spells for reference; optional convenience, not required once experienced.
- [server] Druids and sorcerers can additionally use a spellbook as an off-hand item functioning like a shield (defensive equipment slot use).

## 5.4.2 Casting Spells

- [server] Spells are cast by typing the exact incantation text and pressing Return (e.g. "utevo lux" for Light).
- [server] Casting a spell consumes mana; insufficient mana blocks the cast (shows "You do not have enough mana" + a smoke effect, no other consequence).
- [server] Casting a spell not yet learned, or below the required experience level, fails with "You must learn this spell first" or "Your level is too low" respectively (also with a smoke visual).
- [client] Mana level is visible via a blue status bar and in the character's skills menu.
- [server] Every spell has a cooldown, visually tracked in a cooldown bar.
- [server] 3 primary spell groups: Healing, Attack, Support; a spell may also belong to a secondary group.
- [server] Casting a spell puts a cooldown on that spell AND on every group it belongs to; while on cooldown, that spell or any other spell sharing a cooled-down group cannot be cast.
- [server] Attempting to cast while on cooldown shows "You are exhausted" and fails.
- [server] Cooldowns do NOT tick down while the character is logged off (frozen, not paused-and-resumed differently — simply doesn't decrease offline).
- [client] Hotkeys can be bound to spells; pressing one auto-fills the console entry line with that spell's incantation for fast casting (does not auto-submit, based on description — types text ready for Return).

## 5.4.3 Magic Level

- [server] Magic Level (magic skill) trains like other skills: via casting spells, using wands/rods, or offline training — increases after enough cumulative mana has been spent this way, with a confirmation message on level-up.
- [server] Vocation affects training speed for magic level: druid/sorcerer = fast; paladin/monk = slower; knight = very slow ("painfully slow" qualitative framing).
- [server] Magic level directly scales spell damage/healing power — same spell, higher magic level = more damage/healing, independent of character level (example given: level-25 caster with magic level 34 out-damages the same spell at magic level 30).
- [server] Rune usage requires BOTH a minimum experience level AND a minimum magic level (both gates independently enforced). Example given: "Ultimate Healing" rune requires magic level 4 and experience level 24.
- [server] This magic-level gate on runes matters for non-primary casters too (paladins, monks, knights), since they rely on runes more than instant spells.

## 5.4.4 Wands and Rods

- [server] Exclusive to Druids (rods) and Sorcerers (wands) — their equivalent of a knight's melee weapon.
- [server] Equipped in a hand slot; usable simultaneously with a shield (unlike two-handed distance weapons).
- [server] Activated by attacking a target via battle list or game window; functions as a limited-range ranged attack.
- [server] Each shot consumes mana; that spent mana counts toward magic-level training progress, same as casting a spell.
- [server] If mana is insufficient, the wand/rod simply won't fire until enough mana regenerates for the next shot (no explicit error stated, contrast with 5.4.2's explicit messages for spells).
- [server] Wands/rods vary by: weight, mana cost per shot, attack damage, and damage type; higher-tier versions cost more mana per shot.
- [server] Minimum experience level requirement applies per wand/rod model.
- [server] Entry-level items: "Wand of Vortex" (sorcerer) and "Snakebite Rod" (druid) require level ≤7; explicitly framed as beginner-friendly due to low mana cost enabling frequent use, despite lower damage.
- [server] Every druid/sorcerer receives one starter wand/rod for free on Newhaven; additional ones purchasable in magic shops in every major city.
- [server] Even late-game, experienced casters keep a wand/rod equipped as a supplementary damage source alongside rune/instant spells (rune/instant spells eventually outperform wands/rods in raw power, but wands/rods remain cheap/reliable filler damage).

## 5.4.5 Runes

- [server] Runes are consumable items that store a spell's effect for later use, distinct from instant-cast spells (a "class of spells" that only exist as runes).
- [server] Creating a rune: cast the rune-creation spell while holding/carrying a blank rune in inventory; exactly one blank rune is charged per cast, but the resulting charge may split into multiple charged runes from that single blank (2, 3, or up to 10 output runes per successful cast, spell-dependent).
- [server] Blank runes and pre-charged runes are both purchasable from magic shops.
- [server] A successfully charged rune changes colour/appearance and becomes a usable "Use with..." targeted item.
- [client] Targeting workflow: right-click rune → "Use with..." → crosshair cursor → click target in game window; OR select the target directly from the battle list (no need to click the creature's sprite) for easier aiming.
- [server] Runes are single-use (consumed on cast) but stack up to 100 per pile.
- [client] Remaining rune count in a stack is displayed as a small number badge, shown only while the stack is inside a container (backpack/depot).
- [server] Using a rune requires both a minimum magic level (shown by inspecting the rune) and a minimum experience level (listed in the spells section); note the listed experience level for a rune may represent either the level needed to USE it or the level needed to CREATE it — the manual flags this as a possible ambiguity per rune entry.

## 5.4.6 Using Magic in Combat

- [both] Druids/sorcerers are described as weak melee fighters who must lean on their large offensive-spell/wand/rod kit to compensate.
- **Aiming Runes**
- [server] Target selection via battle list works for runes too (fire-and-forget once target selected).
- [server] Offensive/attack runes cannot be aimed at another character within the first 10 seconds after that character's login, UNLESS the caster has been attacked first and is defending themselves; this 10-second restriction does NOT apply to field runes (field runes can be placed regardless of the target's login timer).
- **Area Attacks**
- [server] Attack spells vary widely in area-of-effect, from single-tile (missile/strike) up to very large multi-tile coverage (e.g. "Great Fireball", "Hell's Core").
- [both] Large AoE spells are easy to aim but hard to precisely predict the footprint of, risking accidental hits on bystanders — which can trigger skull/marking consequences (ties to combat 5.3.12).
- [both] Recommended mitigation: form a party with nearby friendly players before casting AoE spells, since party members are excluded from area-spell damage (per combat 5.3.13).
- **Immunities and Sensitivities**
- [server] Creatures vary in sensitivity per damage type on a spectrum, not binary; most creatures take at least some damage from most types.
- [server] A small subset of creatures are FULLY immune to one or more specific damage types (example given: a water elemental takes zero damage from fire, earth, or ice).
- [both] Recommended to consult the creatures/spells library sections to match spell/wand/rod choice to a specific enemy's resistances.
- **Spend your Mana Wisely**
- [both] Recommended to always keep enough mana/runes in reserve for emergency self-healing.
- [both] "Invisibility" and "Magic Shield" spells are highlighted as key defensive/escape tools.
- [server] Mana potions (purchasable at magic shops) provide a fast way to restore mana mid-hunt.
- [both] Wands/rods are again emphasized as an always-bring backup damage source for spellcasters.

## Open questions for Oteryn

- No exact cooldown durations (seconds) are given for any named spell or spell group (5.4.2) — needed per-spell for server implementation, must be sourced from the spell library, not this manual page.
- No exact mana costs are given for any specific spell (5.4.2, 5.4.4) beyond qualitative "low cost" for beginner wands/rods.
- No exact magic-level training rate/formula (mana-spent-to-level-up curve) is given (5.4.3) — only that "enough" spent mana triggers a level.
- No numeric vocation multipliers for magic-level training speed (fast/slow/painfully slow are qualitative only) (5.4.3).
- No formula is given for how magic level scales spell damage/healing (5.4.3) — only a directional example (magic level 34 > magic level 30 at same char level).
- Ambiguity explicitly flagged by the manual itself: for runes, whether the listed "experience level" in the spell library refers to the level required to USE vs to CREATE the rune is not always clear per entry (5.4.5) — needs per-rune data clarification from the library section.
- No numeric attack damage/damage-type table is given per wand/rod model (5.4.4) — only that it "varies"; must be sourced from item data, not this manual page.
- No explicit confirmation whether the 10-second no-attack-rune-on-fresh-login rule (5.4.6) is identical to the general 10-second PvP-attack immunity in combat 5.3.11, or a separate rule — treated here as likely the same mechanic but worth verifying against combat section wording.
- No numeric AoE radius/shape data given for any named area spell (5.4.6) — must come from spell library, not this manual page.
