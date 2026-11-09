//! Accepted ARENA1 A6 cumulative Drome levels, from the same repeatable-read World cut.
use super::*;
pub(super) async fn read_lifetime_levels(
    tx: &mut Transaction<'_, Postgres>,
    characters: &[Source],
    limits: CutLimits,
) -> Result<Vec<Source>> {
    let mut output = Vec::new();
    for page in characters.chunks(limits.page_rows) {
        let ids: Vec<_> = page
            .iter()
            .map(|source| uuid_hex(source.character_id.as_bytes()))
            .collect();
        let rows=sqlx::query("SELECT uuid_send(character_id) AS character_id,body FROM game_character_tibiadrome WHERE character_id=ANY($1::text[]::uuid[]) ORDER BY character_id")
            .bind(&ids).fetch_all(&mut **tx).await?;
        let mut known = BTreeMap::new();
        for row in rows {
            let state: super::super::tibiadrome::Record =
                serde_json::from_value(row.try_get("body")?).map_err(|_| corrupt())?;
            if let Some(levels) = state.progress.lifetime_levels {
                known.insert(character(&row)?, levels);
            }
        }
        for source in page {
            let Some(levels) = known.remove(&source.character_id) else {
                continue;
            };
            let mut values = [(0, 0); 9];
            values[0] = (levels, 0);
            output.push(Source {
                character_id: source.character_id,
                name: source.name.clone(),
                vocation: source.vocation.clone(),
                level: source.level,
                values,
            });
        }
    }
    Ok(output)
}
