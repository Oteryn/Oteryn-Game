use oteryn_client_runtime::tutorial_vocation as wire;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VocationPanel {
    pub selected: bool,
    pub pending: bool,
    pub observation: Option<wire::Observation>,
    pub feedback: Option<&'static str>,
}
