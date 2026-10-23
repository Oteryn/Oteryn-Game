//! Matching operation/event hashes alone never prove an audit envelope or a transition.
use super::*;

#[test]
fn guild_event_guard_membership_payload_has_complete_binding() -> TestResult {
    for action in 1..=5 {
        text_run("guild_event_whole", async |h| {
            historical_character(h, 91, id(WORLD)).await?;
            initial_guild(h, 1, if matches!(action, 3 | 4) { 91 } else { 0 }).await?;
            if action == 4 {
                initial_member(h, 1, 0, 3).await?;
            } else if action == 5 {
                initial_member(h, 1, 91, 3).await?;
            }
            if matches!(action, 2 | 3) {
                initial_invitation(
                    h,
                    1,
                    if action == 3 { 0 } else { 91 },
                    if action == 3 { 91 } else { 0 },
                    false,
                )
                .await?;
            }
            guard(h).await?;
            let operation = match action {
                1 => GuildCommand::Invite(character(91)),
                2 => GuildCommand::Revoke(character(91)),
                3 => GuildCommand::Accept,
                4 => GuildCommand::Leave,
                5 => GuildCommand::Exclude(character(91)),
                _ => return Err("unexpected membership action".into()),
            };
            let req = request(1, 1, operation);
            let valid = serde_json::to_value(GuildAuditProposal {
                version: 1,
                occurrence: req.identity.occurrence,
                transaction_id: req.identity.transaction_id,
                event_id: req.identity.event_id,
                guild_id: *gid(1).as_bytes(),
                world_id: id(WORLD),
                actor: *character(0).as_bytes(),
                target: *if matches!(action, 3 | 4) {
                    character(0)
                } else {
                    character(91)
                }
                .as_bytes(),
                action,
                previous_revision: 1,
                revision: 2,
                rank_before: matches!(action, 4 | 5).then_some(3),
                rank_after: (action == 3).then_some(4),
                removed_invitations: u8::from(matches!(action, 2 | 3)),
                occurred_at_unix_ms: req.identity.occurred_at_unix_ms,
            })?;
            let before = footprint(h).await?;
            event_failure::install_payload(h).await?;
            for key in valid.as_object().ok_or("audit not object")?.keys() {
                for remove in [false, true] {
                    let mut altered = valid.clone();
                    if remove {
                        altered
                            .as_object_mut()
                            .ok_or("audit not object")?
                            .remove(key);
                    } else {
                        altered[key] = match key.as_str() {
                            "actor" | "target" | "guild_id" | "world_id" | "occurrence"
                            | "transaction_id" | "event_id" => serde_json::to_value(sid(999, 90))?,
                            _ => serde_json::json!(99),
                        };
                    }
                    event_failure::set_payload(h, &altered).await?;
                    assert!(
                        apply(h, req.clone(), fence()?).await.is_err(),
                        "action={action}, field={key}, remove={remove}"
                    );
                    assert_eq!(footprint(h).await?, before);
                    assert_eq!(revision(h, 1).await?, "1");
                }
            }
            let mut extra = valid.clone();
            extra["caller_authority"] = true.into();
            event_failure::set_payload(h, &extra).await?;
            assert!(apply(h, req.clone(), fence()?).await.is_err());
            assert_eq!(footprint(h).await?, before);
            event_failure::set_payload(h, &valid).await?;
            assert_eq!(
                result(apply(h, req.clone(), fence()?).await?).result,
                GuildResult::Ok
            );
            event_failure::remove_payload(h).await?;
            assert!(matches!(
                apply(h, req, fence()?).await?,
                GuildOutcome::Replayed(_)
            ));
            Ok(())
        })?;
    }
    Ok(())
}

#[test]
fn guild_event_guard_cannot_fabricate_leave_success_for_missing_member() -> TestResult {
    text_run("guild_event_noop", async |h| {
        historical_character(h, 91, id(WORLD)).await?;
        initial_guild(h, 1, 91).await?;
        guard(h).await?;
        // A proposed success with no source member previously passed the final empty-state
        // test. Reject it at receipt insertion, before any event/revision can be manufactured.
        let req = request(1, 1, GuildCommand::Leave);
        let error = sqlx::query("INSERT INTO game_guild_operations(occurrence,transaction_id,requested_event_id,game_session_id,command_id,actor,world_id,guild_id,target,action,request_binding,result,previous_revision,revision,event_id,audit_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($5,'hex')::uuid,4,sha256('request'::bytea),0,1,2,encode($3,'hex')::uuid,sha256('audit'::bytea))")
            .bind(req.identity.occurrence.as_slice()).bind(req.identity.transaction_id.as_slice()).bind(req.identity.event_id.as_slice())
            .bind(req.command.game_session_id().as_bytes().as_slice()).bind(character(0).as_bytes().as_slice())
            .bind(id(WORLD).as_slice()).bind(gid(1).as_bytes().as_slice()).execute(&h.pool).await.expect_err("missing source member accepted");
        let database = error.as_database_error().ok_or("not a database error")?;
        assert_eq!(database.code().as_deref(), Some("23514"));
        assert!(
            database
                .message()
                .contains("actual actor/target/invitation")
        );
        assert_eq!(h.count("game_guild_operations").await?, 0);
        assert_eq!(revision(h, 1).await?, "1");
        Ok(())
    })
}
