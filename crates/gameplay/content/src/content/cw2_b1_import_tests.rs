use super::super::item_identity::{ItemKeyAliasTable, RetiredItemKey, tibia_item_key};
use super::*;
use serde_json::Value;

const B1_EVIDENCE: &[u8] = include_bytes!(
    "../../../../../docs/agents/evidence/OTV2-20260919-content-world-cw2-b1-item-identity-catalog.json"
);
const DONOR_CENSUS: &[u8] = include_bytes!(
    "../../../../../tools/content-schema/item-authoring/samples/donor-census-crystal-summer-update-00ce02a5.json"
);
const ALIAS_CROSSWALK: &[u8] = include_bytes!(
    "../../../../../docs/agents/evidence/OTV2-20260928-item-donor-identity-b1b-alias-crosswalk.json"
);
const CRYSTAL_BINDINGS: &[u8] =
    include_bytes!("../../../../../imports/crystalserver/bindings/items.json");
const ITEM_KEY_ALIASES: &[u8] = include_bytes!("../../../../../content/items/aliases.json");
const FROZEN_FULL_FAMILY_ALLOCATION_DIGEST: &str =
    "ee9219ccf9d8b2350911abca321507ff924ccd4cb83196efd08b91fbdf098966";

/// Donor ids held by the alias gate: 35500 and 54610 are `PROBABLE_MATCH` (a unique
/// counterpart differing only in sprite), the other six are `AMBIGUOUS` (several).
const HELD_DONOR_IDS: [u64; 8] = [35500, 53380, 54609, 54610, 54613, 54614, 54615, 54616];

fn epoch2() -> ProtectedCw2B1DonorIdentityEpoch2Import {
    protected_cw2_b1_donor_identity_epoch_2_import(B1_EVIDENCE, DONOR_CENSUS, ALIAS_CROSSWALK)
        .expect("donor identity epoch 2")
}

fn record_key(record: &ProjectReferenceRecord) -> &str {
    item_record_key(record)
}

fn binding_key(candidate: &ImportCandidate) -> &str {
    candidate
        .candidate_target
        .split('@')
        .next()
        .expect("candidate target key")
}

/// `(source id, key)` pairs of the epoch-2 batch, ascending by source id.
fn minted(imported: &ProtectedCw2B1DonorIdentityEpoch2Import) -> Vec<(u64, String)> {
    let mut minted = imported
        .batch
        .candidates
        .iter()
        .map(|candidate| {
            (
                candidate.source_numeric_id.expect("source id"),
                binding_key(candidate).to_owned(),
            )
        })
        .collect::<Vec<_>>();
    minted.sort();
    minted
}

fn bindings_file() -> Vec<Value> {
    let file: Value = serde_json::from_slice(CRYSTAL_BINDINGS).expect("bindings JSON");
    file["bindings"].as_array().expect("bindings array").clone()
}

#[test]
fn epoch_2_mints_exactly_the_pinned_range_in_ascending_source_id_order() {
    let imported = epoch2();
    assert_eq!(imported.records.len(), CW2_B1_DONOR_EPOCH2_MINTED_COUNT);
    assert_eq!(imported.records.len(), 404);
    assert_eq!(CW2_B1_DONOR_EPOCH2_FIRST_SEQUENCE, 38_094);
    assert_eq!(CW2_B1_DONOR_EPOCH2_LAST_SEQUENCE, 38_497);
    assert_eq!(
        imported.allocation_digest_sha256,
        CW2_B1_DONOR_EPOCH2_ALLOCATION_DIGEST_SHA256
    );

    let minted = minted(&imported);
    assert_eq!(minted.len(), 404);
    for (rank, (_, key)) in minted.iter().enumerate() {
        assert_eq!(
            key,
            &format!("oteryn:item.registry.i{:08}", 38_094 + rank),
            "rank {rank}"
        );
    }
    assert!(minted.windows(2).all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(minted[0].1, "oteryn:item.registry.i00038094");
    assert_eq!(minted[403].1, "oteryn:item.registry.i00038497");

    let mut record_keys = imported.records.iter().map(record_key).collect::<Vec<_>>();
    let batch_keys = minted
        .iter()
        .map(|(_, key)| key.as_str())
        .collect::<Vec<_>>();
    record_keys.sort_unstable();
    assert_eq!(record_keys, batch_keys);

    // Identity-only pointers, as for epoch-1 opaque keys; no other change of state.
    assert!(imported.records.iter().all(|record| matches!(
        record,
        ProjectReferenceRecord::Item {
            materializable: false,
            stack_class: ItemStackDocument::Unknown,
            ..
        }
    )));
    assert!(imported.alias_bindings.is_empty());
    // Eight same-name donors match an existing Item on article/plural and the full
    // attribute set and differ at most in sprite: held, with neither a key nor an alias.
    assert_eq!(
        imported.unbound_source_item_ids,
        HELD_DONOR_IDS.to_vec(),
        "sprite-only differences are held, never minted"
    );
    let minted_ids = minted.iter().map(|(id, _)| *id).collect::<BTreeSet<_>>();
    assert!(HELD_DONOR_IDS.iter().all(|id| !minted_ids.contains(id)));
    assert_eq!(minted.len() + HELD_DONOR_IDS.len(), 412);
    assert!(imported.batch.candidates.iter().all(|candidate| {
        candidate.normalized_fields.iter().any(|field| {
            field.field_path == "source.native-key-authorship"
                && field.value
                    == CandidateValue::Text("OTERYN_OPAQUE_REGISTRY_ALLOCATION_EPOCH_2".to_owned())
        })
    }));
}

#[test]
fn epoch_2_is_deterministic_and_leaves_the_frozen_epoch_1_import_untouched() {
    let first = epoch2();
    let second = epoch2();
    assert_eq!(first, second);

    let frozen =
        protected_cw2_b1_full_item_family_import(B1_EVIDENCE).expect("frozen full item family");
    assert_eq!(
        frozen.allocation_digest_sha256,
        FROZEN_FULL_FAMILY_ALLOCATION_DIGEST
    );
    assert_eq!(frozen.records.len(), CW2_B1_FULL_ITEM_FAMILY_COUNT);
    assert_eq!(CW2_B1_OPAQUE_ITEM_COUNT, 38_093);
    assert_eq!(
        frozen.allocation_digest_sha256,
        protected_cw2_b1_full_item_family_import(B1_EVIDENCE)
            .expect("frozen full item family")
            .allocation_digest_sha256
    );

    // No key and no source id is shared with epoch 1, and retired 2,921 stays retired.
    let frozen_keys = frozen
        .records
        .iter()
        .map(record_key)
        .collect::<BTreeSet<_>>();
    let frozen_source_ids = frozen
        .batch
        .candidates
        .iter()
        .filter_map(|candidate| candidate.source_numeric_id)
        .collect::<BTreeSet<_>>();
    let minted = minted(&first);
    assert!(
        minted
            .iter()
            .all(|(source_id, key)| !frozen_keys.contains(key.as_str())
                && !frozen_source_ids.contains(source_id))
    );
    assert!(
        !minted
            .iter()
            .any(|(_, key)| key == R7_P04_GOLD_COIN_OLD_KEY)
    );
    assert!(!minted.iter().any(|(_, key)| key == R7_P04_GOLD_COIN_KEY));
    assert_eq!(
        minted
            .iter()
            .map(|(_, key)| key.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        minted.len()
    );
}

#[test]
fn committed_bindings_requalify_epochs_1_and_2_through_the_alias_table() {
    // ITEM-ID-1b (A12 §4.2): every historical row, epoch 1 (the frozen allocation with
    // the R7-P04 gold coin at its promoted key) and epoch 2 (the minted donor ids), is
    // bound EXACT to the Tibia key of its own id when its retired key aliases, and
    // emits no binding when its retired key is D149 (`RETIRED_WITHOUT_SUCCESSOR`).
    let imported = epoch2();
    let frozen =
        protected_cw2_b1_full_item_family_import(B1_EVIDENCE).expect("frozen full item family");
    let aliases = ItemKeyAliasTable::parse(ITEM_KEY_ALIASES).expect("aliases");
    let historical = frozen
        .batch
        .candidates
        .iter()
        .map(|candidate| {
            let key = binding_key(candidate);
            let key = if key == R7_P04_GOLD_COIN_OLD_KEY {
                R7_P04_GOLD_COIN_KEY
            } else {
                key
            };
            (
                CW2_B1_SOURCE_REVISION,
                candidate.source_numeric_id.expect("source id"),
                key.to_owned(),
            )
        })
        .chain(
            minted(&imported)
                .into_iter()
                .map(|(id, key)| (CW2_B1_DONOR_EPOCH2_SOURCE_REVISION, id, key)),
        )
        .collect::<Vec<_>>();
    assert_eq!(
        historical.len(),
        CW2_B1_FULL_ITEM_FAMILY_COUNT + CW2_B1_DONOR_EPOCH2_MINTED_COUNT
    );
    let mut expected = BTreeMap::new();
    let mut unbound = 0;
    for (revision, source_id, key) in historical {
        match aliases
            .resolve(&key)
            .expect("historical key has an alias entry")
        {
            RetiredItemKey::Alias { target } => {
                assert_eq!(Some(target.clone()), tibia_item_key(source_id));
                expected.insert((revision.to_owned(), source_id.to_string()), target.clone());
            }
            RetiredItemKey::WithoutSuccessor { .. } => {
                assert_eq!(revision, CW2_B1_SOURCE_REVISION);
                unbound += 1;
            }
        }
    }
    assert_eq!(unbound, 4_590);

    let bindings = bindings_file();
    assert_eq!(bindings.len(), expected.len());
    assert_eq!(bindings.len(), 33_971);
    for row in &bindings {
        assert_eq!(row["disposition"], "EXACT");
        assert_eq!(row["identity_namespace"], "ots/item_server_id");
        assert_eq!(row["source_key"], "oteryn:source.crystalserver");
        assert_eq!(row["target"]["family"], "Item");
        assert_eq!(row["target"]["revision"], "definition-r1");
        let slot = (
            row["source_revision"]
                .as_str()
                .expect("revision")
                .to_owned(),
            row["external_id"].as_str().expect("external id").to_owned(),
        );
        let target = expected.remove(&slot).expect("bound historical row");
        assert_eq!(row["target"]["key"], target.as_str());
    }
    assert!(expected.is_empty());

    // No collision anywhere in the file: every key is unique and no retired key remains.
    let keys = bindings
        .iter()
        .map(|row| row["target"]["key"].as_str().expect("key"))
        .collect::<BTreeSet<_>>();
    assert_eq!(keys.len(), bindings.len());
    assert!(
        keys.iter()
            .all(|key| key.starts_with("oteryn:item.tibia.i"))
    );
}

#[test]
fn epoch_2_inputs_are_pinned_by_exact_bytes() {
    let mut census = DONOR_CENSUS.to_vec();
    census.push(b' ');
    assert!(matches!(
        protected_cw2_b1_donor_identity_epoch_2_import(B1_EVIDENCE, &census, ALIAS_CROSSWALK),
        Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "donor census bytes"
        ))
    ));
    let mut crosswalk = ALIAS_CROSSWALK.to_vec();
    crosswalk.push(b' ');
    assert!(matches!(
        protected_cw2_b1_donor_identity_epoch_2_import(B1_EVIDENCE, DONOR_CENSUS, &crosswalk),
        Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "donor alias crosswalk bytes"
        ))
    ));
    assert!(
        protected_cw2_b1_donor_identity_epoch_2_import(
            &B1_EVIDENCE[..B1_EVIDENCE.len() - 1],
            DONOR_CENSUS,
            ALIAS_CROSSWALK
        )
        .is_err()
    );
}

/// Re-pin a modified crosswalk so its state handling can be exercised: the modified bytes
/// and the resulting allocation are bound by digest exactly as the frozen ones are.
fn admit_modified(
    edit: impl FnOnce(&mut Vec<Value>),
    minted_count: usize,
    allocation: &[(u64, String)],
) -> Result<ProtectedCw2B1DonorIdentityEpoch2Import, ProtectedCw2B1ImportError> {
    let mut crosswalk: Value = serde_json::from_slice(ALIAS_CROSSWALK).expect("crosswalk");
    edit(crosswalk["rows"].as_array_mut().expect("rows"));
    let mut digest_input = Vec::new();
    for (source_id, key) in allocation {
        digest_input.extend_from_slice(source_id.to_string().as_bytes());
        digest_input.push(0);
        digest_input.extend_from_slice(key.as_bytes());
        digest_input.push(b'\n');
    }
    let digest = world_project_sha256(&digest_input);
    crosswalk["epoch_2"]["allocation_digest_sha256"] = Value::String(digest.clone());
    let bytes = serde_json::to_vec(&crosswalk).expect("crosswalk bytes");
    let pins = DonorEpoch2Pins {
        census_bytes: CW2_B1_DONOR_EPOCH2_CENSUS_BYTES,
        census_sha256: CW2_B1_DONOR_EPOCH2_CENSUS_SHA256,
        census_id_count: CW2_B1_DONOR_EPOCH2_CENSUS_ID_COUNT,
        crosswalk_bytes: bytes.len(),
        crosswalk_sha256: Box::leak(world_project_sha256(&bytes).into_boxed_str()),
        minted_count,
        allocation_digest_sha256: Box::leak(digest.into_boxed_str()),
    };
    admit_donor_identity_epoch_2(B1_EVIDENCE, DONOR_CENSUS, &bytes, &pins)
}

fn set_state(row: &mut Value, state: &str, target: Option<&str>) {
    row["state"] = Value::String(state.to_owned());
    if let Some(target) = target {
        row["alias_target_key"] = Value::String(target.to_owned());
    }
}

#[test]
fn only_no_match_rows_mint_and_aliases_bind_to_existing_keys() {
    let baseline = minted(&epoch2());
    // Minted ids 1 (alias), 2 (ambiguous) and 3 (conflict) mint nothing; every later rank
    // shifts down, so the key is the rank among the minted ids and not the census position.
    let expected =
        std::iter::once(baseline[0].clone())
            .chain(baseline[4..].iter().enumerate().map(|(offset, (id, _))| {
                (*id, opaque_item_key(CW2_B1_OPAQUE_ITEM_COUNT + 2 + offset))
            }))
            .collect::<Vec<_>>();
    let (alias_id, ambiguous_id, conflict_id) = (baseline[1].0, baseline[2].0, baseline[3].0);
    let imported = admit_modified(
        |rows| {
            for row in rows.iter_mut() {
                match row["source_item_id"].as_u64() {
                    Some(id) if id == alias_id => set_state(
                        row,
                        "ACCEPTED_ALIAS",
                        Some("oteryn:item.registry.i00000001"),
                    ),
                    Some(id) if id == ambiguous_id => set_state(row, "AMBIGUOUS", None),
                    Some(id) if id == conflict_id => set_state(row, "CONFLICT", None),
                    _ => {}
                }
            }
        },
        expected.len(),
        &expected,
    )
    .expect("modified crosswalk");
    assert_eq!(minted(&imported), expected);
    assert_eq!(
        imported.alias_bindings,
        vec![(alias_id, "oteryn:item.registry.i00000001".to_owned())]
    );
    let mut unbound = HELD_DONOR_IDS.to_vec();
    unbound.extend([ambiguous_id, conflict_id]);
    unbound.sort_unstable();
    assert_eq!(imported.unbound_source_item_ids, unbound);
    assert_eq!(imported.records.len(), expected.len());
    assert_eq!(
        imported.records.last().map(record_key),
        Some("oteryn:item.registry.i00038494")
    );

    // The gold coin's retired key is not an alias target; its promoted key is.
    for (target, admitted) in [
        (R7_P04_GOLD_COIN_OLD_KEY, false),
        (R7_P04_GOLD_COIN_KEY, true),
        ("oteryn:item.registry.i00038094", false),
        ("oteryn:item.registry.i99999999", false),
    ] {
        let result = admit_modified(
            |rows| set_state(&mut rows[0], "ACCEPTED_ALIAS", Some(target)),
            baseline.len() - 1,
            &baseline[1..]
                .iter()
                .enumerate()
                .map(|(offset, (id, _))| {
                    (*id, opaque_item_key(CW2_B1_OPAQUE_ITEM_COUNT + 1 + offset))
                })
                .collect::<Vec<_>>(),
        );
        assert_eq!(result.is_ok(), admitted, "{target}");
    }
}

#[test]
fn a_state_that_carries_a_target_or_is_unknown_fails_closed() {
    let baseline = minted(&epoch2());
    let shifted = baseline[1..]
        .iter()
        .enumerate()
        .map(|(offset, (id, _))| (*id, opaque_item_key(CW2_B1_OPAQUE_ITEM_COUNT + 1 + offset)))
        .collect::<Vec<_>>();
    assert!(matches!(
        admit_modified(
            |rows| set_state(
                &mut rows[0],
                "AMBIGUOUS",
                Some("oteryn:item.registry.i00000001")
            ),
            shifted.len(),
            &shifted
        ),
        Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "unbound state carries a target"
        ))
    ));
    assert!(matches!(
        admit_modified(
            |rows| set_state(&mut rows[0], "MINT_ME", None),
            shifted.len(),
            &shifted
        ),
        Err(ProtectedCw2B1ImportError::EvidenceMismatch(
            "alias crosswalk state"
        ))
    ));
    // A missing row, or rows out of census order, are not the frozen census ids.
    assert!(
        admit_modified(
            |rows| {
                rows.pop();
            },
            baseline.len(),
            &baseline
        )
        .is_err()
    );
    assert!(admit_modified(|rows| rows.swap(0, 1), baseline.len(), &baseline).is_err());
}
