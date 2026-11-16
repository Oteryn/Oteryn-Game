//! Real in-game Premium window using the shared native form owner.
use crate::play::PlayLink;
pub(in crate::play) fn open(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    crate::play::feature_0115_mail::view::native::open(link, super::premium_game::premium_form(link));
}
