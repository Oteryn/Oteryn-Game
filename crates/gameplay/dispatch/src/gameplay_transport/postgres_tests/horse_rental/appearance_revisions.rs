//! Component proof: cosmetic definition provenance is independent of gameplay pins.
//! This finite TestContent isolates the durable owner, not production source admission.
use super::*;
use crate::domain::appearance::{AppearanceContent, AppearanceSelection, Sex};
use crate::durability::character_appearance::{
    AppearanceChangeOccurrence, AppearanceChangeOutcome, AppearanceChangeRequest,
    AppearanceValidationContext,
};
struct CosmeticContent;
impl AppearanceContent<String> for CosmeticContent {
    fn palette_size(&self) -> u16 {
        133
    }
    fn outfit_addon_count(&self, key: &String) -> Option<u8> {
        (key == "fixture:outfit").then_some(0)
    }
    fn outfit_is_starter(&self, key: &String, sex: Sex) -> bool {
        key == "fixture:outfit" && sex == Sex::Male
    }
    fn addon_is_starter(&self, _: &String, _: Sex, _: u8) -> bool {
        false
    }
    fn mount_exists(&self, _: &String) -> bool {
        false
    }
}
#[test]
fn srv7_appearance_cosmetic_provenance_differs_from_gameplay_root_pin() -> TestResult {
    run(async |admin| {
        let h = Harness::create_appearance_with_admission_seed(
            admin,
            "srv7_appearance_revisions",
            true,
            false,
        )
        .await?;
        prepare_current_session(&h).await?;
        let mut gameplay_fence = super::super::fixture::fence(1)?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(gameplay_fence.game_session_id)
                .await?;
        gameplay_fence.character_lease_generation = current.current_character_lease().generation();
        assert_eq!(
            gameplay_fence.character_lease_generation, 2,
            "real fresh admission advanced its Character lease"
        );

        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let request = AppearanceChangeRequest {
            occurrence: AppearanceChangeOccurrence::from_bytes(id(181)).map_err(debug)?,
            before: None,
            after: AppearanceSelection {
                outfit_key: "fixture:outfit".into(),
                colours: [1, 2, 3, 4],
                addons: 0,
                mount_key: None,
            },
            content_revision: "cosmetics-only-r1".into(),
            transport_binding: None,
        };
        let outcome = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                gameplay_fence,
                request.clone(),
                AppearanceValidationContext {
                    sex: Sex::Male,
                    content: CosmeticContent,
                    content_revision: "cosmetics-only-r1".into(),
                },
            )
            .await;
        assert!(
            matches!(outcome, Ok(AppearanceChangeOutcome::Committed(_))),
            "separate cosmetic provenance must preserve the current gameplay fence: {outcome:?}"
        );
        let revisions:(String,String,String)=sqlx::query_as("SELECT receipt.content_revision,receipt.cosmetics_revision,selected.content_revision FROM game_character_appearance_receipts receipt JOIN game_character_appearance selected USING(character_id) WHERE receipt.appearance_occurrence_id=encode($1,'hex')::uuid")
            .bind(request.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            revisions,
            (
                "content-1".into(),
                "cosmetics-only-r1".into(),
                "cosmetics-only-r1".into()
            ),
            "global gameplay pin and cosmetic predecessor remain independently stamped"
        );
        let replay = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                gameplay_fence,
                request.clone(),
                AppearanceValidationContext {
                    sex: Sex::Male,
                    content: CosmeticContent,
                    content_revision: "cosmetics-only-r1".into(),
                },
            )
            .await?;
        assert!(matches!(
            replay,
            AppearanceChangeOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_appearance_receipts")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        let mut next_fence = gameplay_fence;
        next_fence.expected_character_revision =
            crate::domain::CharacterRevision::new(2).map_err(debug)?;
        let all_gameplay_pins:bool=sqlx::query_scalar("SELECT (r.profile_revision,r.ruleset_revision,r.content_revision,r.simulation_revision,r.evidence_revision,r.declaration_revision,r.policy_revision,r.reward_revision) IS NOT DISTINCT FROM (p.profile_revision,p.ruleset_revision,p.content_revision,p.simulation_revision,p.evidence_revision,p.declaration_revision,p.policy_revision,p.reward_revision) FROM game_character_appearance_receipts r JOIN game_character_progression_state p USING(character_id) WHERE r.appearance_occurrence_id=encode($1,'hex')::uuid")
            .bind(request.occurrence.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert!(
            all_gameplay_pins,
            "cosmetic provenance must not replace any gameplay receipt pin"
        );
        let mut wrong_document = request.clone();
        wrong_document.occurrence =
            AppearanceChangeOccurrence::from_bytes(id(182)).map_err(debug)?;
        wrong_document.before = h
            .root
            .read_character_appearance(&authority, next_fence.character_id)
            .await?;
        wrong_document.after.colours[0] = 2;
        let refused = h
            .root
            .commit_character_appearance(
                &authority,
                &h.node,
                next_fence,
                wrong_document,
                AppearanceValidationContext {
                    sex: Sex::Male,
                    content: CosmeticContent,
                    content_revision: "other-cosmetics-document".into(),
                },
            )
            .await;
        assert!(
            refused.is_err(),
            "independent appearance content document must still match the request"
        );
        assert_eq!(sqlx::query_scalar::<_, String>("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(gameplay_fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?, "2");
        for (occurrence, outfit, mount) in [
            (183, "fixture:unknown-outfit", None),
            (184, "fixture:outfit", Some("fixture:unknown-mount")),
        ] {
            let before = h
                .root
                .read_character_appearance(&authority, next_fence.character_id)
                .await?;
            let mut after = request.after.clone();
            after.outfit_key = outfit.into();
            after.mount_key = mount.map(str::to_owned);
            let invalid = AppearanceChangeRequest {
                occurrence: AppearanceChangeOccurrence::from_bytes(id(occurrence))
                    .map_err(debug)?,
                before,
                after,
                content_revision: "cosmetics-only-r1".into(),
                transport_binding: None,
            };
            let result = h
                .root
                .commit_character_appearance(
                    &authority,
                    &h.node,
                    next_fence,
                    invalid,
                    AppearanceValidationContext {
                        sex: Sex::Male,
                        content: CosmeticContent,
                        content_revision: "cosmetics-only-r1".into(),
                    },
                )
                .await;
            assert!(
                result.is_err(),
                "unknown cosmetic identity must not acquire permission from cosmetic provenance"
            );
        }
        assert_eq!(sqlx::query_scalar::<_, String>("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(gameplay_fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?, "2");
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_appearance_receipts")
                .fetch_one(&h.pool)
                .await?,
            1
        );
        Ok(())
    })
}
