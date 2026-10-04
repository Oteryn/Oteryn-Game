# Cached monster spell references

This source-only sidecar preserves all 5,237 Canary/Crystal source profiles and 20,742 attack/defense slots from the immutable r28 archive. It joins exact source file identities and parameters to the existing r22 current audit. It does not modify source data, slot admission, engine behavior or runtime activation.

The cached Tibiopedia observations cover 1,869 species: 1,736 readable pages and 133 unavailable pages, with 5,680 recorded attack observations. Each fact retains its requested/final URL, observation date, original normal HTTP read method and page digest when available. This export reads only verified local cached facts and makes no network requests. All 1,820 records in the prior `monster-references` facts file are preserved identically in the current 1,869-record file; no second fetch is needed.

`cached-monster-spell-facts.jsonl.gz` retains literal attack labels, published elements/ranges and target observations. A scalar damage label whose prior parser did not normalize a range remains `numeric_range: null`. Empty effect wording never becomes an effect identifier. Creature abilities remain page observations rather than inferred spell mechanics.

`cached-monster-profile-coverage.jsonl.gz` keeps every source variant and its exact revision/path/SHA/blob identity. The 284 profiles without attack/defense slots remain present with empty slot lists.

`cached-monster-slot-comparisons.jsonl.gz` retains every cached field comparison after proving the whole source parameter dictionary is unchanged, including false, zero and null types. The observed damage range and element-based candidates do not prove a unique source spell match. No slot is marked fully wiki verified. The audit contains 137,377 unpublished field observations and 10,818 fields without an unambiguous external match.

`missing-monster-spell-reference-targets.jsonl.gz` lists 1,757 species that actually have source attack/defense slots, their source candidate variants, exact captured URL, page availability and uncertain source fields. The 112 species with no source slots are preserved in coverage and facts but create no unnecessary spell-research targets.

BR and Fandom availability statements come from the dated cached audit, not a new connectivity check: BR returned HTTP403; Fandom returned HTTP402; Remote Desktop was unavailable. The cached audit reports that existing BR creature normalization omits attacks. Those data cannot substantiate spell interval/chance/effect claims. Their missing spell evidence is explicit; no guessed page URL or bestiary-to-spell verification is emitted.

The proof binds the original archive and all four source members, cached manifests, schema, producer and compressed/payload digests. The shared strict schema validates each line according to its `record_kind`. All datasets have `runtime_activation: false`; profile, fact and comparison records also have `source_override: false`.
