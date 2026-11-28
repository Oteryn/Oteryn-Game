use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::character_info::{CharacterInfoReply, offered};
use oteryn_protocol_oteryn::character_info::{
    CAPABILITY_CHARACTER_INFO_V1, CharacterInfoOperation, CharacterTitle, PageRequest,
};

#[test]
fn character_info_foreign_titles_are_read_only_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(admin, "foreign_titles", true, false)
                    .await?;
                let result = query_foreign_titles(&h).await;
                h.cleanup().await?;
                let (before, views) = result?;
                for (reply, after) in views {
                    let CharacterInfoReply::Payload(bytes) = reply else {
                        return Err("consented Character Info query was rejected".into());
                    };
                    assert_eq!(bytes.get(69..), Some([0, 0].as_slice()));
                    assert_eq!(
                        after, before,
                        "a viewer must leave both target title rows byte-identical"
                    );
                }
                eprintln!("POSTGRES_COMPLETED consented foreign Character Info");
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "foreign title worker panicked")??;
    Ok(())
}

type TitleRows = (Vec<u8>, Vec<u8>);
type ForeignViews = (TitleRows, [(CharacterInfoReply, TitleRows); 2]);

async fn query_foreign_titles(h: &Harness) -> TestResult<ForeignViews> {
    prepare_current_session(h).await?;
    h.seed_appearance_observer().await?;
    let target = id(101);
    sqlx::query("WITH list AS (
        INSERT INTO game_vip_lists(account_id,world_id,next_entry_no)
        SELECT account_id,world_id,2 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
        RETURNING account_id,world_id
    ), entry AS (
        INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no)
        SELECT account_id,world_id,encode($2,'hex')::uuid,1 FROM list
        RETURNING account_id,world_id,character_id
    )
    INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at)
    SELECT account_id,world_id,encode($1,'hex')::uuid,character_id,2,now()+interval '1 day' FROM entry")
        .bind(id(41).as_slice()).bind(target.as_slice()).execute(&h.pool).await?;
    let title = i16::from(CharacterTitle::AdeptEnhancer.id());
    sqlx::query(
        "INSERT INTO game_character_titles(character_id,title,earned_at_unix_ms)
        VALUES(encode($1,'hex')::uuid,$2,100)",
    )
    .bind(target.as_slice())
    .bind(title)
    .execute(&h.pool)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_title_selection(character_id,title,revision)
        VALUES(encode($1,'hex')::uuid,$2,1)",
    )
    .bind(target.as_slice())
    .bind(title)
    .execute(&h.pool)
    .await?;
    let before = title_rows(h, target).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(h, &authority);
    let mut admitted = deps.admitted;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("viewer session fence")?
            .item_fence(),
    );
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered(&[], true), &[CAPABILITY_CHARACTER_INFO_V1])
            .ok_or("Character Info capability")?;
    let query = owner
        .character_info_command(
            admitted,
            CharacterInfoOperation::QueryVisible {
                character: target,
                page: PageRequest {
                    offset: 0,
                    limit: 1,
                },
            },
        )
        .await;
    let after_query = title_rows(h, target).await?;
    let look = owner
        .character_info_command(
            admitted,
            CharacterInfoOperation::LookVisible { character: target },
        )
        .await;
    let after_look = title_rows(h, target).await?;
    Ok((before, [(query, after_query), (look, after_look)]))
}

async fn title_rows(h: &Harness, target: [u8; 16]) -> TestResult<TitleRows> {
    // A deleted award is encoded as empty bytes so the assertion reports both rows.
    Ok(sqlx::query_as(
        "SELECT
            convert_to(COALESCE((SELECT row_to_json(t)::text FROM game_character_titles t
                WHERE t.character_id=encode($1,'hex')::uuid AND t.title=$2),''), 'UTF8'),
            convert_to((SELECT row_to_json(s)::text FROM game_character_title_selection s
                WHERE s.character_id=encode($1,'hex')::uuid), 'UTF8')",
    )
    .bind(target.as_slice())
    .bind(i16::from(CharacterTitle::AdeptEnhancer.id()))
    .fetch_one(&h.pool)
    .await?)
}
