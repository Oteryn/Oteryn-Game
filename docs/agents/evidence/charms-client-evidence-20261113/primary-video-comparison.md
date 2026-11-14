# Itexo and Alytreta: bounded primary-video cases

Read2026-11-13; source main `3e168ce6afe374485da20ee0ac13d3749558e2f8`.
This successor observes original video frames and exported captions, reproduces
specific source arithmetic, and preserves six cases in `primary-video-cases.json`.
Original video, images and caption text remain unpublished.

## Itexo: Cleanse immunity replacement

The verified original is [CHARMS: NUMB, CLEANSE and LOW BLOW: Testing and
Explanation](https://www.youtube.com/watch?v=W_0EnSycMNI), channel Itexo,
published11June2020 according to expanded YouTube description. Cleanse chapter
runs0:38–1:15. This matches Danashi's creator/trio/six-year-age/specific
conclusion; the inspected Discord exchange itself did not supply a URL, so
message-linked identity remains unproven.

Sampled original frames0:38,0:51,1:01 and1:06 show actual game footage, attacks
and a status-icon strip; [1:06](https://www.youtube.com/watch?v=W_0EnSycMNI&t=66s)
shows the Cleanse poison-removal notification. Exported Spanish captions
(authored or unspecified) at1:01–1:13 explicitly describe the author's own test:
a second proc on another status replaces the first status's immunity, and
immunities do not accumulate. This is stronger provenance than the later
Discord recollection. The sampled frames alone do not establish a continuous
11sec expiry/reapplication trace or the full RNG/condition-instance sequence.

Current `foundation/condition.rs` has these relevant seams:

- `shared_mutation_slot`, lines81–87: only SpellSkills(0) aliases Attributes;
  it does not collapse all negative conditions into one global immunity slot.
- `prepare_cleanse`, lines1417–1459: selects an eligible retained instance and
  computes an11sec immunity deadline.
- `commit_cleanse`, lines1463–1480: removes the selected instance, removes only
  matching-key immunity, and inserts the new deadline. Other keys survive.

For a **derived illustrative** sequence, poison immunity expires at11000ms,
then a successful fire proc at2000ms gives fire immunity until13000ms. Current
storage retains both distinct keys until expiry; the historical creator rule
would retain fire only. These times are explanatory inputs, not measured film
times. Receipt replay protection is a separate rule and does not answer what
a new successful proc does. No current Global verdict is made from a2020 film.

## Alytreta: two identical targets and rounding

[Original video](https://www.youtube.com/watch?v=MxiCFl3OT_8), channel Alytreta,
was linked by the2022 Reddit discussion. Its Portuguese automatic captions
help locate the experiment, while actual frames supply the numbers below.
At[11:45](https://www.youtube.com/watch?v=MxiCFl3OT_8&t=705s), the creator's
two-identical-creatures tables show both on/off samples; the off table was
also read at12:03 and12:13. Gear life leech is50%; Vampiric uses its historical
4% bonus. Values are tabulated by the creator rather than independently
recomputed from all original combat receipts.

| Mode | Displayed Hit | Displayed heal | Candidate `Hit × amount × 1.1/2` | Rounded candidate | Current unscaled helper if `real=Hit` |
| --- | ---: | ---: | ---: | ---: | ---: |
| Vamp off,50% |795|219|218.625|219|398|
| Vamp off,50% |784|216|215.600|216|392|
| Vamp on,54% |958|285|284.526|285|517|
| Vamp on,54% |948|282|281.556|282|512|

All four match nearest-integer results under the candidate target factor and
reject direct floor on these specific inputs. **None is a half-integer tie**,
so they do not prove half-away versus ties-to-even, universal formula, mixed
target grouping or per-target versus aggregate rounding. Current stage bonuses
must not be substituted for the old4% when interpreting the old measurements.

The exact source comparison is:

- `combat-rules/resolution/equipped_offence.rs::leech_amount`, lines153–159:
  integer `(real*skill+5000)/10000`, clamped to real damage; skill is hundredths
  of a percent. The unscaled column reproduces this helper conditionally.
- `dispatch/attack/equipped_offence.rs::leech_hit`, lines74–100: supplies
  `health_before-max(health_after,0)` to the helper for each committed result.
- `dispatch/attack/drain.rs`, lines850–864: applies charm bonuses, adds talisman
  life skill where eligible, then calls `leech_hit` with that result/offence.
- `dispatch/attack/charm_combat.rs::bonuses`, lines199–225: adds percentage
  points to an existing equipment leech skill. It does not grant missing gear
  leech or apply an AoE denominator at this seam.

These inspected seams provide a concrete source divergence candidate, not a
deployed Oteryn execution or complete audit of every possible upstream path.

## Alytreta: overkill client log

At[23:20](https://www.youtube.com/watch?v=MxiCFl3OT_8&t=1400s), paused original
client footage shows30HP lost by a cave rat and258HP healed. The webcam covers
the beginning of the log lines, but the numeric loss/heal and recording date
26June2022 are readable. Narration23:06–23:16 independently describes the
258-heal/30HP/50%-life-leech example and identifies test server elsewhere in
the same demonstration. This remains **historical test-server evidence**.

With30 committed HP loss and50% gear, the inspected current source gives
`(30*5000+5000)//10000 = 15`, versus recorded258. The nominal attack magnitude
before victim-HP clamping is not uniquely readable, so the report does not
invert258 into an invented raw hit. This case exposes the consequence of
using actual HP loss as the leech base; it does not prove current Global rules.

## Scope and source integrity

The additional owner URL1392508385475166248 was actually opened: Kusnier's
`community-tools-list`, posted9July2025 and edited21Oct2026. Its inspected list
contains tools including TibiaTools; it supplies no Itexo film link or new
Cleanse measurement. No unrelated tools were installed or followed.

The preceding both-Discord pass was shallow, bounded first-window research.
The owner's filtered Wound/pierce screenshot is a concrete next lead, not
something this video-only successor claims to have investigated. No prior
zero-result or partial window establishes absence of evidence.

```sh
python docs/agents/evidence/charms-client-evidence-20261113/validate_primary_video.py --repository .
python docs/agents/evidence/charms-client-evidence-20261113/validate_primary_video.py --test
```
