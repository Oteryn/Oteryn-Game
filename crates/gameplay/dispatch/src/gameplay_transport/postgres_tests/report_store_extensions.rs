//! Negotiated report and Store commands through the admitted connection.
use super::fixture::{Harness, debug, id, run};
use super::store_fixtures::{platform, serve};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::domain::CharacterId;
use crate::durability::guild_chat_fixture as characters;
use crate::foundation::CommandStatus;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_fence_of;
use oteryn_protocol_oteryn::{player_report as report, store::browse};

fn scenario(case: u32) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "report_store_extensions", true, false)
                .await?;
        prepare_current_session(&harness).await?;
        let target = CharacterId::from_bytes(characters::sid(90, 1)).map_err(debug)?;
        if case == 6333 {
            characters::historical_character(
                &harness.pool,
                90,
                target,
                characters::sid(90, 2),
                id(42),
                "Reported Player",
            )
            .await?;
        }
        let deps = Dependencies::new()?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, _) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = controller.active();
        owner.store = Some(platform(deps.admitted.world_id.as_bytes()));
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(admitted.game_session_id)
        .await?
        .0;
        admitted.item_fence = item_fence_of(current);
        let (capability, payload) = match case {
            6333 => {
                let mut bytes = vec![2];
                bytes.extend(
                    report::encode_player_report_request(&report::PlayerReportRequest {
                        kind: report::KIND_RULE_VIOLATION,
                        category: 0,
                        reason: 1,
                        action: 0,
                        target: "Reported Player".into(),
                        comment: "Inappropriate name".into(),
                        map_x: 0,
                        map_y: 0,
                        map_floor: 0,
                    })
                    .map_err(debug)?,
                );
                (585, bytes)
            }
            6334 => (585, vec![1, b'M']),
            6335 => (
                586,
                browse::Query {
                    category: "featured".into(),
                    search: String::new(),
                    sort: 0,
                    cursor: String::new(),
                }
                .encode()
                .map_err(debug)?,
            ),
            6336 => (586, Vec::new()),
            6337 => (587, b"Al Dric".to_vec()),
            _ => return Err("unknown test command".into()),
        };
        let command = (case, payload);
        assert_eq!(
            serve(&owner, &mut admitted, std::slice::from_ref(&command)).await?,
            [(CommandStatus::Rejected, Vec::new())]
        );
        let required = if capability == 585 { 76 } else { 154 };
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[required, capability])
                .ok_or("extension capability not offered")?;
        let mut commands = vec![command];
        if case == 6333 {
            let malformed = report::PlayerReportRequest {
                kind: report::KIND_RULE_VIOLATION,
                category: 0,
                reason: 1,
                action: 0,
                target: "Reported Player".into(),
                comment: "Unstorable\0comment".into(),
                map_x: 0,
                map_y: 0,
                map_floor: 0,
            };
            let mut payload = vec![1];
            payload.extend(report::encode_player_report_request(&malformed).map_err(debug)?);
            commands.push((case, payload));
        }
        if case == 6334 {
            commands.push((case, vec![2]));
        }
        if case == 6337 {
            commands.push((case, b"Al  Dric".to_vec()));
            commands.push((case, b"Aldric1".to_vec()));
        }
        let results = serve(&owner, &mut admitted, &commands).await?;
        match case {
            6333 => {
                assert_eq!(results[0].0, CommandStatus::Accepted);
                let reply = report::decode_player_report_reply(&results[0].1).map_err(debug)?;
                assert_eq!(reply.code, report::PlayerReportCode::Accepted);
                let stored: (String, i16) = sqlx::query_as(
                    "SELECT encode(uuid_send(reported_character_id), 'hex'), sanction_suggestion FROM game_player_reports WHERE report_no=$1",
                ).bind(i64::try_from(reply.report)?).fetch_one(&harness.pool).await?;
                let target_hex = format!("{:032x}", u128::from_be_bytes(*target.as_bytes()));
                assert_eq!(stored, (target_hex, 2));
                assert_eq!(results.get(1), Some(&(CommandStatus::Rejected, Vec::new())));
            }
            6334 => assert_eq!(
                results,
                vec![(CommandStatus::Rejected, b"Sorry, not possible.".to_vec()); 2]
            ),
            6335 => {
                assert_eq!(results[0].0, CommandStatus::Accepted);
                let page = browse::Page::decode(&results[0].1).map_err(debug)?;
                // Neither Crystal-marked mount is commercially admitted in the pinned catalogue.
                assert!(page.products.is_empty());
                assert!(page.next.is_empty());
                assert_eq!(page.total, 60);
            }
            6336 => {
                assert_eq!(results[0].0, CommandStatus::Accepted);
                let expected = [
                    b"\x0a\x02".as_slice(),
                    &(b"home/banner_armouredarcher.png".len() as u16).to_le_bytes(),
                    b"home/banner_armouredarcher.png",
                    &(b"home/banner_podiumoftenacity.png".len() as u16).to_le_bytes(),
                    b"home/banner_podiumoftenacity.png",
                ]
                .concat();
                assert_eq!(results[0].1, expected);
            }
            6337 => assert_eq!(
                results,
                [
                    (CommandStatus::Accepted, vec![1]),
                    (CommandStatus::Accepted, vec![0]),
                    (CommandStatus::Accepted, vec![0]),
                ]
            ),
            _ => return Err("unknown test command".into()),
        }
        Ok(())
    })
}

#[test]
fn named_report_dispatch_retains_non_authoritative_sanction_choice() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(6333).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "named report dispatch panicked")??;
    Ok(())
}

#[test]
fn report_conversation_dispatch_refuses_continue_and_close() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(6334).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "report conversation dispatch panicked")??;
    Ok(())
}

#[test]
fn store_recent_dispatch_only_returns_admitted_new_offers() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(6335).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "recent offers dispatch panicked")??;
    Ok(())
}

#[test]
fn store_banner_dispatch_returns_donor_order_and_cycle() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(6336).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "banner dispatch panicked")??;
    Ok(())
}

#[test]
fn store_name_input_dispatch_enforces_character_name_rules() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(6337).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Store name input dispatch panicked")??;
    Ok(())
}
