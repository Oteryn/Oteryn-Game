# R7 experience-threshold evidence packet

This public-safe packet supports only three bounded Reference candidates for manifest revision 5:

- cumulative experience at level 7: `2,600`;
- cumulative experience at level 8: `4,200`;
- the arithmetic span between those thresholds: `1,600`.

The immutable target remains the Global Tibia production-observable state immediately after the 2026-07-28 server save. None of the sources below captures that exact boundary. The registered values therefore remain `DERIVED`; Oteryn implementation remains `NOT_STARTED`; parity remains `PARITY_PENDING_EVIDENCE`.

## s1-current-official-experience-table

- source type: `OFFICIAL_PUBLIC`
- locator: <https://www.tibia.com/library/?subtopic=experiencetable>
- retrieved at: `2026-09-27T19:05:37Z`
- transport: direct HTTPS response, HTTP 200
- response length: `684258` bytes
- response SHA-256: `80d454985452607cf5aa004cb5e050831baad3efcb88158302f3bab24fecd321`
- response validators: no `ETag` and no `Last-Modified`; `Cache-Control: no-store`
- admitted observation: the current official table lists level 7 at `2,600` cumulative experience and level 8 at `4,200` cumulative experience
- disposition: `CLEARED`, post-target official observation
- ceiling: this capture establishes the current official values. Without a target-boundary response validator or a 2026-07-28 capture, it does not prove uninterrupted continuity through the target cut.

## s2-official-level-8-auction

- source type: `OFFICIAL_PUBLIC`
- locator: <https://www.tibia.com/charactertrade/?auctionid=2158982&page=details&subtopic=pastcharactertrades>
- stable record: archived auction `2158982`, character `Hoogari Phlen`
- auction start: `2026-05-21`
- retrieved at: `2026-09-27T19:06:25Z`
- transport: direct HTTPS response, HTTP 200
- response length: `175000` bytes
- response SHA-256: `858b4248855b6d4eae05deb3bb4b0e6ce5ec188eae50f13c6d3a8fd16a7732d5`
- admitted observation: the official record reports `Level 8` and `Experience 4,200`
- disposition: `CLEARED`, pre-target official observation
- ceiling: the record proves one official pre-target level-8 observation. It does not by itself prove the complete table, the level-7 threshold, or uninterrupted continuity to the target cut.

## s3-tibiawiki-br-target-near-revision

- source type: `COMMUNITY_CORROBORATION`
- locator: <https://www.tibiawiki.com.br/index.php?oldid=440226&title=Tabela_de_Experi%C3%AAncia>
- revision ID: `440226`
- parent revision ID: `440225`
- revision timestamp: `2026-07-12T14:43:40Z`
- MediaWiki revision SHA-1: `fc404abd0b107a111ad0f898b3c650280d4a4fcc`
- verification route: MediaWiki revision API plus pinned wikitext
- admitted observation: the pinned revision lists level 7 at `2,600` and level 8 at `4,200`
- disposition: `CLEARED`, target-near community corroboration sixteen days before the target
- ceiling: the source is community-maintained and cannot become primary Global authority. It narrows temporal uncertainty but does not prove the exact target boundary.

## s4-fandom-historical-revision

- source type: `COMMUNITY_CORROBORATION`
- locator: <https://tibia.fandom.com/wiki/Experience_Table?oldid=1038127>
- revision ID: `1038127`
- parent revision ID: `1038126`
- revision timestamp: `2023-09-15T00:13:19Z`
- MediaWiki revision SHA-1: `215beba01255525fced93bd9e1cadd7387ec1670`
- verification route: MediaWiki revision API plus pinned wikitext
- admitted observation: the pinned revision lists level 7 at `2,600` and level 8 at `4,200`
- disposition: `CLEARED`, historical community corroboration
- ceiling: age and community authorship prevent promotion to primary or exact target-cut evidence.

## s5-ots-hypothesis-inventory

The owner requested Canary and CrystalServer as implementation references. They were inspected at exact immutable revisions and retained as read-only migration hypotheses:

| implementation | exact revision | exact file/blob | observation | disposition |
|---|---|---|---|---|
| Canary | [`47dfd51f45280a59a1d3e50ba7edd573d7234446`](https://github.com/opentibiabr/canary/commit/47dfd51f45280a59a1d3e50ba7edd573d7234446) | [`src/creatures/players/player.cpp`](https://github.com/opentibiabr/canary/blob/47dfd51f45280a59a1d3e50ba7edd573d7234446/src/creatures/players/player.cpp#L4537-L4539), blob `d1b12e93fdf34598af61c7f69adceac9d977e819` | `getExpForLevel` uses the standard cubic cumulative-XP expression; substitution yields 2,600 and 4,200 for levels 7 and 8 | `OTS_HYPOTHESIS_ONLY` |
| CrystalServer | [`ff7ede593c69d4c658b382c97443e8155926924a`](https://github.com/zimbadev/crystalserver/commit/ff7ede593c69d4c658b382c97443e8155926924a) | [`src/creatures/players/player.cpp`](https://github.com/zimbadev/crystalserver/blob/ff7ede593c69d4c658b382c97443e8155926924a/src/creatures/players/player.cpp#L4517-L4519), blob `e43d3d671ccce995c97976931c3ec6ec298301d9` | the same cumulative-XP expression appears at the pinned revision and produces the same two values | `OTS_HYPOTHESIS_ONLY` |

Both repository revisions post-date the target. Agreement between two OTS implementations is useful for locating the likely formula and future fixture design, but it is not evidence of Global behavior, target-date continuity, Oteryn acceptance, or Oteryn runtime implementation. These OTS locators are intentionally excluded from the manifest cases' authoritative `target.sources` lists.

## s6-field-disposition-and-arithmetic

| field | proposed value | classification | confidence | evidence ceiling |
|---|---:|---|---|---|
| `XPThreshold(7)` | `2,600` | `DERIVED` | `MEDIUM_HIGH` | target-near and historical community revisions agree with the post-target official table; no exact primary target-boundary capture |
| `XPThreshold(8)` | `4,200` | `DERIVED` | `HIGH` | official pre-target auction plus target-near community revision and current official table agree; exact boundary continuity remains unproven |
| `LevelXPSpan(7)` | `1,600` | `DERIVED` | `MEDIUM_HIGH` | exact subtraction `4,200 - 2,600`; inherits the continuity ceiling of both threshold inputs |

The arithmetic span is a table interval. It is not a death-loss percentage, a `160 XP` death penalty, awarded kill XP, a low-level bonus, an order-of-operations rule, a delevel rule, or a rounding rule.

## Explicit exclusions

- no Combat, Server Seam, Movement, native-room, loot-runtime, DUR-03, protocol, client, SQL, persistence or production change;
- no execution of legacy Lua or OTS code as evidence;
- no exact target-boundary `PROVEN` classification;
- no `PARITY_CONFIRMED` claim;
- no runtime policy, fixture or threshold implementation in Oteryn;
- no death-loss rate, blessing modifier, PvP modifier, awarded kill XP, low-level multiplier, delevel behavior or rounding claim;
- no inference that access to a public page grants authority beyond the factual fields admitted above.

## Provenance and legal disposition

Only factual numeric values, public revision metadata and short implementation observations are paraphrased. No proprietary asset or long-form copied text is stored. Public source locators remain traceable, and community/OTS evidence retains its lower authority classification. Provenance and legal disposition for the bounded fields above is `CLEARED`; unresolved target continuity remains explicit in each case.
