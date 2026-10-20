/// Content-bound test policy. Numerical approximation never grants target or session authority.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MagnitudePolicy {
    #[default]
    Strict,
    BaselineTest,
}
