use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestRefusal {
    CapabilityNotSelected,
    UnknownQuest,
    LimitExceeded,
    Malformed,
    RevisionMismatch,
    Rejected,
    ReconciliationRequired,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestState {
    pub selected: bool,
    pub revision: Option<u64>,
    pub log: QuestLog,
    pub list: Vec<QuestListEntry>,
    pub refusal: Option<QuestRefusal>,
}

impl QuestState {
    pub fn from_snapshot(
        selected: &[u32],
        snapshot: Option<&GatedSnapshot>,
    ) -> Result<Self, QuestRefusal> {
        let mut state = Self {
            selected: selected.contains(&CAPABILITY_QUEST_LOG_V1),
            ..Self::default()
        };
        if !state.selected {
            return if snapshot.is_some() {
                Err(QuestRefusal::CapabilityNotSelected)
            } else {
                Ok(state)
            };
        }
        let snapshot = snapshot.ok_or(QuestRefusal::Malformed)?;
        if snapshot.domain_id != STATE_DOMAIN_QUEST_LOG
            || snapshot.snapshot_type != SNAPSHOT_TYPE_QUEST_LOG_V1
        {
            return Err(QuestRefusal::Malformed);
        }
        let log = decode_quest_log(&snapshot.payload).map_err(wire_refusal)?;
        state.replace(log);
        state.revision = Some(snapshot.revision);
        Ok(state)
    }

    pub fn apply(&mut self, delta: &AppliedDelta<QuestLog>) -> Result<(), QuestRefusal> {
        self.require_selected()?;
        if self.revision != Some(delta.base_revision) || delta.new_revision <= delta.base_revision {
            return Err(QuestRefusal::RevisionMismatch);
        }
        // Session validates the bytes; validate typed input too before changing state.
        encode_quest_log(&delta.value).map_err(wire_refusal)?;
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
