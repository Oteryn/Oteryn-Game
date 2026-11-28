use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestRefusal {
    CapabilityNotSelected,
    UnknownQuest,
    LimitExceeded,
    Malformed,
    RevisionMismatch,
    Rejected,
    MissionTrackingRejected,
    ReconciliationRequired,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestState {
    pub selected: bool,
    /// Capability 577 projects each confirmed pair as a single-mission quest line.
    pub mission_tracking: bool,
    pub revision: Option<u64>,
    pub log: QuestLog,
    pub list: Vec<QuestListEntry>,
    pub refusal: Option<QuestRefusal>,
}

/// Ordered updates: a replacement discards the entire old projection before later deltas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestUpdate {
    Snapshot(GatedSnapshot),
    Delta(AppliedDelta<QuestLog>),
}

impl QuestState {
    pub fn apply_update(&mut self, update: &QuestUpdate) -> Result<(), QuestRefusal> {
        match update {
            QuestUpdate::Delta(delta) => self.apply(delta),
            QuestUpdate::Snapshot(snapshot) => {
                self.require_selected()?;
                *self = Self::from_selected_snapshot(self.mission_tracking, snapshot)?;
                Ok(())
            }
        }
    }

    pub fn from_snapshot(
        selected: &[u32],
        snapshot: Option<&GatedSnapshot>,
    ) -> Result<Self, QuestRefusal> {
        if !selected.contains(&CAPABILITY_QUEST_LOG_V1) {
            return if snapshot.is_some() {
                Err(QuestRefusal::CapabilityNotSelected)
            } else {
                Ok(Self::default())
            };
        }
        Self::from_selected_snapshot(
            selected.contains(&CAPABILITY_QUEST_MISSION_TRACKING_V1),
            snapshot.ok_or(QuestRefusal::Malformed)?,
        )
    }

    fn from_selected_snapshot(
        mission_tracking: bool,
        snapshot: &GatedSnapshot,
    ) -> Result<Self, QuestRefusal> {
        if snapshot.domain_id != STATE_DOMAIN_QUEST_LOG
            || snapshot.snapshot_type != SNAPSHOT_TYPE_QUEST_LOG_V1
        {
            return Err(QuestRefusal::Malformed);
        }
        let log = decode_quest_log(&snapshot.payload).map_err(wire_refusal)?;
        let mut state = Self {
            selected: true,
            mission_tracking,
            revision: Some(snapshot.revision),
            ..Self::default()
        };
        state.replace(log);
        Ok(state)
    }

    pub fn apply(&mut self, delta: &AppliedDelta<QuestLog>) -> Result<(), QuestRefusal> {
        self.require_selected()?;
        if self.revision != Some(delta.base_revision) || delta.new_revision <= delta.base_revision {
            return Err(QuestRefusal::RevisionMismatch);
        }
        // Session validates the bytes; validate typed input too before changing state.
        let encode = if self.mission_tracking {
            encode_quest_log_missions
        } else {
            encode_quest_log
        };
        encode(&delta.value).map_err(wire_refusal)?;
        self.replace(delta.value.clone());
        self.revision = Some(delta.new_revision);
        self.refusal = None;
        Ok(())
    }

    fn replace(&mut self, log: QuestLog) {
        if let Some(QuestLogView::List(list)) = &log.view {
            self.list.clone_from(list);
        }
        for entry in &mut self.list {
            let line = match &log.view {
                Some(QuestLogView::Line(line)) if line.quest_index == entry.quest_index => {
                    Some(line)
                }
                _ => log
                    .tracked
                    .iter()
                    .find(|line| line.quest_index == entry.quest_index),
            };
            if let Some(line) = line {
                entry.completed = line.completed;
            }
        }
        self.log = log;
    }

    pub(super) fn require_selected(&self) -> Result<(), QuestRefusal> {
        if self.selected {
            Ok(())
        } else {
            Err(QuestRefusal::CapabilityNotSelected)
        }
    }

    pub(super) fn known(&self, quest: u32) -> bool {
        self.list.iter().any(|q| q.quest_index == quest)
            || self.log.tracked.iter().any(|q| q.quest_index == quest)
            || matches!(&self.log.view, Some(QuestLogView::Line(line)) if line.quest_index == quest)
    }

    pub fn record_result(
        &mut self,
        status: CommandStatus,
        payload: &[u8],
    ) -> Result<(), QuestRefusal> {
        if !payload.is_empty() {
            return Err(QuestRefusal::Malformed);
        }
        self.refusal = match status {
            CommandStatus::Accepted => None,
            CommandStatus::Rejected => Some(QuestRefusal::Rejected),
            CommandStatus::DuplicateReplay | CommandStatus::DuplicateOutcomeExpired => {
                Some(QuestRefusal::ReconciliationRequired)
            }
        };
        Ok(())
    }
}

pub(super) fn wire_refusal(error: QuestLogWireError) -> QuestRefusal {
    match error {
        QuestLogWireError::Malformed => QuestRefusal::Malformed,
        QuestLogWireError::LimitExceeded => QuestRefusal::LimitExceeded,
    }
}
