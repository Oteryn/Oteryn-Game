//! SRVGAP-B9: native workshop grades qualify, revoke and re-earn a conditional title.
use super::fixture::{Harness, id, run};
use super::{TestResult, debug, prepare_current_session};
use oteryn_gameplay_progression::atelier::{AtelierState, FragmentKind};
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use oteryn_protocol_oteryn::character_info::CharacterTitle;

#[test]
fn reaudit_title_predicates_expired_award_can_be_earned_again() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                let started = std::time::Instant::now();
                for explicit in [false, true] {
                    let h = Harness::create_with_admission_seed(
                        admin.clone(),
                        "reaudit_title_expiry",
                        false,
                        false,
                    )
                    .await?;
                    prepare_current_session(&h).await?;
                    let current =
                        crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                            h.root.clone(),
                        )
                        .current_session_at(
                            crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
                        )
                        .await?
                        .0;
                    let fence =
                        crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    // Exercise the expiry storage seam with an eligible title; Drome has no native rota.
                    let title = CharacterTitle::Aeternal;
                    h.root
                        .unlock_character_title(&authority, &h.node, fence, title, 100, Some(200))
                        .await
                        .map_err(debug)?;
                    h.root
                        .select_character_title(&authority, &h.node, fence, Some(title))
                        .await
                        .map_err(debug)?;
                    {
                        use crate::gameplay_transport::capabilities::SelectedCapabilities;
                        use crate::gameplay_transport::character_info::CharacterInfoReply;
                        use oteryn_protocol_oteryn::character_info::CharacterInfoOperation;
                        let deps = super::Dependencies::new()?;
                        let owner = deps.owner(&h, &authority);
                        let mut admitted = deps.admitted;
                        admitted.item_fence = Some(fence);
                        let offers = crate::gameplay_transport::character_info::offered(&[], true);
                        admitted.continuity.selected_capabilities =
                            SelectedCapabilities::select(&offers, &[97])
                                .ok_or("title capability")?;
                        assert_eq!(
                            owner
                                .character_info_command(
                                    admitted,
                                    CharacterInfoOperation::Select(Some(title))
                                )
                                .await,
                            CharacterInfoReply::Rejected,
                            "a client cannot select an expired award"
                        );
                    }
                    let now = 1_800_000_000_000;
                    if explicit {
                        assert!(
                            h.root
                                .qualify_and_unlock_character_title(
                                    &authority, &h.node, fence, title, now, None, None
                                )
                                .await
                                .map_err(debug)?
                        );
                    } else {
                        let granted = h
                            .root
                            .refresh_character_titles(&authority, &h.node, fence, now, None, None)
                            .await
                            .map_err(debug)?;
                        assert!(granted.contains(&title));
                    }
                    let read = h
                        .root
                        .read_and_requalify_character_titles(
                            &authority,
                            &h.node,
                            fence,
                            fence.character_id,
                            now,
                            None,
                            None,
                        )
                        .await
                        .map_err(debug)?;
                    assert!(
                        read.earned.contains(&(title, None)),
                        "an expired row must not suppress a newly eligible award"
                    );
                    assert_eq!(
                        read.selected, None,
                        "expiry clears the old selection even if re-earned"
                    );
                    assert_eq!(read.revision, 2);
                    drop(authority);
                    drop(seal);
                    h.cleanup().await?;
                }
                eprintln!(
                    "POSTGRES_COMPLETED reaudit_title_predicates_expiry {:?}",
                    started.elapsed()
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "title expiry scenario panicked")??;
    Ok(())
}

#[test]
fn reaudit_title_predicates_native_weapon_mastery() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| mastery_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "title mastery scenario panicked")??;
    Ok(())
}

fn mastery_scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "reaudit_title_mastery", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(
                    crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
                )
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        seed_mastery(&h).await?;
        assert!(
            h.root
                .qualify_and_unlock_character_title(
                    &authority,
                    &h.node,
                    fence,
                    CharacterTitle::AdeptArmorer,
                    1_800_000_000_000,
                    None,
                    None
                )
                .await
                .map_err(debug)?,
            "native weapon Mastery must qualify without combat skill 100"
        );
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_COMPLETED reaudit_title_predicates_mastery {:?}",
            started.elapsed()
        );
        Ok(())
    })
}

/// Seed the current native Mastery fact while retaining a complete proficiency receipt chain.
pub(super) async fn seed_mastery(h: &Harness) -> TestResult {
    use crate::durability::character_proficiency::ProficiencyDefinitions;
    use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
    let data = oteryn_gameplay_content::content::import_committed_weapon_proficiency_data()?;
    let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("proficiency catalogue")?;
    let definition = data
        .bindings()
        .keys()
        .filter_map(|key| catalogue.resolve_weapon(key))
        .find(|definition| definition.shape.track_shape().mastery_level() >= 3)
        .ok_or("masterable weapon")?;
    let shape = definition.shape.track_shape();
    let selections: Vec<Option<i16>> = vec![None; usize::from(shape.perk_levels())];
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_character_proficiency_receipts(proficiency_occurrence_id,character_id,original_character_revision,committed_character_revision,cause,command_binding,policy_digest,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) SELECT encode($2,'hex')::uuid,character_id,character_revision,character_revision+1,'training',$3,$4,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,1 FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(id(190).as_slice()).bind([1_u8].as_slice()).bind(catalogue.digest().as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_proficiency_receipt_lines(proficiency_occurrence_id,character_id,committed_character_revision,cause,item_key,definition_key_before,definition_revision_before,definition_key_after,definition_revision_after,progress_before,progress_after,selections_before,selections_after) SELECT proficiency_occurrence_id,character_id,committed_character_revision,cause,$2,$3,$4,$3,$4,0,$5,$6,$6 FROM game_character_proficiency_receipts WHERE proficiency_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(190).as_slice()).bind(&definition.canonical_item_key).bind(&definition.definition_key).bind(&definition.definition_revision)
        .bind(i64::from(shape.final_progress())).bind(&selections).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_proficiency(character_id,item_key,definition_key,definition_revision,progress,selections,committed_character_revision,last_proficiency_occurrence_id) SELECT character_id,item_key,definition_key_after,definition_revision_after,progress_after,selections_after,committed_character_revision,proficiency_occurrence_id FROM game_character_proficiency_receipt_lines WHERE proficiency_occurrence_id=encode($1,'hex')::uuid")
        .bind(id(190).as_slice()).execute(&mut *tx).await?;
    for sql in [
        "UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid",
        "UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid",
    ] {
        sqlx::query(sql)
            .bind(id(41).as_slice())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[test]
fn reaudit_title_predicates_native_grades_unlock_select_revoke() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "title predicates scenario panicked")??;
    Ok(())
}

// A retained workshop snapshot with its same-transaction receipt and Character successor.
// No resources are awarded or spent by this fixture.
async fn workshop(h: &Harness, state: &mut AtelierState) -> TestResult {
    state.revision += 1;
    let json = serde_json::to_value(&*state)?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_character_atelier_receipts(character_id,occurrence,request_sha256,before_revision,after_revision,outcome_json,kind,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) SELECT character_id,gen_random_uuid(),$2,$3,$4,$5,$6,character_revision,character_revision+1,level,level,total_experience,total_experience,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind([9_u8; 32].as_slice())
        .bind(i64::try_from(state.revision - 1)?).bind(i64::try_from(state.revision)?)
        .bind(&json).bind(if state.revision == 1 { "GEM_INIT" } else { "SET_LOCK" })
        .execute(&mut *tx).await?;
    for sql in [
        "UPDATE game_character_roots SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid",
        "UPDATE game_character_progression_state SET character_revision=character_revision+1 WHERE character_id=encode($1,'hex')::uuid",
    ] {
        sqlx::query(sql)
            .bind(id(41).as_slice())
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_character_atelier_state(character_id,atelier_revision,gold,fragments_basic,fragments_supreme,state_json) VALUES(encode($1,'hex')::uuid,$2,0,0,0,$3) ON CONFLICT(character_id) DO UPDATE SET atelier_revision=excluded.atelier_revision,state_json=excluded.state_json")
        .bind(id(41).as_slice()).bind(i64::try_from(state.revision)?).bind(json)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "reaudit_title_predicates", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(
                    crate::foundation::GameSessionId::decode(&id(50)).map_err(debug)?,
                )
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let data = WheelGemData::embedded()?;
        let mut state = AtelierState {
            vocation: "sorcerer".into(),
            grade_ruleset_revision: Some(data.revision().into()),
            ..Default::default()
        };
        for seed in 100_u8..108 {
            state.gems.insert(
                u128::from(seed),
                oteryn_gameplay_progression::atelier::Gem {
                    id: u128::from(seed),
                    quality: oteryn_gameplay_progression::atelier::GemQuality::Lesser,
                    mods: vec![3],
                    locked: false,
                    vessel: false,
                    slot: None,
                    ruleset_revision: data.revision().into(),
                },
            );
        }
        for (kind, field) in [
            (FragmentKind::Basic, "basic_mods_position_1"),
            (FragmentKind::Basic, "basic_mods_position_2"),
            (FragmentKind::Supreme, "supreme_mods"),
        ] {
            for value in data.gems()["vocations"]["sorcerer"][field]
                .as_array()
                .ok_or("mod list")?
            {
                state
                    .grades
                    .insert((kind, u16::try_from(value.as_u64().ok_or("mod id")?)?), 2);
            }
        }
        workshop(&h, &mut state).await?;
        let now = 1_800_000_000_000;
        let title = CharacterTitle::AdeptEnhancer;
        assert!(
            h.root
                .qualify_and_unlock_character_title(
                    &authority, &h.node, fence, title, now, None, None
                )
                .await
                .map_err(debug)?,
            "all native Grade III mods must qualify Adept Enhancer"
        );
        h.root
            .select_character_title(&authority, &h.node, fence, Some(title))
            .await
            .map_err(debug)?;
        let read = h
            .root
            .read_and_requalify_character_titles(
                &authority,
                &h.node,
                fence,
                fence.character_id,
                now,
                None,
                None,
            )
            .await
            .map_err(debug)?;
        assert_eq!(read.selected, Some(title));
        // A missing canonical mod cannot be replaced by an unknown mod with a high grade.
        state.grades.remove(&(FragmentKind::Supreme, 0));
        state.grades.insert((FragmentKind::Supreme, u16::MAX), 3);
        workshop(&h, &mut state).await?;
        let read = h
            .root
            .read_and_requalify_character_titles(
                &authority,
                &h.node,
                fence,
                fence.character_id,
                now,
                None,
                None,
            )
            .await
            .map_err(debug)?;
        assert_eq!(read.selected, None);
        assert_eq!(read.revision, 2);
        assert!(!read.earned.iter().any(|(earned, _)| *earned == title));
        state.grades.insert((FragmentKind::Supreme, 0), 3);
        workshop(&h, &mut state).await?;
        let granted = h
            .root
            .refresh_character_titles(&authority, &h.node, fence, now, None, None)
            .await
            .map_err(debug)?;
        assert!(
            granted.contains(&title),
            "Grade IV also satisfies Grade III"
        );
        state.grade_ruleset_revision = Some("obsolete-grades".into());
        workshop(&h, &mut state).await?;
        let read = h
            .root
            .read_and_requalify_character_titles(
                &authority,
                &h.node,
                fence,
                fence.character_id,
                now,
                None,
                None,
            )
            .await
            .map_err(debug)?;
        assert!(!read.earned.iter().any(|(earned, _)| *earned == title));
        assert!(
            h.root
                .qualify_and_unlock_character_title(
                    &authority,
                    &h.node,
                    fence,
                    CharacterTitle::ReigningDromeChampion,
                    now,
                    None,
                    None
                )
                .await
                .is_err(),
            "no rota owner means no Drome qualification"
        );
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!(
            "POSTGRES_COMPLETED reaudit_title_predicates {:?}",
            started.elapsed()
        );
        Ok(())
    })
}
