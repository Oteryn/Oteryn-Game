//! Small test-only wrapper around the existing DEATH-1 request fixture.
pub(crate) fn pvp_death_request(tag: u8) -> super::TestResult<super::CharacterDeathRequest<10>> {
    super::death(tag, &[])
}

pub(crate) async fn join_pvp_writers<A: std::future::Future, B: std::future::Future>(
    first: A,
    second: B,
) -> (A::Output, B::Output) {
    super::super::join_two(first, second).await
}
