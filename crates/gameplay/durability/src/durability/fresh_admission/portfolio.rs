use super::*;
use sqlx::Acquire;

pub(in crate::durability) async fn lock_admission_portfolio(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account: &str,
) -> Result<()> {
    lock_admission_portfolios(tx, &[account], false, false).await
}

/// Acquiring both locks in a savepoint lets an unrelated account use the admission
/// relations while a portfolio is held by character creation.
pub(in crate::durability) async fn lock_admission_portfolios(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    accounts: &[impl AsRef<str>],
    create_missing: bool,
    no_key_update: bool,
) -> Result<()> {
    let row_lock = if no_key_update {
        "SELECT 1 FROM game_character_account_guards WHERE account_id=$1::uuid FOR NO KEY UPDATE NOWAIT"
    } else {
        "SELECT 1 FROM game_character_account_guards WHERE account_id=$1::uuid FOR UPDATE NOWAIT"
    };
    loop {
        let mut attempt = tx.begin().await?;
        super::super::db::lock_admission_relations(&mut attempt).await?;
        let acquired = async {
            for account in accounts {
                let account = account.as_ref();
                let found = sqlx::query(row_lock).bind(account).fetch_optional(&mut *attempt).await?;
                if found.is_some() || !create_missing {
                    continue;
                }
                // ON CONFLICT can wait on a concurrent insertion of an invisible row.
                let timeout: String = sqlx::query_scalar("SHOW lock_timeout").fetch_one(&mut *attempt).await?;
                sqlx::query("SET LOCAL lock_timeout = '1ms'").execute(&mut *attempt).await?;
                sqlx::query("INSERT INTO game_character_account_guards(account_id) VALUES($1::uuid) ON CONFLICT DO NOTHING")
                    .bind(account).execute(&mut *attempt).await?;
                sqlx::query(row_lock).bind(account).fetch_one(&mut *attempt).await?;
                sqlx::query("SELECT set_config('lock_timeout',$1,true)").bind(timeout).execute(&mut *attempt).await?;
            }
            Ok::<_, sqlx::Error>(())
        }
        .await;
        match acquired {
            Ok(()) => {
                attempt.commit().await?;
                return Ok(());
            }
            Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("55P03") => {
                attempt.rollback().await?;
                // The semantic pass supplies the outer transaction deadline.
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
}
