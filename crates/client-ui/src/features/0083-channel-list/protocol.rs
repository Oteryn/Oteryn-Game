pub use oteryn_client_runtime::channel_list::{ServedChannel, decode};
// oteryn-choice: a session replacement clears the previous node's projection, including absent data.
pub fn apply(state: &mut Option<ServedChannel>, channel: Option<ServedChannel>) {
    *state = channel;
}
