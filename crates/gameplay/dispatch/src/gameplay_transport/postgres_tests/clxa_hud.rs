//! CLX-A: database-backed character values crossing the existing HUD query boundary.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::{
    CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
};

#[test]
fn clxa_hud_postgres_general_stats_preserve_skill_values_and_stamina_boundaries() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        run(async |admin| {
            use super::cyclopedia::support::exchange;
            reset();
            let h = Harness::create_with_admission_seed(admin, "clxa_hud", true, false).await?;
            eprintln!("POSTGRES_STARTED CLX-A general_stats_skill_and_stamina_boundaries");
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let owner = deps.owner(&h, &authority);
            deps.initialize(&owner).await;
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id).await?.0;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                &[OfferedCapability { id: CAPABILITY_CYCLOPEDIA_CHARACTER_V1, requires: &[] }],
                &[CAPABILITY_CYCLOPEDIA_CHARACTER_V1],
            ).ok_or("character capability")?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'knight',7,0,10,0,11,0,42,0,13,0,14,0,15,0,16,0,2,encode($2,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET magic_level=7,fist_level=10,club_level=11,sword_level=42,axe_level=13,distance_level=14,shielding_level=15,fishing_level=16")
                .bind(id(41).as_slice()).bind(id(30).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            for (command, minutes) in [(1, 0_u16), (2, 60), (3, 2520)] {
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_character_build_state SET stamina_minutes=$2 WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).bind(i32::from(minutes)).execute(&mut *tx).await?;
                tx.commit().await?;
                let (status, bytes) = exchange(&owner, &mut admitted, command, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, &[1, 0, 0, 1]).await?;
                assert_eq!(status, CommandStatus::Accepted);
                assert_eq!(bytes.len(), 42, "the complete native GeneralStats layout");
                assert_eq!(bytes[0], 1);
                assert_eq!(u16::from_le_bytes([bytes[13], bytes[14]]), 7);
                for (slot, level) in [10, 11, 42, 13, 14, 15, 16].into_iter().enumerate() {
                    let at = 15 + slot * 2;
                    assert_eq!(u16::from_le_bytes([bytes[at], bytes[at + 1]]), level);
                }
                assert_eq!(u16::from_le_bytes([bytes[29], bytes[30]]), minutes);
                eprintln!("CLX-A verified GeneralStats: magic=7 sword=42 stamina={minutes}");
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }).map_err(|e| e.to_string())
    })?.join().map_err(|_| "CLX-A PostgreSQL scenario panicked")??;
    Ok(())
}
