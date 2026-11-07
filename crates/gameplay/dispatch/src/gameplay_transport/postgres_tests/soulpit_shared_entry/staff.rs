//! Crystal 201189f soulpit_fight.lua:37-77 exempts actual GMs from level8.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::spell_owner_commit::SpellLanePermit;

#[test]
fn soulpit_shared_entry_uses_current_trusted_gm_group_not_forged_hint_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("soulpit-gm-level", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "soulpit_gm_level", false, false)
                .await?;
            super::super::reset();
            let evidence = prepare_source(&h).await?;
            let deps = Dependencies::new()?;
            let admitted = admit_inner(&h, &deps, &evidence, true).await?;
            let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(admitted.destination.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                level, 1,
                "trusted GM entry must preserve the real low level"
            );
            h.cleanup().await
        })
    })
}

pub(super) struct GmProof<'a, 'f, 's> {
    pub h: &'a Harness,
    pub authority: &'a ReconciledCharacterAuthority<'f, 's>,
    pub source: CurrentCharacterItemFence,
    pub group: &'a ActivityEnterRequest,
    pub entry: &'a SoulpitEntryRequest,
    pub owner: &'a ActivityHandoffOwner,
    pub permit: &'a SpellLanePermit,
    pub native: &'a NativeGameplayState,
}

impl GmProof<'_, '_, '_> {
    pub async fn verify(&self) -> TestResult<super::super::staff::Signing> {
        self.initialize_level_one().await?;
        self.refuse(self.entry.clone(), "ordinary level1").await?;
        let mut forged = self.entry.clone();
        forged.participants[0].game_master = true;
        self.refuse(forged, "forged GM hint at level1").await?;
        let signing = super::super::staff::Signing::new()?;
        signing.bootstrap(self.h).await?;
        let now = clock().ok_or("staff fixture clock")?.0;
        let character = self
            .source
            .character_id
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let request = signing.request(
            2,
            &signing.config.newest_activation_sha256,
            200,
            now,
            crate::durability::staff::rules::Action::Inspect {
                character_id: character,
            },
        );
        let canonical = serde_json::to_vec(&serde_json::to_value(request)?)?;
        let result = self
            .h
            .root
            .submit_staff_tools(
                self.authority,
                &self.h.node,
                self.source,
                signing.sign(&canonical, false)?,
                false,
                clock,
            )
            .await?
            .ok_or("signed GM principal binding")?;
        assert_eq!(
            oteryn_protocol_oteryn::staff::decode_result(&result)?.code,
            oteryn_protocol_oteryn::staff::StaffCode::Ok
        );
        let group:i16=sqlx::query_scalar("SELECT group_id FROM game_staff_character_groups WHERE character_id=encode($1,'hex')::uuid")
            .bind(self.source.character_id.as_bytes().as_slice()).fetch_one(&self.h.pool).await?;
        assert_eq!(
            group, 6,
            "the signed current ADMIN roster must bind its native GM group"
        );
        // A retained group row is historical data once the configured activation
        // is revoked. It must not grant a fresh rule exemption on its own.
        self.h.root.configure_staff(None).await.map_err(debug)?;
        self.refuse(self.entry.clone(), "revoked configured staff activation")
            .await?;
        self.h
            .root
            .configure_staff(Some(signing.config.clone()))
            .await
            .map_err(debug)?;
        assert!(
            !self.entry.participants[0].game_master,
            "positive case must not rely on the caller hint"
        );
        Ok(signing)
    }

    async fn refuse(&self, entry: SoulpitEntryRequest, label: &str) -> TestResult {
        assert_eq!(
            self.owner
                .enter_soulpit(
                    self.authority,
                    &self.h.node,
                    self.permit,
                    self.group.clone(),
                    &entry,
                    self.native
                )
                .await
                .map_err(|error| format!("{label}: {error:?}"))?,
            ActivityEnterOutcome::Refused(
                crate::durability::activity_handoff::ActivityRefusal::AuthorityRejected
            ),
            "{label}"
        );
        assert_eq!(
            entry_counts(self.h).await?,
            (2, 0, 0, 0),
            "{label} must not burn or create a handoff"
        );
        Ok(())
    }

    async fn initialize_level_one(&self) -> TestResult {
        use crate::domain::progression::{
            FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
        };
        use crate::durability::character_progression::{
            ProgressionInitializationOutcome, ProgressionInitializationRequest,
        };
        use oteryn_simulation_determinism::{ExactI64, RoundingMode};
        let context = ProgressionRevisionContext {
            profile: "profile-1".into(),
            ruleset: "ruleset-1".into(),
            content: "content-1".into(),
            simulation: "simulation-1".into(),
            evidence: "evidence-1".into(),
            declaration: "declaration-1".into(),
        };
        let item = self.source;
        let fence = CurrentCharacterGameplayFence {
            character_id: item.character_id,
            game_session_id: item.game_session_id,
            connection_generation: item.connection_generation,
            character_lease_generation: item.character_lease_generation,
            runtime_scope: item.runtime_scope,
            scope_ownership_generation: item.scope_ownership_generation,
            expected_character_revision: CharacterRevision::new(1).map_err(debug)?,
        };
        let initialized = self
            .h
            .root
            .initialize_character_progression(
                self.authority,
                &self.h.node,
                fence,
                ProgressionInitializationRequest {
                    context: context.clone(),
                    policy_revision: "policy-1".into(),
                    reward_revision: "reward-1".into(),
                    policy: FiniteProgressionPolicy {
                        context,
                        policy_revision: "policy-1".into(),
                        reward_revision: "reward-1".into(),
                        death_policy_revision: "death-1".into(),
                        declared_difference_revision: "declaration-1".into(),
                        thresholds: [
                            LevelThreshold {
                                level: 1,
                                minimum_experience: ExactI64::new(0),
                            },
                            LevelThreshold {
                                level: 2,
                                minimum_experience: ExactI64::new(100),
                            },
                        ],
                        terminal_exclusive_experience: ExactI64::new(200),
                        death_loss_numerator: 1,
                        death_loss_denominator: 10,
                        death_loss_rounding: RoundingMode::Floor,
                    },
                },
            )
            .await
            .map_err(debug)?;
        assert!(matches!(
            initialized,
            ProgressionInitializationOutcome::Initialized(_)
        ));
        Ok(())
    }
}

fn clock() -> Option<(u64, u64)> {
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis(),
    )
    .ok()?;
    Some((now, now))
}
