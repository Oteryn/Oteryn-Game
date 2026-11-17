//! T087: real modifier capture, durable reload, protocol caller and current fences.
#[path = "cyclopedia_offence/setup.rs"]
mod setup;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::character_authority::CharacterAuthorityError;
use crate::durability::cyclopedia_character::combat::CombatSnapshot;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::combat::{CombatStat as Stat, CombatStats};
use oteryn_protocol_oteryn::cyclopedia_character::offence::{
    Effect, Page, Query, Snapshot, SourceKind,
};
use oteryn_protocol_oteryn::cyclopedia_character::{
    CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, CharacterTab,
};
use sqlx::Row;

#[test]
fn t087_offence_augments_postgres_real_capture_privacy_replay_and_fences() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "P12c combat scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async |admin| {
        reset();
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            super::item_spell_augments::activate(deps.admitted.world_id, false)?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native gameplay")?;
        let h = Harness::create_with_admission_seed(admin, "t087_offence", true, false).await?;
        eprintln!(
            "POSTGRES_STARTED t087_offence_augments_postgres_real_capture_privacy_replay_and_fences"
        );
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let fence = setup::configure(&h, &mut deps, pin).await?;
        setup::seed_equipment(&h).await?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        let effects = owner
            .read_admission_effects(deps.admitted.game_session_id)
            .await
            .map_err(|_| "read effects")?
            .ok_or("effects")?;
        {
            let runtime = deps.runtime.lock().await;
            owner
                .install_admission_suppressions(
                    &runtime,
                    deps.actor,
                    deps.admitted.game_session_id,
                    Some(effects),
                )
                .await
                .ok_or("install equipment")?;
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let tab = COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, tab, &[8, 0, 0, 1])
                .await?
                .0,
            CommandStatus::Rejected
        );
        assert_eq!(
            row_count(&h).await?,
            0,
            "unselected capability cannot populate statistics"
        );
        let offers = [
            OfferedCapability {
                id: 6,
                requires: &[],
            },
            OfferedCapability {
                id: 17,
                requires: &[6],
            },
            OfferedCapability {
                id: CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
                requires: &[],
            },
        ];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[6, 17, CAPABILITY_CYCLOPEDIA_CHARACTER_V1])
                .ok_or("capabilities")?;
        let query = Query::default().encode().map_err(debug)?;
        let (status, bytes) = exchange(&owner, &mut admitted, 2, tab, &query).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let first = Page::decode(&bytes).map_err(debug)?;
        assert_eq!(first.character, id(41));
        assert_eq!(first.stats.tab(), CharacterTab::Offence);
        assert!(first.stats.get(Stat::CriticalChance).is_some());
        let facts = first.facts.as_ref().ok_or("captured modifiers")?;
        assert_ne!(facts.known_sources & SourceKind::Equipment.bit(), 0);
        let native_facts = &native
            .item_policy("oteryn:item.tibia.i2854", "definition-r1")
            .ok_or("backpack policy")?
            .record()
            .attributes
            .spell_augments;
        assert_eq!(facts.rows.len(), native_facts.len());
        assert_eq!(facts.sources.len(), 1);
        assert_eq!(facts.sources[0].slot, 3);
        assert!(
            facts
                .rows
                .iter()
                .any(|r| r.effect == Effect::Cooldown && r.value == 100000)
        );
        assert!(
            facts
                .rows
                .iter()
                .any(|r| r.effect == Effect::BaseDamage && r.value == 10000)
        );
        let (status, replay) = exchange(&owner, &mut admitted, 2, tab, &query).await?;
        assert_eq!(status, CommandStatus::DuplicateReplay);
        assert_eq!(replay, bytes);
        // Reload the exact bytes and revisions, independent of the runtime observation.
        let row=sqlx::query("SELECT observed_owner_micros::text,offence,defence,misc,offence_augments FROM game_character_cyclopedia_combat WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let saved =
            Snapshot::decode(&row.try_get::<Vec<u8>, _>("offence_augments")?).map_err(debug)?;
        assert_eq!(saved.rows.len(), native_facts.len());
        setup::legacy_binding(&h, &authority, fence).await?;
        let equipment = crate::gameplay_transport::equipment_move::read_current_equipment(
            &owner,
            admitted.game_session_id,
        )
        .await
        .ok_or("equipment")?;
        let mut snapshot = CombatSnapshot {
            observed_owner_micros: row
                .try_get::<String, _>("observed_owner_micros")?
                .parse::<u64>()?
                + 1,
            equipment,
            offence: CombatStats::decode(&row.try_get::<Vec<u8>, _>("offence")?).map_err(debug)?,
            defence: CombatStats::decode(&row.try_get::<Vec<u8>, _>("defence")?).map_err(debug)?,
            misc: CombatStats::decode(&row.try_get::<Vec<u8>, _>("misc")?).map_err(debug)?,
        };
        h.root
            .record_cyclopedia_combat(&authority, &h.node, fence, snapshot.clone())
            .await
            .map_err(debug)?;
        let target = crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?;
        let legacy = h
            .root
            .read_cyclopedia_offence(&authority, &h.node, fence, target, Query::default())
            .await
            .map_err(debug)?;
        assert!(
            legacy.facts.is_none(),
            "old observations are unavailable, never known empty"
        );
        snapshot.observed_owner_micros += 1;
        let empty = Snapshot {
            known_sources: 7,
            ..Default::default()
        };
        let written = h
            .root
            .record_cyclopedia_combat_with_augments(
                &authority,
                &h.node,
                fence,
                snapshot.clone(),
                Some(empty.clone()),
            )
            .await
            .map_err(debug)?;
        assert!(!written.replayed);
        let replay = h
            .root
            .record_cyclopedia_combat_with_augments(
                &authority,
                &h.node,
                fence,
                snapshot.clone(),
                Some(empty),
            )
            .await
            .map_err(debug)?;
        assert!(replay.replayed);
        assert_eq!(written.sample_revision, replay.sample_revision);
        assert!(matches!(
            h.root
                .record_cyclopedia_combat_with_augments(
                    &authority,
                    &h.node,
                    fence,
                    snapshot.clone(),
                    Some(saved)
                )
                .await,
            Err(CharacterAuthorityError::Conflict)
        ));
        let mut stale = fence;
        stale.character_lease_generation += 1;
        assert!(
            h.root
                .record_cyclopedia_combat_with_augments(
                    &authority,
                    &h.node,
                    stale,
                    snapshot.clone(),
                    Some(Snapshot::default())
                )
                .await
                .is_err()
        );
        assert!(
            h.root
                .read_cyclopedia_offence(&authority, &h.node, stale, target, Query::default())
                .await
                .is_err()
        );
        let known = h
            .root
            .read_cyclopedia_offence(&authority, &h.node, fence, target, Query::default())
            .await
            .map_err(debug)?;
        assert!(
            known
                .facts
                .as_ref()
                .is_some_and(|f| f.rows.is_empty() && f.known_sources == 7)
        );
        // An offline foreign sample remains subject to the current viewer's consent.
        setup::seed_foreign(&h).await?;
        let foreign = Query {
            target: Some(id(60)),
            ..Default::default()
        }
        .encode()
        .map_err(debug)?;
        assert!(
            owner
                .cyclopedia_character_wire(admitted, 0, tab, &foreign)
                .await
                .is_err()
        );
        sqlx::query("INSERT INTO game_vip_lists(account_id,world_id) SELECT account_id,world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) SELECT account_id,world_id,encode($2,'hex')::uuid,1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) SELECT account_id,world_id,character_id,encode($2,'hex')::uuid,2,now()+interval '1 day' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        let permitted = owner
            .cyclopedia_character_wire(admitted, 0, tab, &foreign)
            .await
            .map_err(|_| "foreign read")?;
        let permitted = Page::decode(&permitted).map_err(debug)?;
        assert_eq!(permitted.character, id(60));
        assert!(permitted.facts.as_ref().is_some_and(|f| f.rows.is_empty()));
        sqlx::query(
            "UPDATE game_vip_consents SET consent=1 WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(60).as_slice())
        .execute(&h.pool)
        .await?;
        assert!(
            owner
                .cyclopedia_character_wire(admitted, 0, tab, &foreign)
                .await
                .is_err()
        );
        eprintln!(
            "POSTGRES_EXECUTED t087_offence_augments_postgres_real_capture_privacy_replay_and_fences"
        );
        h.pool.close().await;
        Ok(())
    })
}
async fn row_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_cyclopedia_combat")
            .fetch_one(&h.pool)
            .await?,
    )
}
