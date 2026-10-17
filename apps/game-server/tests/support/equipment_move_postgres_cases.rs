// EQUIP-WIRE-1 (ITEM-MOVE-WIRE-1 §4, ITEM-MOVE-2a slice): command 9's equip and unequip through
// the equipment owner on PostgreSQL 17.6. One fenced transaction moves one item between a direct
// main-backpack entry and a free slot, writes one equipment receipt and never advances
// CharacterRevision; every refusal rolls back and writes nothing. The replay-first read finds
// the receipt by its CommandRef. The policies are a test-only qualified profile with the same
// shape the active NativeGameplayState lookup produces. A resent CommandRef replays its receipt
// under a per-CommandRef lock that the replay read also takes, and a timed item never enters a
// slot while the Game node hosts no timed lane. The new occupied-slot case below adds
// the two-item/four-location whole-item swap; its additive migration is CP-gated and NOT_RUN.
use crate::corpse_transfer_postgres_cases::{equip_backpack, locations, typed};
use crate::domain::CharacterId;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_equipment::{
    EquipmentIntent, EquipmentMove, EquipmentMoveContent, EquipmentMoveRefusal,
    EquipmentMoveReplay, apply_equipment_in_transaction, assert_equipment_authority_in_transaction,
    initialize_equipment_in_transaction,
};
use crate::durability::equipment_policy_abi::{
    EquipmentClaims, EquipmentPolicyLookup, QualifiedEquipmentPolicy, policy_registration,
};
use crate::durability::item_transfer::{ItemDefinitionFacts, ItemStackClass, ItemTransferOutcome};
use crate::item_transfer_postgres_cases::{
    BACKPACK, CHANNEL, CHARACTER, Harness, SESSION, TestResult, WORLD, command, configured_admin,
    debug, fence, id, runtime, to_backpack,
};

const DIGEST: [u8; 32] = [1; 32];
const SWORD: &str = "fixture:equip.sword";
const ARMOR: &str = "fixture:equip.armor";
const HEAVY: &str = "fixture:equip.heavy-armor";
const BAG: &str = "fixture:equip.bag";
const TIMED: &str = "fixture:equip.timed-helmet";
/// Durable slot codes (ITEM-MOVE-WIRE-1 §2).
const TORSO: u8 = 2;
const HEAD: u8 = 1;
const WEAPON: u8 = 5;
const SLOTS: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 10];

/// The test profile: a sword for the weapon slot, armor (one needing level 999) and a bag for
/// the torso slot, and the main backpack of capacity 3 that is in no equipment slot.
struct Profile;

impl policy_registration::Registered for Profile {}

fn policy(slot: Option<u8>, level: u16, capacity: Option<u16>) -> Option<QualifiedEquipmentPolicy> {
    let claims = SLOTS
        .into_iter()
        .map(|candidate| {
            let claim = if Some(candidate) == slot {
                Ok(EquipmentClaims {
                    slots: vec![candidate],
                    groups: Vec::new(),
                    level,
                    vocations: Vec::new(),
                })
            } else {
                Err("equipment slot incompatible with Content")
            };
            (candidate, claim)
        })
        .collect();
    QualifiedEquipmentPolicy::from_qualified_content(DIGEST, claims, capacity)
}

impl EquipmentPolicyLookup for Profile {
    fn source_digest(&self) -> [u8; 32] {
        DIGEST
    }

    fn equipment_policy(&self, key: &str, revision: &str) -> Option<QualifiedEquipmentPolicy> {
        if revision != "definition-r1" {
            return None;
        }
        match key {
            SWORD => policy(Some(WEAPON), 0, None),
            ARMOR => policy(Some(TORSO), 0, None),
            HEAVY => policy(Some(TORSO), 999, None),
            BAG => policy(Some(TORSO), 0, Some(8)),
            TIMED => policy(Some(HEAD), 0, None),
            BACKPACK => policy(None, 0, Some(3)),
            _ => None,
        }
    }
}

impl EquipmentMoveContent for Profile {
    fn equipment_stack_class(
        &self,
        key: &str,
        revision: &str,
    ) -> crate::durability::item_transfer::ItemStackClass {
        use crate::durability::item_transfer::ItemStackClass;
        if revision == "definition-r1" && [ARMOR, HEAVY, SWORD, TIMED].contains(&key) {
            ItemStackClass::NonStackable
        } else {
            ItemStackClass::Unknown
        }
    }
    fn is_plain_item(&self, production_key: &str, _revision_ref: &str) -> bool {
        ![BAG, BACKPACK].contains(&production_key)
    }

    fn is_timed_item(&self, production_key: &str, _revision_ref: &str) -> bool {
        production_key == TIMED
    }
}

fn plain(key: &str) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        weight: Some(1),
        definition: typed("Item", key, "definition-r1"),
        stack: ItemStackClass::NonStackable,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

const fn equip(item: [u8; 16], slot: u8) -> EquipmentMove {
    EquipmentMove {
        item,
        from_slot: None,
        to_slot: Some(slot),
    }
}

const fn unequip(item: [u8; 16], slot: u8) -> EquipmentMove {
    EquipmentMove {
        item,
        from_slot: Some(slot),
        to_slot: None,
    }
}

/// The active Content of Channel 43 and the initialized empty equipment owner, as the fresh
/// admission leaves them.
async fn activate_and_initialize(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(WORLD).as_slice()).bind(id(CHANNEL).as_slice())
        .bind(DIGEST.as_slice()).bind([2_u8; 32].as_slice()).bind([3_u8; 32].as_slice())
        .execute(&harness.pool).await?;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
        .execute(&mut *tx)
        .await?;
    let equipment = assert_equipment_authority_in_transaction(
        &mut tx,
        &harness.root,
        authority,
        &harness.node,
        &fence()?,
        command(90)?,
        DIGEST,
    )
    .await
    .map_err(debug)?;
    initialize_equipment_in_transaction(&mut tx, &equipment)
        .await
        .map_err(debug)?;
    tx.commit().await?;
    Ok(())
}

async fn character_revision(harness: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots \
         WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_one(&harness.pool)
    .await?)
}

/// Row counts of every relation an equipment move could write.
async fn footprint(harness: &Harness) -> TestResult<Vec<i64>> {
    let mut counts = harness.footprint().await?;
    for relation in [
        "game_character_equipment_slots",
        "game_character_equipment_receipts",
        "game_character_equipment_audit_outbox",
    ] {
        counts.push(harness.count(relation).await?);
    }
    let revision: String = sqlx::query_scalar(
        "SELECT revision::text FROM game_character_equipment_state \
         WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(CHARACTER).as_slice())
    .fetch_one(&harness.pool)
    .await?;
    counts.push(revision.parse()?);
    Ok(counts)
}

async fn worn(
    harness: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<Vec<(u8, [u8; 16])>> {
    let read = harness
        .root
        .read_movement_equipment(authority, &harness.node, &fence()?, DIGEST)
        .await
        .map_err(debug)?;
    Ok(read
        .equipment()
        .items
        .iter()
        .map(|item| (item.slot, item.item_instance_id))
        .collect())
}

#[test]
fn an_item_is_equipped_and_unequipped_once_and_every_refusal_writes_nothing() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "equipmove").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        equip_backpack(&harness, &authority, fence()?, SESSION).await?;
        let mut items = Vec::new();
        for (n, key) in [SWORD, ARMOR, HEAVY, BAG].into_iter().enumerate() {
            let item = harness.mint(&authority, key, 1).await?;
            match harness
                .transfer(
                    &authority,
                    fence()?,
                    to_backpack(command(2 + n as u64)?, item, plain(key)),
                )
                .await
                .map_err(debug)?
            {
                ItemTransferOutcome::Committed(_) => items.push(item),
                other => return Err(format!("expected a commit, got {other:?}").into()),
            }
        }
        let [sword, armor, heavy, bag] = items[..] else {
            return Err("four backpack entries".into());
        };
        activate_and_initialize(&harness, &authority).await?;
        let revision = character_revision(&harness).await?;
        let moved = |n: u64, request: EquipmentMove| {
            let harness = &harness;
            let authority = &authority;
            async move {
                harness
                    .root
                    .commit_equipment_move(
                        authority,
                        &harness.node,
                        &fence()?,
                        command(n)?,
                        &Profile,
                        request,
                    )
                    .await
                    .map_err(|error| -> Box<dyn std::error::Error> { debug(error).into() })
            }
        };

        // Equip: the sword leaves the backpack for the weapon slot, with one receipt.
        let before = footprint(&harness).await?;
        let receipt = moved(10, equip(sword, WEAPON))
            .await?
            .map_err(|refusal| format!("expected a commit, got {refusal:?}"))?;
        assert!(!receipt.replayed);
        assert_eq!(receipt.revision_after, 2);
        assert_eq!(worn(&harness, &authority).await?, [(WEAPON, sword)]);
        assert_eq!(locations(&harness, sword).await?, 0);
        assert_ne!(footprint(&harness).await?, before);
        assert_eq!(character_revision(&harness).await?, revision);

        // Replay first: the CommandRef names the committed move of this Character only.
        let character = CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?;
        let other = CharacterId::from_bytes(id(99)).map_err(debug)?;
        for (who, n, expected) in [
            (character, 10, EquipmentMoveReplay::Moved),
            (character, 11, EquipmentMoveReplay::Absent),
            (other, 10, EquipmentMoveReplay::Conflicting),
        ] {
            assert_eq!(
                harness
                    .root
                    .read_equipment_move_receipt(&authority, who, command(n)?)
                    .await
                    .map_err(debug)?,
                expected
            );
        }

        // Every refusal rolls back: nothing is written.
        let committed = footprint(&harness).await?;
        for (n, request, refusal) in [
            (11, equip(armor, WEAPON), EquipmentMoveRefusal::SlotMismatch),
            (12, equip(armor, HEAD), EquipmentMoveRefusal::SlotMismatch),
            (13, equip(bag, TORSO), EquipmentMoveRefusal::SlotMismatch),
            (
                14,
                equip(heavy, TORSO),
                EquipmentMoveRefusal::RequirementNotMet,
            ),
            (15, unequip(sword, TORSO), EquipmentMoveRefusal::Stale),
            (16, unequip(armor, TORSO), EquipmentMoveRefusal::Stale),
        ] {
            assert_eq!(moved(n, request).await?.err(), Some(refusal), "{n}");
            assert_eq!(footprint(&harness).await?, committed, "{n}");
        }
        assert_eq!(character_revision(&harness).await?, revision);

        // Unequip back into the backpack, then the backpack (capacity 3) is full.
        moved(18, equip(armor, TORSO))
            .await?
            .map_err(|refusal| format!("expected a commit, got {refusal:?}"))?;
        moved(19, unequip(sword, WEAPON))
            .await?
            .map_err(|refusal| format!("expected a commit, got {refusal:?}"))?;
        assert_eq!(worn(&harness, &authority).await?, [(TORSO, armor)]);
        assert_eq!(locations(&harness, sword).await?, 1);
        let full = footprint(&harness).await?;
        assert_eq!(
            moved(20, unequip(armor, TORSO)).await?.err(),
            Some(EquipmentMoveRefusal::NoRoom)
        );
        assert_eq!(footprint(&harness).await?, full);
        assert_eq!(character_revision(&harness).await?, revision);

        // A resent CommandRef replays its own receipt; another move under it writes nothing.
        let replayed = moved(19, unequip(sword, WEAPON))
            .await?
            .map_err(|refusal| format!("expected a replay, got {refusal:?}"))?;
        assert!(replayed.replayed);
        assert_eq!(
            moved(19, equip(sword, WEAPON)).await?.err(),
            Some(EquipmentMoveRefusal::Rejected)
        );
        assert_eq!(footprint(&harness).await?, full);

        // A timed item never enters a slot: nothing is written.
        let timed = harness.mint(&authority, TIMED, 1).await?;
        match harness
            .transfer(
                &authority,
                fence()?,
                to_backpack(command(21)?, timed, plain(TIMED)),
            )
            .await
            .map_err(debug)?
        {
            ItemTransferOutcome::Committed(_) => {}
            other => return Err(format!("expected a commit, got {other:?}").into()),
        }
        let stocked = footprint(&harness).await?;
        assert_eq!(
            moved(22, equip(timed, HEAD)).await?.err(),
            Some(EquipmentMoveRefusal::TimedNotHosted)
        );
        assert_eq!(footprint(&harness).await?, stocked);

        // The replay read waits for an attempt of the same CommandRef that holds its lock.
        let mut holder = harness.pool.begin().await?;
        let mut key = id(SESSION).to_vec();
        key.extend_from_slice(&23_u64.to_be_bytes());
        sqlx::query(
            "SELECT pg_advisory_xact_lock(hashtextextended(\
             'oteryn:equipment-move:' || encode($1, 'hex'), 0))",
        )
        .bind(key.as_slice())
        .execute(&mut *holder)
        .await?;
        let released = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let release = tokio::spawn({
            let released = released.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                released.store(true, std::sync::atomic::Ordering::SeqCst);
                holder.rollback().await
            }
        });
        let read = harness
            .root
            .read_equipment_move_receipt(&authority, character, command(23)?)
            .await
            .map_err(debug)?;
        let read = Ok::<_, Box<dyn std::error::Error>>((
            read,
            released.load(std::sync::atomic::Ordering::SeqCst),
        ));
        release.await??;
        assert_eq!(read?, (EquipmentMoveReplay::Absent, true));

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

// Explicit synthetic test profile: two different torso definitions, and an independent
// hand resource conflict. No deployed Content/stack semantics are inferred from these facts.
struct SwapProfile;
impl policy_registration::Registered for SwapProfile {}
impl EquipmentPolicyLookup for SwapProfile {
    fn source_digest(&self) -> [u8; 32] {
        DIGEST
    }
    fn equipment_policy(&self, key: &str, revision: &str) -> Option<QualifiedEquipmentPolicy> {
        if revision != "definition-r1" {
            return None;
        }
        if key != HEAVY && key != SWORD {
            return Profile.equipment_policy(key, revision);
        }
        let target = if key == HEAVY { TORSO } else { WEAPON };
        let claims = SLOTS
            .into_iter()
            .map(|slot| {
                (
                    slot,
                    if slot == target {
                        Ok(EquipmentClaims {
                            slots: vec![slot],
                            groups: vec!["fixture:shared-hand".to_owned()],
                            level: 0,
                            vocations: vec![],
                        })
                    } else {
                        Err("equipment slot incompatible with Content")
                    },
                )
            })
            .collect();
        QualifiedEquipmentPolicy::from_qualified_content(DIGEST, claims, None)
    }
}
impl EquipmentMoveContent for SwapProfile {
    fn equipment_stack_class(
        &self,
        key: &str,
        revision: &str,
    ) -> crate::durability::item_transfer::ItemStackClass {
        if key == HEAVY {
            crate::durability::item_transfer::ItemStackClass::Unknown
        } else {
            Profile.equipment_stack_class(key, revision)
        }
    }
    fn is_plain_item(&self, key: &str, revision: &str) -> bool {
        Profile.is_plain_item(key, revision)
    }
    fn is_timed_item(&self, key: &str, revision: &str) -> bool {
        Profile.is_timed_item(key, revision)
    }
}

// Same exact test profile, changing only availability of the admitted stack-class fact.
struct KnownNonStackSwapProfile;
impl policy_registration::Registered for KnownNonStackSwapProfile {}
impl EquipmentPolicyLookup for KnownNonStackSwapProfile {
    fn source_digest(&self) -> [u8; 32] {
        DIGEST
    }
    fn equipment_policy(&self, key: &str, revision: &str) -> Option<QualifiedEquipmentPolicy> {
        SwapProfile.equipment_policy(key, revision)
    }
}
impl EquipmentMoveContent for KnownNonStackSwapProfile {
    fn is_plain_item(&self, key: &str, revision: &str) -> bool {
        SwapProfile.is_plain_item(key, revision)
    }
    fn is_timed_item(&self, key: &str, revision: &str) -> bool {
        SwapProfile.is_timed_item(key, revision)
    }
    fn equipment_stack_class(
        &self,
        key: &str,
        revision: &str,
    ) -> crate::durability::item_transfer::ItemStackClass {
        Profile.equipment_stack_class(key, revision)
    }
}

// NOT_RUN until CP assigns/installs the additive swap migration through the native
// migration path. This case never installs external SQL or bypasses a persisted guard.
#[test]
fn occupied_slot_swap_is_atomic_at_full_backpack_and_replay_bound() -> TestResult {
    use sqlx::Row;
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "equipswap").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness.root.open_character_authority(&seal).await.map_err(debug)?;
        equip_backpack(&harness, &authority, fence()?, SESSION).await?;
        let mut items=Vec::new();
        for (n,key) in [ARMOR,HEAVY,SWORD].into_iter().enumerate() {
            let item=harness.mint(&authority,key,1).await?;
            match harness.transfer(&authority,fence()?,to_backpack(command(2+n as u64)?,item,plain(key))).await.map_err(debug)? {
                ItemTransferOutcome::Committed(_) => items.push(item),
                other => return Err(format!("expected bootstrap transfer, got {other:?}").into()),
            }
        }
        let [armor,heavy,sword]=items[..] else {return Err("three items".into());};
        activate_and_initialize(&harness,&authority).await?;
        let revision=character_revision(&harness).await?;
        let moved=|n,request| {let harness=&harness;let authority=&authority;async move {
            harness.root.commit_equipment_move(authority,&harness.node,&fence()?,command(n)?,&SwapProfile,request).await
                .map_err(|error| -> Box<dyn std::error::Error> {debug(error).into()})
        }};
        moved(10,equip(armor,TORSO)).await?.map_err(|e|format!("equip armor: {e:?}"))?;
        moved(11,equip(sword,WEAPON)).await?.map_err(|e|format!("equip sword: {e:?}"))?;
        let before=footprint(&harness).await?;
        // Only the target occupant is excluded. The independent hand claim must still refuse.
        assert_eq!(moved(12,equip(heavy,TORSO)).await?.err(),Some(EquipmentMoveRefusal::SlotMismatch));
        assert_eq!(footprint(&harness).await?,before);
        moved(13,unequip(sword,WEAPON)).await?.map_err(|e|format!("unequip sword: {e:?}"))?;
        let second_heavy=harness.mint(&authority,HEAVY,1).await?;
        assert!(matches!(harness.transfer(&authority,fence()?,to_backpack(command(6)?,second_heavy,plain(HEAVY))).await.map_err(debug)?,ItemTransferOutcome::Committed(_)));
        let entries:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(entries,3,"full accepted test-profile backpack");
        let source_ordinal:String=sqlx::query_scalar("SELECT placement_ordinal::text FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(heavy.as_slice()).fetch_one(&harness.pool).await?;
        let before_items=sqlx::query("SELECT item_instance_id::text,state_revision::text,quantity FROM game_item_instances WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ORDER BY item_instance_id")
            .bind(heavy.as_slice()).bind(armor.as_slice()).fetch_all(&harness.pool).await?;
        let committed_before=footprint(&harness).await?;
        let expected_snapshot=harness.root.read_movement_equipment(&authority,&harness.node,&fence()?,DIGEST).await.map_err(debug)?.equipment().clone();
        let mut stale = fence()?;
        stale.connection_generation = crate::foundation::ConnectionGeneration::new(stale.connection_generation.get()+1).map_err(debug)?;
        assert_eq!(harness.root.commit_equipment_move(&authority,&harness.node,&stale,command(30)?,&SwapProfile,equip(heavy,TORSO)).await.map_err(debug)?.err(),Some(EquipmentMoveRefusal::Rejected));
        assert_eq!(footprint(&harness).await?,committed_before);
        let receipt=moved(14,equip(heavy,TORSO)).await?.map_err(|e|format!("swap: {e:?}"))?;
        assert!(!receipt.replayed);
        assert_eq!(worn(&harness,&authority).await?,[(TORSO,heavy)]);
        let proof=sqlx::query("SELECT r.item_instance_id=encode($2,'hex')::uuid AS incoming,r.swap_item_instance_id=encode($3,'hex')::uuid AS displaced,r.backpack_ordinal::text,r.swap_backpack_ordinal::text,(SELECT count(*) FROM game_item_container_entries WHERE character_id=r.character_id) AS entries,(SELECT count(*) FROM game_character_equipment_audit_outbox WHERE transaction_id=r.transaction_id) AS audits FROM game_character_equipment_receipts r WHERE r.transaction_id=encode($1,'hex')::uuid")
            .bind(receipt.transaction_id.as_slice()).bind(heavy.as_slice()).bind(armor.as_slice()).fetch_one(&harness.pool).await?;
        assert!(proof.try_get::<bool,_>("incoming")? && proof.try_get::<bool,_>("displaced")?);
        assert_eq!(proof.try_get::<String,_>("backpack_ordinal")?,source_ordinal);
        assert_ne!(proof.try_get::<String,_>("swap_backpack_ordinal")?,source_ordinal,"new immutable entry");
        assert_eq!(proof.try_get::<i64,_>("entries")?,entries);
        assert_eq!(proof.try_get::<i64,_>("audits")?,1);
        let payload:Vec<u8>=sqlx::query_scalar("SELECT envelope FROM game_character_equipment_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid")
            .bind(receipt.transaction_id.as_slice()).fetch_one(&harness.pool).await?;
        let event:serde_json::Value=serde_json::from_slice(&payload)?;
        assert_eq!(event["locations"].as_array().map(Vec::len),Some(4));
        assert_eq!(event["items"].as_array().map(Vec::len),Some(2));
        assert_eq!(event["locations"][0]["parent"],event["locations"][3]["parent"]);
        assert_eq!(event["locations"][0]["ordinal"],serde_json::json!(source_ordinal.parse::<u64>()?));
        assert_eq!(event["locations"][3]["ordinal"],serde_json::json!(proof.try_get::<String,_>("swap_backpack_ordinal")?.parse::<u64>()?));

        for old in before_items {
            let identity:String=old.try_get("item_instance_id")?;
            let old_revision=old.try_get::<String,_>("state_revision")?.parse::<u64>()?;
            let current:String=sqlx::query_scalar("SELECT state_revision::text FROM game_item_instances WHERE item_instance_id=$1::text::uuid")
                .bind(identity).fetch_one(&harness.pool).await?;
            assert_eq!(current.parse::<u64>()?,old_revision+1);
            let quantity:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=$1::text::uuid")
                .bind(old.try_get::<String,_>("item_instance_id")?).fetch_one(&harness.pool).await?;
            assert_eq!(quantity,old.try_get::<i64,_>("quantity")?);
        }
        assert_eq!(character_revision(&harness).await?,revision);
        let committed=footprint(&harness).await?;
        assert_ne!(committed,committed_before);
        let replay=moved(14,equip(heavy,TORSO)).await?.map_err(|e|format!("replay: {e:?}"))?;
        assert!(replay.replayed);
        assert_eq!(replay.transaction_id,receipt.transaction_id);
        assert_eq!(replay.event_id,receipt.event_id);
        // The lower-level owner's immutable expected binding also replays after the
        // old occupant has left the slot; no old placement is reconstructed as current.
        let mut tx=harness.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        let current=assert_equipment_authority_in_transaction(&mut tx,&harness.root,&authority,&harness.node,&fence()?,command(14)?,DIGEST).await.map_err(debug)?;
        let replay=apply_equipment_in_transaction(&mut tx,&current,&expected_snapshot,&SwapProfile,&EquipmentIntent::Move{item:heavy,from_slot:None,to_slot:Some(TORSO)}).await.map_err(debug)?;
        assert!(replay.replayed);assert_eq!(replay.transaction_id,receipt.transaction_id);
        tx.rollback().await?;

        assert_eq!(moved(14,equip(armor,TORSO)).await?.err(),Some(EquipmentMoveRefusal::Rejected));
        assert_eq!(footprint(&harness).await?,committed);
        // Change only the definition equality invariant: no stack class is guessed.
        assert_eq!(moved(15,equip(second_heavy,TORSO)).await?.err(),Some(EquipmentMoveRefusal::Rejected));
        assert_eq!(footprint(&harness).await?,committed);
        let same=harness.root.commit_equipment_move(&authority,&harness.node,&fence()?,command(16)?,&KnownNonStackSwapProfile,equip(second_heavy,TORSO)).await.map_err(debug)?.map_err(|e|format!("known nonstack swap: {e:?}"))?;
        assert!(!same.replayed);
        assert_eq!(worn(&harness,&authority).await?,[(TORSO,second_heavy)]);
        let entries_after:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(CHARACTER).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(entries_after,entries);
        assert_eq!(character_revision(&harness).await?,revision);
        drop(authority);drop(seal);harness.cleanup().await
    })
}
