//! Convert committed owner views into bounded pages of connection-scoped handles.
use super::handles::{HandleKind, SessionTeamFinderView};
use crate::durability::team_finder::{ApplicantRow, ListingRow, MutationOutcome};
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::team_finder::LeaderListing;
use oteryn_protocol_oteryn::team_finder_members::*;

pub(super) enum DatabaseResult {
    Browse(Vec<ListingRow>),
    Applicants(Option<LeaderListing>, u16, Vec<ApplicantRow>),
    Mutation(MutationOutcome),
}

pub(super) fn project(
    view: &mut SessionTeamFinderView,
    admitted: AdmittedSession,
    result: DatabaseResult,
) -> Option<(bool, Vec<u8>)> {
    let session = admitted.game_session_id;
    let generation = admitted.continuity.connection_generation;
    let result = match result {
        DatabaseResult::Mutation(receipt) => return Some((receipt.accepted, Vec::new())),
        DatabaseResult::Browse(rows) => {
            let more = rows.len() > MAX_PAGE_ENTRIES;
            view.replace_page(HandleKind::Listing);
            let listings = rows
                .into_iter()
                .take(MAX_PAGE_ENTRIES)
                .map(|row| {
                    Some(BrowseListing {
                        leader_handle: view.handle(
                            session,
                            generation,
                            HandleKind::Listing,
                            row.leader_target,
                        )?,
                        name: row.name,
                        listing: row.listing,
                        occupied_slots: row.occupied_slots,
                        status: row.status,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            let next = if more {
                Some(listings.last()?.leader_handle)
            } else {
                None
            };
            TeamFinderResult::Browse { listings, next }
        }
        DatabaseResult::Applicants(listing, occupied_slots, rows) => {
            let more = rows.len() > MAX_PAGE_ENTRIES;
            view.replace_page(HandleKind::Applicant);
            let members = rows
                .into_iter()
                .take(MAX_PAGE_ENTRIES)
                .map(|row| {
                    Some(Applicant {
                        handle: view.handle(
                            session,
                            generation,
                            HandleKind::Applicant,
                            row.target,
                        )?,
                        name: row.name,
                        level: row.level,
                        vocation: row.vocation,
                        status: row.status,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            let next = if more {
                Some(members.last()?.handle)
            } else {
                None
            };
            TeamFinderResult::Applicants {
                listing,
                occupied_slots,
                members,
                next,
            }
        }
    };
    Some((true, encode_result(&result).ok()?))
}
