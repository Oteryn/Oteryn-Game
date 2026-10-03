# MONSTER-D15-BESTIARY: Bestiary profiles from the reference-date TibiaWiki

- Decision: `MONSTER-D15-BESTIARY-V1` (schema row D48)
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (content) and
  protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: owner decision 3b (#1622 comment 5968564302, 2026-10-03). D15 is extended, so the
  converter takes the Bestiary from the reference-date TibiaWiki for every creature. Until now it
  made single pinned corrections (architect ruling 5968176175, MONSTER item 2).
- Allocation: D286 (#1622 comment 5968568302), task `OTV2-20261003-monster-d15-bestiary`.
- Builds on: `OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md` D15, D33, D44, D47 and §9.1, §10.3. Also
  CHARM-0 (the Bestiary fields are Charm progression inputs) and PROF-EFFECT-0 §3.2 (proficiency
  points by difficulty and occurrence). Source order is the FORMULA rule (owner rule 5905825574).
- Runtime, migration and production authority: **NONE**. The converter change is the Monster lane
  child below, under its own allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| MONSTER-D15B-1 | content (Monster lane) | wiki extraction of `bestiaryclass`, `bestiarylevel`, `occurrence`; the derivation table (§3.2); membership join (§3.3); converter adoption with manifest rows; population re-conversion; CONFLICT report; tests in §5 | this decision |

There is no new schema field, command, state domain or durable table. CHARM, PROFICIENCY and
Bosstiary state are untouched.

## 1. Question

Where does every Bestiary field of a Creature definition come from, and how is a creature's
membership in the Bestiary decided?

## 2. Facts

**PROVEN (repository)**

- §9.1 already adopts Bestiary difficulty and occurrence from the reference-date wiki. It does
  not adopt the Bestiary class. Kill thresholds and charm points come from Canary (`FirstUnlock`,
  `SecondUnlock`, `toKill`, `CharmsPoints`, see `canary_batch.py` `bestiary_payload`).
- The schema's `bestiary` carries `class`, `taxonomy`, `difficulty`, `occurrence`, `stars`,
  `kill_thresholds`, `charm_points`, `locations` and `notes`. The v2 mapping matches difficulty,
  occurrence, thresholds and charm points. Class, taxonomy, stars and locations are a GAP there
  (§5).
- The official 15.30 client staticdata (`imports/cipsoft-staticdata/creatures/`) lists 833
  Bestiary races with `source_id` and name. Two fields there look like a difficulty tier (`f4`)
  and an occurrence tier (`f5`). That reading is inferred, not labelled by the source.

**PROVEN (English TibiaWiki, read 2026-10-03)**

- The `Infobox Creature` fields are `bestiaryclass`, `bestiarylevel` and `occurrence`, for
  example `Bride of Night` (revision 1192294: Human, Medium, Very Rare) and
  `Muglex Clan Footman` (revision 1203216: Humanoid, Trivial, Uncommon).
- `Template:Bestiary Table`, revision 1152628, is used on `Cyclopedia#Bestiary` and
  `Bestiary/Difficulties` (revision 1132172). Its rule: the difficulty sets the kills and charm
  points, and every Very Rare creature needs 2/3/5 kills with double charm points.

| Difficulty | Kills, not Very Rare | Charm points | Kills, Very Rare | Charm points |
|---|---|---|---|---|
| Harmless | 5 / 10 / 25 | 1 | 2 / 3 / 5 | 5 |
| Trivial | 10 / 100 / 250 | 5 | 2 / 3 / 5 | 10 |
| Easy | 25 / 250 / 500 | 15 | 2 / 3 / 5 | 30 |
| Medium | 50 / 500 / 1000 | 25 | 2 / 3 / 5 | 50 |
| Hard | 100 / 1000 / 2500 | 50 | 2 / 3 / 5 | 100 |
| Challenging | 200 / 2000 / 5000 | 100 | 2 / 3 / 5 | 200 |

The table is read exactly as written, including the Harmless Very Rare row (5 charm points).

**OTS_HYPOTHESIS_ONLY**: Canary `47dfd51f` and CrystalServer `00ce02a5` Bestiary blocks.

## 3. Decision

### 3.1 Fields (schema D48)

For every creature with a Bestiary profile, the converter takes these values from the English
TibiaWiki page at the reference date (D33), each pinned to the page revision with a
MediaWiki-sourced manifest row. The superseded Canary or Crystal value stays as an
`approved_omission`, as D15 does today.

| Field | Source |
|---|---|
| `class` | `bestiaryclass` |
| `taxonomy` | the class, as §9's 5-monster rule already does; a valid Canary race that disagrees with the class becomes CONFLICT (§3.4) |
| `difficulty` | `bestiarylevel` (unchanged from D15) |
| `occurrence` | `occurrence` (unchanged from D15) |
| `stars` | derived from the difficulty, Harmless 0 to Challenging 5 (unchanged converter rule) |
| `kill_thresholds` | derived from difficulty and occurrence by the §2 table |
| `charm_points` | derived from difficulty and occurrence by the §2 table |
| `locations`, `notes` | not adopted (`locations` stays source text, `notes` stays Oteryn-authored) |

The derived values are not page fields. Each derived value's manifest row cites the template
revision and the two inputs it was computed from.

### 3.2 Derivation table

The §2 table is committed once as converter data. It is keyed by the closed difficulty and
occurrence vocabularies and carries the template revision. An unknown difficulty or occurrence
string stops the conversion. Nothing falls back to Canary silently.

### 3.3 Membership

A creature has a Bestiary profile when both of these hold:

- it is in the official client race table (`imports/cipsoft-staticdata/creatures/`), joined by the
  Canary or Crystal `raceId` equal to `source_id` with the names equal case-insensitively, or, for
  a D44 wiki-authored creature, by the exact client name;
- its reference-date wiki page states all three of `bestiaryclass`, `bestiarylevel` and
  `occurrence`.

The outcomes:

- In both sources and without a Canary profile: the profile is **created** from the wiki. This is
  the Muglex case.
- In both sources and with a Canary profile: the profile is **replaced** field by field under
  §3.1.
- In neither source and with a Canary profile: the profile is **removed** as an
  `approved_omission` citing both sources.
- In only one source, a failed join, or a client race with no converted creature: listed as
  CONFLICT or UNRESOLVED (§3.4). The Canary profile, or no profile, stays.

The join is never by name similarity, and a creature never takes another creature's values.

### 3.4 Conflicts

The converter writes one report of every CONFLICT and UNRESOLVED row, grouped by kind:

- the join failed;
- membership is in one source only;
- the wiki class disagrees with a valid Canary race;
- the wiki difficulty or occurrence disagrees with the client `f4`/`f5` tier, under the inferred
  mapping already used by the converter (stars 0-5, occurrence 0-3);
- a wiki field is unparsable or has a placeholder (`?`, empty).

These rows are not adopted. Each one keeps its current value until the architect rules on its
group in one batch (the QUEST item 3 procedure). The client tiers are only a cross-check, because
their meaning is inferred. If the report shows they agree with the wiki everywhere except
explained cases, the architect may propose them as the official source in a later decision.

### 3.5 The five pinned profiles

The five profiles ruled on 2026-10-03 (Bride of Night, Doomsday Cultist, Midnight Warrior and the
two Muglex creatures) become cases of §3.1-§3.3, and their pins become test fixtures. If the
general rule gives a different value than a pin, the rule wins, and the report names the
difference.

### 3.6 Effect on consumers

- Charm progression (CHARM-0) and proficiency points (PROF-EFFECT-0 §3.2) read the converted
  profile, so they take the new values with the next content revision. No state migration is
  needed: kill counters and points already earned are character state. A changed threshold
  applies to the next unlock check.
- A removed profile ends Bestiary progress, charm assignment and proficiency points for that
  creature, as in Tibia. The CHARM runtime child handles assigned charms on such races when it
  ships. Until then no runtime consumes the content.

## 4. Decision test

1. **Must decide now?** YES. The owner chose 3b, and the Monster lane's next population pass and
   the CHARM and proficiency content children need one rule instead of single pins.
2. **What is blocked?** MONSTER-D15B-1; PROF-CONTENT-2's `bestiary_class_id` crosswalk
   (PROF-EFFECT-0), which needs the class for every creature; CHARM content parity.
3. **What becomes harder later?** Once Bestiary progress is persisted, each content revision that
   changes thresholds or membership has to be explained to players whose counters cross or lose an
   unlock. Deciding before the CHARM and PROFICIENCY runtimes ship keeps that cost at zero now. The
   choice also binds content to TibiaWiki's taxonomy, so a later official schema would need a
   crosswalk.
4. **What would justify superseding it?** A proven labelling of the client staticdata tiers
   (official over wiki under the FORMULA order); a Tibia change to the kill or charm table; a
   CONFLICT rate in §3.4 that shows the wiki is unreliable for one field.
5. **What is not decided?** Bosstiary (D8 is unchanged); `locations`; Bestiary runtime and wire;
   how a removed profile treats charms already assigned (CHARM runtime); the free-account Bestiary
   subset.

## 5. Acceptance tests for MONSTER-D15B-1

- The derivation table reproduces all 12 rows of revision 1152628. An unknown key fails.
- The five pinned profiles convert to the pinned values, or each difference appears in the report.
- Membership: one case each for create, replace, remove, one-source, failed join and a placeholder
  field.
- Every adopted value has a wiki or template manifest row, and every superseded value has an
  `approved_omission`.
- A census `--check` reproduces the report and the converted population byte for byte from the
  pinned inputs.
