use super::fixture::{Harness, id};

use super::TestResult;

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
