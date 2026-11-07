use super::*;

#[test]
fn guild_wire_postgres_roster_pages_and_privacy() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| paging().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "guild paging e2e panicked")??;
    Ok(())
}

fn paging() -> TestResult {
    run(|admin| async move {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "guild_wire_paging", true, false).await?;
        prepare_current_session(&h).await?;
        seed::prepare(&h.pool).await?;
        sqlx::raw_sql(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../apps/game-server/migrations/0161_guild_roster_keyset.sql"
        )))
        .execute(&h.pool)
        .await?;
        let guild = crate::domain::guild::GuildId::from_bytes(seed::sid(300, 3)).map_err(debug)?;
        seed::initial_guild(
            &h.pool,
            guild,
            id(42),
            "Paged Guild",
            CharacterId::from_bytes(id(41)).map_err(debug)?,
            id(40),
        )
        .await?;
        // A legal historical roster with 102 members, before installing native mutation guards.
        for n in 400..501 {
            let character = CharacterId::from_bytes(seed::sid(n, 1)).map_err(debug)?;
            let suffix = [
                b'A' + u8::try_from((n - 400) / 26)?,
                b'A' + u8::try_from((n - 400) % 26)?,
            ];
            let name = format!("Paged Hero {}", std::str::from_utf8(&suffix)?);
            seed::historical_character(&h.pool, n, character, seed::sid(n, 2), id(42), &name)
                .await?;
            seed::initial_member(&h.pool, guild, character, seed::sid(n, 2), 3).await?;
        }
        seed::guard(&h.pool).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let (current, _) =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(current);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[40])
                .ok_or("guild selection")?;
        let guild = *guild.as_bytes();
        let GuildQueryResult::Roster {
            members: first,
            next: Some(after),
            ..
        } = query(
            &owner,
            &mut admitted,
            GuildQuery::Roster { guild, after: None },
        )
        .await?
        else {
            return Err("first page missing cursor".into());
        };
        assert_eq!(first.len(), 100);
        let GuildQueryResult::Roster {
            members: second,
            next,
            ..
        } = query(
            &owner,
            &mut admitted,
            GuildQuery::Roster {
                guild,
                after: Some(after),
            },
        )
        .await?
        else {
            return Err("second page missing".into());
        };
        assert_eq!(second.len(), 2);
        assert!(next.is_none());
        let mut all = first;
        all.extend(second);
        assert!(
            all.windows(2)
                .all(|w| (w[0].level, w[0].character) < (w[1].level, w[1].character))
        );
        let other = seed::sid(999, 3);
        let command = admitted.continuity.next_command_id;
        let (status, bytes) = exchange(
            &owner,
            &mut admitted,
            command,
            COMMAND_TYPE_GUILD_QUERY,
            &encode_guild_query(&GuildQuery::Roster {
                guild: other,
                after: None,
            })
            .map_err(debug)?,
        )
        .await?;
        assert_eq!(status, CommandStatus::Rejected);
        assert!(bytes.is_empty());
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
