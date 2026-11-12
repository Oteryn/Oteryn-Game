//! CL7 P08: read-only, pinned catalogue and live session progress. No custody or XP writes.
use super::{ComposedFreshAdmission, item_ref_admission::pinned_index};
use crate::content::ReferenceItemField;
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::weapon_proficiency::{
    PROFICIENCY_PAGE_SIZE, ProficiencyPage, ProficiencyRow,
};

macro_rules! authority_methods {
    () => {
        fn weapon_proficiency_browse(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _session: $crate::foundation::GameSessionId,
            _offset: u32,
        ) -> impl std::future::Future<
            Output = Option<oteryn_protocol_oteryn::weapon_proficiency::ProficiencyPage>,
        > {
            async { None }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn weapon_proficiency_browse(
            &self,
            actor: $crate::foundation::ExactActorRef,
            session: $crate::foundation::GameSessionId,
            offset: u32,
        ) -> Option<oteryn_protocol_oteryn::weapon_proficiency::ProficiencyPage> {
            $crate::gameplay_transport::weapon_proficiency_browse::execute(
                self, actor, session, offset,
            )
            .await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    offset: u32,
) -> Option<ProficiencyPage> {
    let catalogue = owner.proficiency_catalogue.as_ref()?;
    // The same lock fences both the content pin and the session that owns the XP tracks.
    let runtime = owner.runtime.lock().await;
    runtime.owner_fence().ok()?;
    runtime.player_control_facts(actor, session).ok()?;
    let index = pinned_index(&runtime, owner.active_generation)?;
    let states = owner.states.proficiencies.lock().ok()?;
    let progress = states.get(&session);
    let mut rows = Vec::with_capacity(PROFICIENCY_PAGE_SIZE);
    let mut total = 0u32;
    for key in catalogue.weapon_keys() {
        let Some(revision) = index.revision(key) else {
            continue;
        };
        let definition = index.definition_ref("Item", key, revision)?;
        if total >= offset && rows.len() < PROFICIENCY_PAGE_SIZE {
            let track = progress.and_then(|state| state.track(key));
            let empty = catalogue.empty_track(key)?;
            let track = track.unwrap_or(&empty);
            rows.push(ProficiencyRow {
                definition,
                name: owner
                    .active_generation?
                    .native_gameplay()?
                    .item_policy(key, revision)
                    .and_then(|policy| match &policy.record().semantics.presentation {
                        ReferenceItemField::Known(presentation) => match &presentation.name {
                            ReferenceItemField::Known(name) => Some(name.clone()),
                            _ => None,
                        },
                        _ => None,
                    })
                    .unwrap_or_else(|| format!("Weapon {definition}")),
                experience: track.progress,
                level: track.shape.track_shape().level(track.progress),
                selections: track.selections.clone(),
            });
        }
        total = total.checked_add(1)?;
    }
    if offset > total {
        return None;
    }
    Some(ProficiencyPage {
        offset,
        total,
        rows,
    })
}
