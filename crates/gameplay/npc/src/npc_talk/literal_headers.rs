//! Reuse the previously qualified NPCFIX read-only multipart predicate.
pub(super) fn topic(keyword: &crate::content::ProjectV2DialogueKeyword) -> bool {
    !keyword.reply.is_empty()
        && !keyword.triggers.is_empty()
        && keyword.children.is_empty()
        && !keyword.fallback
        && !keyword.only_unfocus
        && !keyword.reset
        && !keyword.ungreet
        && keyword.move_up.is_none()
        && keyword.links.is_empty()
}
