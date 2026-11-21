use super::*;
#[test]
fn nx1_staff_ghost_uses_donor_access_and_always_retains_own_entity()
-> Result<(), Box<dyn std::error::Error>> {
    let (runtime, gm, viewer, gm_session) =
        crate::gameplay_transport::spell_presentations::tests::runtime();
    let census = runtime
        .positioned_actor_census()
        .map_err(|e| format!("{e:?}"))?;
    let viewer_session = census
        .iter()
        .find_map(|(a, _, s)| (*a == viewer).then_some(*s).flatten())
        .ok_or("viewer session")?;
    let entries = census
        .iter()
        .filter(|(_, _, session)| session.is_some())
        .map(|(a, p, _)| world_spatial::ChannelEntity {
            source_icons: Default::default(),
            display_name: None,
            health_percent: 100,
            direction: world_spatial::StepDirection::South,
            kind: world_spatial::VisibleKind::Player,
            identity: a.placement_identity(),
            generation: a.actor_local_generation(),
            position: world_spatial::ActorPosition {
                x: p.position().x,
                y: p.position().y,
                floor: p.position().floor,
            },
            revision: 1,
        })
        .collect::<Vec<_>>();
    let view = |observer: ExactActorRef| world_spatial::ChannelEntities {
        content_generation: [1; 32],
        observer: observer.placement_identity(),
        entities: entries.clone(),
        lights: Default::default(),
        observed_at_ms: 0,
    };
    let flags = WorldTools::default();
    assert!(flags.set(
        gm,
        gm_session,
        StaffLiveStatus {
            ghost: true,
            invisible: true,
            group: 4,
            ..Default::default()
        }
    ));
    assert!(flags.invisible(gm));
    let mut normal = view(viewer);
    flags.filter(viewer, &mut normal);
    assert!(
        !normal
            .entities
            .iter()
            .any(|e| e.identity == gm.placement_identity())
    );
    assert!(
        normal
            .entities
            .iter()
            .any(|e| e.identity == viewer.placement_identity())
    );
    let mut own = view(gm);
    flags.filter(gm, &mut own);
    assert!(
        own.entities
            .iter()
            .any(|e| e.identity == gm.placement_identity())
    );
    assert!(flags.set(
        viewer,
        viewer_session,
        StaffLiveStatus {
            group: 5,
            ..Default::default()
        }
    ));
    let mut staff = view(viewer);
    flags.filter(viewer, &mut staff);
    assert!(
        staff
            .entities
            .iter()
            .any(|e| e.identity == gm.placement_identity())
    );
    assert!(flags.set(
        gm,
        gm_session,
        StaffLiveStatus {
            ghost: true,
            group: 6,
            ..Default::default()
        }
    ));
    let mut senior = view(viewer);
    flags.filter(viewer, &mut senior);
    assert!(
        senior
            .entities
            .iter()
            .any(|e| e.identity == gm.placement_identity())
    );
    flags.forget(gm_session);
    assert!(!flags.invisible(gm));
    Ok(())
}
#[test]
fn nx1_staff_message_publication_is_private_bounded_and_idempotent()
-> Result<(), Box<dyn std::error::Error>> {
    let (_, _, _, session) = crate::gameplay_transport::spell_presentations::tests::runtime();
    let hub = crate::gameplay_transport::raid_notices::ServerMessageHub::default();
    assert!(hub.join(session, 1));
    hub.staff_publish("one", Some(session), "Private notice")
        .map_err(|_| "publish")?;
    hub.staff_publish("one", Some(session), "Private notice")
        .map_err(|_| "replay")?;
    assert!(hub.staff_publish("one", Some(session), "changed").is_err());
    assert_eq!(
        hub.take(session, 1),
        Some(
            oteryn_protocol_oteryn::zones_nologout::ServerMessage::StatusText(
                "Private notice".into()
            )
        )
    );
    assert_eq!(hub.take(session, 1), None);
    hub.staff_forget("one");
    hub.staff_publish("broadcast", None, "Public notice")
        .map_err(|_| "broadcast")?;
    assert_eq!(
        hub.take(session, 1),
        Some(
            oteryn_protocol_oteryn::zones_nologout::ServerMessage::Warning("Public notice".into())
        )
    );
    hub.leave(session, 1);
    assert!(
        hub.staff_publish("offline", Some(session), "Absent")
            .is_err()
    );
    Ok(())
}
