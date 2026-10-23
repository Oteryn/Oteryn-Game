//! The existing PVP-1a Character-root fixture, shared without another harness.
use super::id;

pub(crate) async fn seed_root(
    pool: &sqlx::PgPool,
    source: u8,
    seed: u8,
    world: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query(
        "INSERT INTO game_character_roots (character_id, account_id, world_id, lifecycle, \
            character_revision, profile_revision, ruleset_revision, content_revision, \
            starter_template_revision, name) \
         SELECT encode($2,'hex')::uuid, account_id, encode($3,'hex')::uuid, lifecycle, \
                1, profile_revision, ruleset_revision, content_revision, \
                starter_template_revision, \
                'Pvp' || chr(97 + $4::int / 26) || chr(97 + $4::int % 26) \
           FROM game_character_roots WHERE character_id = encode($1,'hex')::uuid",
    )
    .bind(id(source).as_slice())
    .bind(id(seed).as_slice())
    .bind(id(world).as_slice())
    .bind(i32::from(seed))
    .execute(pool)
    .await?;
    Ok(())
}
