use oteryn_input_actions::NormalizedInputEvent;

pub fn open_key(events: &[NormalizedInputEvent]) -> bool {
    crate::staff_tools::open_key(events)
}
