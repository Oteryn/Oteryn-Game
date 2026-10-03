// COPY-only handwritten insertion INSIDE existing explicit_migration, after its
// helpers. Its private Source/seed/migration/load helpers remain private.
async fn recovery_snapshot(h: &Harness) -> TestResult<(String, String)> {
    let admissions = sqlx::query_scalar("SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY recovery_generation),'[]'::jsonb)::text FROM game_character_recovery_admissions a")
        .fetch_one(&h.pool).await?;
    Ok((snapshot(h).await?, admissions))
}

#[test]
fn public_source_aware_recovery_reconciles_exclusive_successor_after_migration() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let h = Harness::create(admin, "prof1_source_recovery", true).await?;
        let outcome = source_recovery_body(&h).await;
        h.cleanup().await?;
        outcome
    })
}

async fn source_recovery_body(h: &Harness) -> TestResult {
    use crate::character_recovery_fence::CharacterRecoveryStore;
    contention::assert_seed_context(h).await?;
    let root = &h.root;
    let source = Source::view(1);
    // End this complete borrow scope before acquiring exclusive recovery.
    {
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = root.open_character_authority(&seal).await.map_err(debug)?;
        let actor = Actor {
            h,
            authority: &authority,
        };
        seed(&actor).await?;
        let result = actor
            .call(actor.at(8, migration(110)?)?.source(source))
            .await
            .map_err(debug)?;
        let Committed(receipt) = result else {
            return Err(debug(result).into());
        };
        assert_eq!(receipt.committed_character_revision.get(), 9);
        assert_eq!(load(&actor, source).await.map_err(debug)?.len(), 2);
        drop(
            root.open_character_authority_with_proficiency_definitions(
                &seal,
                Some(Arc::new(source)),
            )
            .await
            .map_err(debug)?,
        );
    }
    let before = recovery_snapshot(h).await?;
    assert_eq!(h.root_revision().await?, "9");
    assert_eq!(h.count("game_character_recovery_admissions").await?, 1);

    // Independent real retained predecessor from another scope. A valid
    // migration source does not replace the database's external proof.
    let parent = std::env::temp_dir().join(format!(
        "oteryn-prof1-foreign-parent-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&parent)?;
    let name = h
        .database
        .url
        .rsplit('/')
        .next()
        .ok_or("fixture database name")?;
    let directory = parent.join(name);
    std::fs::create_dir(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700))?;
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    }
    let foreign =
        CharacterRecoveryStore::open(&directory, "character-other", "game-ops").map_err(debug)?;
    drop(foreign.authorize_fresh_store(id(170), 100).map_err(debug)?);
    let foreign_transition = foreign.begin_recovery(1, id(171), 200).map_err(debug)?;
    assert!(matches!(
        root.reconcile_character_recovery_with_proficiency_definitions(
            &foreign_transition,
            Some(Arc::new(source))
        )
        .await,
        Err(AdmissionError::Unavailable(Unavailable))
    ));
    assert_eq!(
        recovery_snapshot(h).await?,
        before,
        "foreign predecessor admitted"
    );
    drop(foreign_transition);
    drop(foreign);
    std::fs::remove_dir_all(directory)?;

    let transition = h.recovery.begin_recovery(1, id(180), 200).map_err(debug)?;
    assert_eq!(transition.record().recovery_generation, 2);
    assert_eq!(transition.record().predecessor_generation, 1);
    assert!(matches!(
        root.reconcile_character_recovery(&transition).await,
        Err(AdmissionError::Unavailable(Unavailable))
    ));
    assert_eq!(
        recovery_snapshot(h).await?,
        before,
        "no-source successor insert did not roll back"
    );
    for fault in [Fault::Missing, Fault::Map] {
        assert!(
            matches!(
                root.reconcile_character_recovery_with_proficiency_definitions(
                    &transition,
                    Some(Arc::new(source.fault(fault)))
                )
                .await,
                Err(AdmissionError::Unavailable(Unavailable))
            ),
            "{fault:?}"
        );
        assert_eq!(
            recovery_snapshot(h).await?,
            before,
            "{fault:?}: successor insert did not roll back"
        );
    }
    root.reconcile_character_recovery_with_proficiency_definitions(
        &transition,
        Some(Arc::new(source)),
    )
    .await
    .map_err(debug)?;
    let admitted = recovery_snapshot(h).await?;
    assert_eq!(admitted.0, before.0, "recovery changed roots or WP history");
    assert_ne!(admitted.1, before.1);
    let generations: Vec<String> = sqlx::query_scalar("SELECT recovery_generation::text FROM game_character_recovery_admissions ORDER BY recovery_generation")
        .fetch_all(h.pool).await?;
    assert_eq!(generations, ["1", "2"]);
    root.reconcile_character_recovery_with_proficiency_definitions(
        &transition,
        Some(Arc::new(source)),
    )
    .await
    .map_err(debug)?;
    assert_eq!(
        recovery_snapshot(h).await?,
        admitted,
        "idempotent recovery changed admission"
    );
    drop(transition);

    let seal = h.recovery.seal_current().map_err(debug)?;
    assert_eq!(seal.record().recovery_generation, 2);
    assert_eq!(seal.record().recovery_event_id, id(180));
    let authority = root
        .open_character_authority_with_proficiency_definitions(&seal, Some(Arc::new(source)))
        .await
        .map_err(debug)?;
    let actor = Actor {
        h,
        authority: &authority,
    };
    let tracks = load(&actor, source).await.map_err(debug)?;
    assert_eq!(tracks.len(), 2);
    for item in [SWORD, AXE] {
        let track = tracks
            .iter()
            .find(|t| t.state.item_key() == item)
            .ok_or("missing recovered track")?;
        assert_eq!(track.state, retained(item, NEW, PROGRESS, &choices(item))?);
        assert_eq!(track.committed_character_revision.get(), 9);
    }
    assert_eq!(h.root_revision().await?, "9");
    assert_eq!(
        recovery_snapshot(h).await?,
        admitted,
        "sealed generation-two read changed durable state"
    );
    Ok(())
}
