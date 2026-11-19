use super::fixture::{Harness, debug, id};

use super::TestResult;

use crate::durability::admission_authority_guards::AdmissionGuardStore;

use crate::durability::fresh_admission::FreshAdmissionStore;

use crate::foundation::admission_authority_publication::AdmissionAuthorityGuardStateV1;

use crate::foundation::fresh_admission_durability::FreshAdmissionDurableOutcomeV1;

use crate::foundation::{
    AuthenticatedTransportRefV1, CharacterId, GameSessionId, RuntimeScopeRefV1,
};

use crate::gameplay_transport::item_fence_of;

pub(super) async fn fresh_after_release(
    h: &Harness,
    character: crate::domain::CharacterId,
    origin: RuntimeScopeRefV1,
) -> TestResult<crate::foundation::GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut source = super::fresh_fixture::fresh::Source::new(now)?;
    let keys = source
        .rows
        .iter()
        .map(|row| row.key.clone())
        .collect::<Vec<_>>();
    source.rows = AdmissionGuardStore::from_root(h.root.clone())
        .load(&keys)
        .await?
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("current fresh guards")?;
    for row in &source.rows {
        match &row.state {
            AdmissionAuthorityGuardStateV1::Account { presence, .. } => assert!(presence.is_none()),
            AdmissionAuthorityGuardStateV1::Character {
                holder,
                lease_generation,
                ..
            } => {
                assert!(holder.is_none());
                source.current.character_lease_generation = *lease_generation;
            }
            AdmissionAuthorityGuardStateV1::Runtime {
                ownership_generation,
                ready,
                ..
            } => {
                assert!(*ready);
                source.current.scope_ownership_generation = *ownership_generation;
            }
            _ => {}
        }
    }
    assert_eq!(
        source.current.character_id,
        CharacterId::decode(character.as_bytes()).map_err(debug)?
    );
    assert_eq!(
        RuntimeScopeRefV1::channel(source.current.world_id, source.current.channel_id),
        origin
    );
    let character_text: String = sqlx::query_scalar("SELECT character_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    let session = GameSessionId::decode(&id(197)).map_err(debug)?;
    let transport = AuthenticatedTransportRefV1::decode(&[197; 16]).map_err(debug)?;
    let request =
        source.activity_room_request(session, transport, &character_text, [197; 32], now)?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    assert!(matches!(
        store.commit(&request).await?,
        FreshAdmissionDurableOutcomeV1::Committed(_)
    ));
    let current = store.current_session_at(session).await?.0;
    assert_eq!(current.current_transport(), Some(transport));
    assert_eq!(
        item_fence_of(current)
            .ok_or("fresh current fence")?
            .character_id,
        character
    );
    Ok(current)
}
