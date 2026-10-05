# Monster spell external comparison

This evidence covers every attack/defense slot in the exhaustive pinned Canary and Crystal source censuses. It compares public Tibiopedia attack observations with source fields, retaining a separate outcome for each field. It does not certify these fields as official Tibia constants or certify runtime monster execution.

The source populations are pinned in `summary.json`. Parsed public facts are in `tibiopedia-monster-facts.jsonl`; each record includes the requested and final URL, normal HTTP access method, observation date, whole-page SHA256 and availability outcome. Original HTML and upstream Lua are not included. The source-slot join is `slot_key = source/file/block/1-based-index` in `monster-slot-field-comparisons.json`.

Tibiopedia exposes attack names, observed damage ranges, elements, some target wording and creature abilities. It generally does not publish exact engine intervals, chances, effect identifiers, condition tick schedules or scripted boss callbacks. Those fields remain explicitly `unknown_external_not_published`; unavailable pages remain explicit. Numeric equality is an observed-range comparison, not proof of a source formula. Melee attack/skill/armor semantics and ambiguity between several same-element attacks prevent automatic correction. Element presence alone is not a unique slot identity.

BR and Fandom were unavailable by ordinary HTTP and all Remote Desktop endpoints were offline. Existing normalized BR creature captures omit attack data; they cannot be used to pretend that attacks were checked. Tavily quota was exhausted. No repeated blocked-source requests were made. A public Tibiopedia `/monsters/all` link index supports canonical naming resolution without guessing page addresses.

All-field-complete wiki verification remains false where public evidence does not publish the engine field. This is an exhaustive audit with explicit unknowns, not a declaration that every spell was independently verified or fully implemented.
