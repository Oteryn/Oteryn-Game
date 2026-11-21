# Both-Discord Charms pass

Read2026-11-13 through the owner's authorized Discord UI session. Both
**Tibia Analysers** and **Kusnier's server** were actually opened and searched.
These are prioritized Global Tibia community sources. Classification below
describes the evidence supplied by each message, without changing its server
provenance to OTS. Comparison main remains
`3e168ce6afe374485da20ee0ac13d3749558e2f8`.

The JSON matrix retains ten observed result windows, six dated/permalinked
message contexts and five invalid search attempts excluded from conclusions.
Counts describe displayed results, not the number of tests or messages audited.
This bounded pass does not claim complete server-history coverage.

| Topic / source | What the context establishes | Oteryn comparison / qualification |
| --- | --- | --- |
| [Danashi8Sep2026,11:53](https://discord.com/channels/836592213327609957/1381954407716884630/1546820737547903007) | Reports Itexo's statement that Cleanse immunity to another status replaces the previous immunity. His immediate next message says the video is six years old, before charm rework, and may no longer hold. | Current `condition.rs::commit_cleanse` removes only matching-key immunity and retains other keys. This gives a precise historical single-slot versus per-type conflict candidate. It does not prove a current Global defect. The original video was not linked in the inspected context. |
| [Arwa8Sep2026,11:26](https://discord.com/channels/836592213327609957/1381954407716884630/1546813947854856233) | Asks per-hit versus per2sec-turn Cleanse activation with a five-hit combo. Danashi at11:41 interprets the description but explicitly has no proof. | Separate attack/hit seams exist in Rust. This exchange does not resolve a Global turn cap or Dodge+Minor ordering. |
| [Myrkz8Sep2026,11:48](https://discord.com/channels/836592213327609957/1381954407716884630/1546819613734604920) | Tentatively reports scorpion poison ticks triggering Adrenaline; a later reply agrees tentatively while the scorpion remains alive. | Useful source-attribution/DoT eligibility lead, with no attached controlled log/clip in this context. Do not infer Dodge ordering or a complete DoT callgraph from it. |
| [Danashi13Aug2026,19:42](https://discord.com/channels/836592213327609957/1480885195711189022/1537516674633310269) | Uses non-chaining Carnage as an analogy while assuming homing missiles cannot proc charms. Adjacent planned tests and a recalled clip concern missiles. | Compatible with explicit current `CharmDamage` no-reentry rule; the message is an assumption by analogy, not a supplied Carnage chain/reward experiment. |
| [Wojtek Agresja31Oct2026,15:31](https://discord.com/channels/1431612466818977814/1431632652603232397/1566097294812520590) | Directly asserts all hit mobs crit when LowBlow activates. Preceding question specifically mentions same-type AoE versus one elemental proc. Worker independently read both messages. | Catalogue bonus alone does not settle shared/correlated critical-roll scope. Preserve same-type versus all-type ambiguity pending complete consumer tracing or a controlled recording. |
| [monka tonka24Sep2026,19:34](https://discord.com/channels/836592213327609957/836593610811768833/1552735078306807860) | Asks for Cleanse Root/Hex/Fear confirmation. Two follow-ups clarify the question; inspected adjacent messages then move to imbuements. | A question with context checked is not a positive or negative test. Existing official Hex corroboration is retained. |

The Tibia Analysers positive windows were Parry(two-page navigation, first25
read), Cleanse1, Carnage1, leech75(first25 read) and Dodge15. Kusnier positive
windows were Parry683, Cleanse227, Carnage204, leech3089 and Numb47. Only the
first result windows/relevant excerpts were reviewed; selected Cleanse and
Carnage results were opened to inspect surrounding channel messages. Most
other hits are choices, hunting impressions or questions; they do not settle
armor-isolated Parry, mana-only reflection, integer rounding or kill credit.

**Search integrity correction:** the Discord combobox sometimes concatenated
the new text with the previous query. Screenshot and DOM inspection exposed
`dodgenumb`, then `dodgdodgenumb`. Zero-result attempts intended as
`parry mana`, `immunity`, `round`, `leech has:link` and `dodge` on Kusnier are
therefore marked invalid, with no usable result count. They must not become
claims that those subjects are absent. The field was cleared with Discord's
visible Clear control. Positive matching windows and permalink context reads
are retained separately; normalized submitted strings were not captured for
every window, which the matrix states explicitly.

The pass adds research findings only. No message was sent, game action run,
runtime changed or mechanic status promoted. Original Discord media/chat dumps
are not published. The existing source receipts and public-research validator
rehash the unchanged main blobs used by these pointers.

```sh
python docs/agents/evidence/charms-client-evidence-20261113/validate_discord_research.py
python docs/agents/evidence/charms-client-evidence-20261113/validate_discord_research.py --test
```
