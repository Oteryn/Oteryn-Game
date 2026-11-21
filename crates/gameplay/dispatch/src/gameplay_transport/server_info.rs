//! Additional presentation facts use existing fenced owners; no client data enters gameplay.
use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::server_info::{CAPABILITY_SERVER_INFO_V1, Query, Reply, SpellInfo};
pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    base.iter()
        .copied()
        .filter(|e| available || e.id != CAPABILITY_SERVER_INFO_V1)
        .collect()
}
/// The response handler for observed quest copies; also used by the end-to-end caller test.
pub(super) fn quests(source: &super::quest_log::QuestLogSource) -> Option<Vec<u8>> {
    Reply::Quests(source.titles()).encode()
}
fn spells(book: &crate::spell::SpellBook, ids: &[u32]) -> Option<Vec<u8>> {
    let groups: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tools/content-schema/spell-authoring/cooldown-groups.json"
    ))
    .ok()?;
    let groups = groups.get("groups")?.as_array()?;
    let mut rows = Vec::new();
    for id in ids {
        let index = usize::try_from(id.checked_sub(1)?).ok()?;
        if book.inactive_aliases.contains(&index) {
            return None;
        }
        let spell = book.spells.get(index)?;
        let mut durations = Vec::new();
        for group in &spell.groups {
            let ordinal = groups
                .iter()
                .position(|g| g.get("key").and_then(|k| k.as_str()) == Some(&group.key))?;
            durations.push((
                u32::try_from(ordinal + 1).ok()?,
                group.cooldown_micros / 1000,
            ));
        }
        durations.sort_unstable();
        rows.push(SpellInfo {
            cast_index: *id,
            description: spell
                .authored
                .as_ref()
                .and_then(|p| p.header.library_text.clone()),
            cooldown_ms: spell.cooldown_micros / 1000,
            group_cooldowns_ms: durations,
        });
    }
    Reply::Spells(rows).encode()
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn server_info_wire(
        &self,
        admitted: AdmittedSession,
        payload: &[u8],
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_SERVER_INFO_V1)
            || admitted.controller.is_none()
        {
            return None;
        }
        let query = Query::decode(payload)?;
        let actor = admitted.runtime_actor?;
        match query {
            Query::Quests => {
                let super::quest_log::QuestLogObservation::Changed(source) = self
                    .quest_log_observation(actor, admitted.game_session_id, None)
                    .await
                else {
                    return None;
                };
                quests(&source)
            }
            Query::Spells(ids) => {
                // Reuse the exact actor/generation join that owns spell eligibility and learning.
                let entries = self
                    .actor_spell_catalog_with_visibility(actor, admitted.game_session_id, true)
                    .await?;
                if !ids
                    .iter()
                    .all(|id| entries.iter().any(|e| e.cast_index == *id))
                {
                    return None;
                }
                spells(self.spells, &ids)
            }
        }
    }
}
macro_rules! authority_methods {
    () => {
        fn server_info_available(&self) -> bool {
            false
        }
        fn server_info_query(
            &self,
            _admitted: $crate::gameplay_transport::AdmittedSession,
            _payload: &[u8],
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        fn server_info_available(&self) -> bool {
            self.active_generation
                .and_then(|a| a.native_gameplay())
                .is_some()
        }
        async fn server_info_query(
            &self,
            admitted: connection::AdmittedSession,
            payload: &[u8],
        ) -> Option<Vec<u8>> {
            self.server_info_wire(admitted, payload).await
        }
    };
}
pub(super) use composed_methods;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn server_info_spell_handler_wire_client_state_and_tooltip()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| scenario().map_err(|e| e.to_string()))?
            .join()
            .map_err(|_| "spell scenario thread")?
            .map_err(Into::into)
    }
    fn scenario() -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_client_ui::{server_info, spell_catalog};
        let mut id = [42; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let world = crate::foundation::WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
        let (controller, _pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let book = controller
            .active()
            .and_then(|a| a.native_gameplay())
            .ok_or("native content")?
            .spell_book();
        let state = crate::spell::cast::PlayerSpellState::new(
            crate::gameplay_transport::actor_spell::tests::FACTS,
            0,
            0,
        )
        .ok_or("state")?;
        let catalog = super::super::spell_catalog::catalog_with_visibility(book, &state, 0, true)
            .ok_or("catalog")?;
        let spell = catalog
            .iter()
            .find(|e| {
                book.spells[e.cast_index as usize - 1]
                    .authored
                    .as_ref()
                    .is_some_and(|p| p.header.library_text.is_some())
            })
            .ok_or("library description")?;
        let query = Query::Spells(vec![spell.cast_index]);
        let payload = spells(book, &[spell.cast_index]).ok_or("spell handler")?;
        let mut info = server_info::Panel {
            selected: true,
            ..Default::default()
        };
        let _ = server_info::request(&mut info, query.clone()).ok_or("request")?;
        let reply = server_info::apply_result(
            &mut info,
            &query,
            &oteryn_session::GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: oteryn_session::CommandStatus::Accepted,
                payload,
            },
        )
        .ok_or("reply")?;
        let mut panel = spell_catalog::Panel {
            selected: true,
            ..Default::default()
        };
        spell_catalog::protocol::apply(
            &mut panel,
            oteryn_session::spell_catalog::SpellCatalog {
                revision: 1,
                entries: vec![spell.clone()],
            },
        );
        server_info::protocol::observe(reply, &mut Default::default(), &mut panel);
        let metadata = panel.info.get(&spell.cast_index).ok_or("metadata")?;
        let source = &book.spells[spell.cast_index as usize - 1];
        assert_eq!(
            metadata.description,
            source
                .authored
                .as_ref()
                .and_then(|p| p.header.library_text.clone())
        );
        assert_eq!(metadata.cooldown_ms, source.cooldown_micros / 1000);
        for ((_, ms), group) in metadata.group_cooldowns_ms.iter().zip(&source.groups) {
            assert_eq!(*ms, group.cooldown_micros / 1000);
        }
        let text = spell_catalog::detail_text(&panel, &Default::default());
        assert!(text.contains(metadata.description.as_deref().ok_or("description")?));
        assert!(text.contains(&format!("Base spell cooldown: {} ms", metadata.cooldown_ms)));
        assert!(spells(book, &[u32::MAX]).is_none());
        Ok(())
    }
}
