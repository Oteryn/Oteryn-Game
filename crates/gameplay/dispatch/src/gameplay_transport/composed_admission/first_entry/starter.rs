use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Retryable Newhaven kit admission. The durable owner rechecks the current vocation and
    /// item fence; no cached tutorial transition or historical receipt grants current authority.
    pub(in crate::gameplay_transport) async fn grant_newhaven_starter(
        &self,
        fence: CurrentCharacterItemFence,
    ) -> std::result::Result<
        crate::durability::starter_grant::NewhavenStarterOutcome,
        crate::durability::DurabilityError,
    > {
        use crate::durability::starter_grant::NewhavenStarterOutcome;
        use oteryn_gameplay_items::starter_grant::newhaven_starter_items;
        let interpretation = self
            .root
            .read_character_root_interpretation(self.character, fence.character_id)
            .await
            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
        if interpretation.is_none_or(|value| {
            value.revisions()[3]
                != oteryn_gameplay_items::starter_grant::NEWHAVEN_STARTER_TEMPLATE_REVISION
        }) {
            return Ok(NewhavenStarterOutcome::NotApplicable);
        }
        let build = self
            .root
            .read_character_build_state(self.character, fence.character_id)
            .await
            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
        if newhaven_starter_items(build.vocation()).is_none() {
            return Ok(NewhavenStarterOutcome::NotApplicable);
        };
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(crate::durability::DurabilityError::Unavailable)?;
        let definitions =
            super::super::tutorial_vocation::newhaven_kit_definitions(native, build.vocation())
                .ok_or(crate::durability::DurabilityError::InvalidStoredState)?;
        let Some(_permit) = self.spell_lane_permit().await else {
            return Err(crate::durability::DurabilityError::Unavailable);
        };
        let outcome = self
            .root
            .grant_newhaven_starter_items(
                self.character,
                self.holder,
                &fence,
                &item_ref_admission::NativeItemWeights(native),
                build.vocation(),
                definitions,
            )
            .await?;
        if outcome == NewhavenStarterOutcome::Granted {
            self.states
                .tutorial_sessions
                .inventory_changed(fence.game_session_id);
        }
        Ok(outcome)
    }

    pub(super) async fn initialize_starter_items(
        &self,
        native: &crate::content::native_gameplay::NativeGameplayState,
        character_id: domain::CharacterId,
        fence: CurrentCharacterItemFence,
        build: &crate::durability::character_build::DurableBuildState,
        vocation: crate::spell::Vocation,
        phase: &EntryPhase,
    ) -> Result<(), FirstEntryOutcome> {
        #[cfg(test)]
        let stage = |name: &str| {
            if std::env::var_os("OTERYN_SEAM_SPELL_MANIFEST").is_some() {
                eprintln!("SEAM_EVIDENCE spell_first_entry stage={name}");
            }
        };
        phase.set("interpretation");
        let interpretation = self
            .root
            .read_character_root_interpretation(self.character, character_id)
            .await;
        let Ok(Some(interpretation)) = interpretation else {
            return Err(FirstEntryOutcome::RefusedStaleAuthority);
        };
        if interpretation.revisions()[3]
            == oteryn_gameplay_items::starter_grant::NEWHAVEN_STARTER_TEMPLATE_REVISION
        {
            // Newhaven roots get the donor's first-login backpack (send_first_items for a
            // Character without vocation: rope and shovel), then their own profession kit.
            // Never also execute a vocation's legacy send_first_items loadout.
            phase.set("starter_backpack");
            let Some(first_items) =
                crate::durability::starter_grant::StartingLoadout::resolve("none", |id| {
                    let key = format!("oteryn:item.tibia.i{id}");
                    let revision = native.item_index()?.revision(&key)?.to_owned();
                    Some(crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: key,
                        revision_ref: revision,
                    })
                })
            else {
                return Err(FirstEntryOutcome::RefusedUnavailable);
            };
            #[cfg(test)]
            let _ = FAMILIAR_LOGIN_CHECKOUT.try_with(|probe| {
                probe.backpack.set(probe.backpack.get() + 1);
            });
            let backpack_result = self
                .root
                .grant_newhaven_backpack(
                    self.character,
                    self.holder,
                    &fence,
                    &item_ref_admission::NativeItemWeights(native),
                    &first_items,
                )
                .await;
            #[cfg(test)]
            stage(&format!("newhaven_backpack:{backpack_result:?}"));
            if !matches!(
                backpack_result,
                Ok(
                    crate::durability::starter_grant::StarterGrantOutcome::Minted(_)
                        | crate::durability::starter_grant::StarterGrantOutcome::AlreadyGranted
                        | crate::durability::starter_grant::StarterGrantOutcome::Skipped
                )
            ) {
                return Err(FirstEntryOutcome::RefusedUnavailable);
            }
            phase.set("starter_kit");
            #[cfg(test)]
            let _ = FAMILIAR_LOGIN_CHECKOUT.try_with(|probe| {
                probe.kit.set(probe.kit.get() + 1);
            });
            let kit_result = if vocation != crate::spell::Vocation::None {
                Some(self.grant_newhaven_starter(fence).await)
            } else {
                None
            };
            #[cfg(test)]
            stage(&format!("newhaven_kit:{kit_result:?}"));
            // NotAdmitted here means custody has no room. Content failures are errors.
            if kit_result.is_some_and(|result| {
                !matches!(result,
                Ok(crate::durability::starter_grant::NewhavenStarterOutcome::Granted
                    | crate::durability::starter_grant::NewhavenStarterOutcome::AlreadyGranted
                    | crate::durability::starter_grant::NewhavenStarterOutcome::NotAdmitted))
            }) {
                return Err(FirstEntryOutcome::RefusedUnavailable);
            }
        } else {
            let loadout = crate::durability::starter_grant::StartingLoadout::resolve(
                build.vocation(),
                |id| {
                    let key = format!("oteryn:item.tibia.i{id}");
                    let revision = native.item_index()?.revision(&key)?.to_owned();
                    Some(crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: key,
                        revision_ref: revision,
                    })
                },
            );
            if loadout.is_none()
                && oteryn_gameplay_items::starter_grant::starting_loadout::items(build.vocation())
                    .is_some()
            {
                return Err(FirstEntryOutcome::RefusedUnavailable);
            }
            // Crystal 201189f8 data/scripts/creaturescripts/player/send_first_items.lua:107-109:
            // unlisted vocations (including promotions) receive no kit.
            phase.set("starting_loadout");
            if let Some(loadout) = loadout {
                // STARTER-1 (CP D1438): the starter grant of the Character's first entry, once per
                // Character and template; a replay writes nothing.
                if !matches!(
                    self.root
                        .grant_starting_loadout(
                            self.character,
                            self.holder,
                            &fence,
                            &item_ref_admission::NativeItemWeights(native),
                            &loadout,
                        )
                        .await,
                    Ok(
                        crate::durability::starter_grant::StarterGrantOutcome::Minted(_)
                            | crate::durability::starter_grant::StarterGrantOutcome::AlreadyGranted
                            | crate::durability::starter_grant::StarterGrantOutcome::Skipped
                    )
                ) {
                    return Err(FirstEntryOutcome::RefusedUnavailable);
                }
            }
        }
        Ok(())
    }
}
