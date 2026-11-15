pub fn request(
    p: &mut crate::highscores::HighscoresPanel,
    a: crate::highscores::Action,
) -> Option<oteryn_client_runtime::server_mechanic::ServerMechanicCommand> {
    crate::highscores::request(p, a)
}
pub fn world_choices(
    p: &crate::highscores::HighscoresPanel,
) -> impl Iterator<Item = &oteryn_client_runtime::highscores::catalogue::Entry> {
    crate::highscores::world_choices(p)
}
pub fn apply_result(
    p: &mut crate::highscores::HighscoresPanel,
    r: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
) {
    crate::highscores::apply_result(p, r);
}
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
