//! Real APP-1 writer, authored content decoder and installed PostgreSQL schema.
use super::*;
use crate::bestiary_postgres_harness::fence;
use crate::domain::appearance::AppearanceSelection;
use crate::durability::character_appearance::{
    AppearanceChangeOccurrence, AppearanceChangeOutcome, AppearanceChangeRequest,
    AppearanceValidationContext, AppearanceWriteError,
};
use oteryn_gameplay_content::content::native_gameplay::test_support::cosmetics_fixture::{
    cosmetics_test_packet, decode_cosmetics,
};

#[test]
fn appearance_earned_authoring_drives_fenced_writer_without_revision_equality() -> TestResult {
    configured_admin().ok_or("APP-3 requires the routed PostgreSQL admin environment")?;
    run("appearance_earned_authoring", async |h| {
        let mut connection = sqlx::PgConnection::connect(&h.database.url).await?;
        seed_unlocks(&mut connection).await?;
        let mut packet = cosmetics_test_packet();
        let authoring = &mut packet["authoring"];
        authoring["document"]["content_revision"] = "content-1".into();
        let mut earned = authoring["document"]["outfits"][0].clone();
        earned["key"] = "oteryn:outfit.earned".into();
        earned["starter_female"] = false.into();
        earned["starter_male"] = false.into();
        earned["starter_addons_female"] = 0.into();
        earned["starter_addons_male"] = 0.into();
        earned["premium"] = true.into();
        authoring["document"]["outfits"]
            .as_array_mut()
            .ok_or("outfits")?
            .push(earned);
        let promotions = authoring["promotions"].as_array_mut().ok_or("promotions")?;
        let new_promotions: Vec<_> = promotions
            .iter()
            .filter(|p| {
                p["path"]
                    .as_str()
                    .is_some_and(|p| p.starts_with("/outfits/0/"))
            })
            .map(|p| {
                let mut p = p.clone();
                p["path"] = p["path"]
                    .as_str()
                    .unwrap_or_default()
                    .replacen("/outfits/0/", "/outfits/1/", 1)
                    .into();
                p
            })
            .collect();
        promotions.extend(new_promotions);
        let mappings = serde_json::json!([
            {"grant_key":"earned", "target":{"kind":"Outfit","outfit_key":"oteryn:outfit.earned"},"compatible_provenance":["content-1"]},
            {"grant_key":"fixture-addon-one", "target":{"kind":"Addon","outfit_key":"oteryn:outfit.earned","addon":1},"compatible_provenance":["content-1"]},
            {"grant_key":"horse", "target":{"kind":"Mount","mount_key":authoring["document"]["mounts"][0]["key"]},"compatible_provenance":["incompatible"]}
        ]);
        authoring["document"]["grant_mappings"] = mappings;
        let mut promotion = authoring["promotions"][0].clone();
        promotion["path"] = "/grant_mappings".into();
        authoring["promotions"]
            .as_array_mut()
            .ok_or("promotions")?
            .push(promotion);
        // Rebind all affected authored values through the existing per-field promotion gate.
        let document = authoring["document"].clone();
        for promotion in authoring["promotions"].as_array_mut().ok_or("promotions")? {
            promotion["value"] = document
                .pointer(promotion["path"].as_str().ok_or("path")?)
                .ok_or("value")?
                .clone();
        }
        let content = decode_cosmetics(&serde_json::to_vec(&packet)?).map_err(debug)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = AppearanceChangeRequest {
            transport_binding: None,
            occurrence: AppearanceChangeOccurrence::from_bytes(id(87)).map_err(debug)?,
            before: None,
            after: AppearanceSelection {
                outfit_key: "oteryn:outfit.earned".into(),
                colours: [0, 1, 2, 132],
                addons: 1,
                mount_key: Some(
                    document["mounts"][0]["key"]
                        .as_str()
                        .ok_or("mount")?
                        .to_owned(),
                ),
            },
            content_revision: "content-1".into(),
        };
        // The same input with substituted sex refuses before the write.
        let rejected = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                request.clone(),
                AppearanceValidationContext {
                    sex: Sex::Female,
                    content: content.clone(),
                    content_revision: "content-1".into(),
                },
            )
            .await;
        assert!(matches!(rejected, Err(AppearanceWriteError::InvalidInput)));
        assert_eq!(h.root_revision().await?, "1");
        let committed = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                fence(1)?,
                request.clone(),
                AppearanceValidationContext {
                    sex: Sex::Male,
                    content,
                    content_revision: "content-1".into(),
                },
            )
            .await
            .map_err(debug)?;
        assert!(matches!(committed, AppearanceChangeOutcome::Committed(_)));
        assert_eq!(h.root_revision().await?, "2");
        assert!(
            sqlx::query("UPDATE game_character_roots SET sex=1")
                .execute(&h.pool)
                .await
                .is_err()
        );
        h.root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        Ok(())
    })
}
