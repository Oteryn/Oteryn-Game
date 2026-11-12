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
        self.initialize_level_one()
            .await
            .map_err(|error| format!("GM level1 initialization: {error:?}"))?;
        self.refuse(self.entry.clone(), "ordinary level1").await?;
        let mut forged = self.entry.clone();
        forged.participants[0].game_master = true;
        self.refuse(forged, "forged GM hint at level1").await?;
        let signing = super::super::staff::Signing::new()
            .map_err(|error| format!("GM signing fixture: {error:?}"))?;
        signing
            .bootstrap(self.h)
            .await
            .map_err(|error| format!("GM trusted activation bootstrap: {error:?}"))?;
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
            .await
            .map_err(|error| format!("GM signed principal binding: {error:?}"))?
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
        let trusted = self
            .h
            .root
            .staff_live_status(self.authority, &self.h.node, self.source)
            .await
            .map_err(|error| format!("GM trusted current-session status: {error:?}"))?;
        assert_eq!(trusted.group, 6);
        assert!(!trusted.banned && !trusted.kicked);
        // Staff configuration is immutable for one root. Match the existing
        // staff restart fixture: a restarted owner without the configured
        // activation must refuse, despite the same retained current group row.
        {
            use sqlx::ConnectOptions;
            let options = self
                .h
                .pool
                .connect_options()
                .as_ref()
                .clone()
                .options([("role", "oteryn_game_runtime")]);
            let runtime_url = options.to_url_lossy();
            let revoked =
                crate::durability::DurabilityRoot::connect_test_runtime(runtime_url.as_str())
                    .map_err(|error| format!("GM revoked-owner root: {error:?}"))?;
            assert!(
                revoked
                    .maintain_ready_once()
                    .await
                    .map_err(|error| format!("GM revoked-owner ready: {error:?}"))?
            );
            revoked
                .configure_staff(None)
                .await
                .map_err(|error| format!("GM revoked-owner configuration: {error:?}"))?;
            let seal = self.h.recovery.seal_current().map_err(debug)?;
            let authority = revoked
                .open_character_authority(&seal)
                .await
                .map_err(|error| format!("GM revoked-owner character authority: {error:?}"))?;
            let owner = ActivityHandoffOwner::new(revoked.clone());
            // This ordinary durable status reader independently proves this
            // exact session, node, lease and scope before the exemption probe.
            let current = revoked
                .staff_live_status(&authority, &self.h.node, self.source)
                .await
                .map_err(|error| format!("GM revoked-owner current-session status: {error:?}"))?;
            assert_eq!(current.group, 0);
            assert!(!current.banned && !current.kicked);
            self.refuse_with(
                &owner,
                &authority,
                self.entry.clone(),
                "revoked configured staff activation",
            )
            .await?;
        }
        assert!(
            !self.entry.participants[0].game_master,
            "positive case must not rely on the caller hint"
        );
        Ok(signing)
    }

    async fn refuse(&self, entry: SoulpitEntryRequest, label: &str) -> TestResult {
        self.refuse_with(self.owner, self.authority, entry, label)
            .await
    }

    async fn refuse_with(
        &self,
        owner: &ActivityHandoffOwner,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        entry: SoulpitEntryRequest,
        label: &str,
    ) -> TestResult {
        assert_eq!(
            owner
                .enter_soulpit(
                    authority,
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
        let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(self.source.character_id.as_bytes().as_slice()).fetch_one(&self.h.pool).await?;
        assert_eq!(level, 1);
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
