#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleList {
    Attack,
    Defence,
}

impl ScheduleList {
    pub(crate) const fn purpose(self) -> &'static str {
        match self {
            Self::Attack => "AI_ATTACK",
            Self::Defence => "AI_DEFENCE",
        }
    }

    pub(crate) const fn atom(self) -> &'static str {
        match self {
            Self::Attack => "attack",
            Self::Defence => "defence",
        }
    }
}
