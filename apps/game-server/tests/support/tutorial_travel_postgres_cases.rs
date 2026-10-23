// Mounted once by the native travel fixture, reusing its genuine authority and placed record.
#[test]
fn tutorial_free_travel_departure_receipt_arrival_and_cooldown_are_atomic() -> TestResult {
    use crate::foundation::{CommandId, CommandRef};
    use oteryn_gameplay_content::content::newhaven_travel::NewhavenRoute;
    use oteryn_gameplay_quests::tutorial::{TutorialProgress, TutorialStep};
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async {
        let h=fixture::Harness::create(admin,"tutorial_free_travel",true).await?;
        let result=async {
            let seal=h.recovery.seal_current().map_err(debug)?;
            let authority=h.root.open_character_authority(&seal).await.map_err(debug)?;
            let current=fence(1)?;
            let progress=TutorialProgress {step:TutorialStep::Island,hunting:true,corruptor:true};
            h.root.character_tutorial_progress(&authority,&h.node,current,Some((0,progress))).await.map_err(debug)?;
            let route=NewhavenRoute::Anna;
            let (x,y,floor)=route.destination();
            let arrival=PendingArrival {occurrence:occurrence(70)?,world_id:current.runtime_scope.world_id(),destination:LastPositionCell {x,y,floor}};
            let command=CommandRef::new(current.game_session_id,CommandId::new(1).map_err(debug)?);
            assert_eq!(h.root.commit_newhaven_travel(&authority,&h.node,current,command,arrival,route).await.map_err(debug)?,Some(arrival));
            assert_eq!(h.root.commit_newhaven_travel(&authority,&h.node,current,command,arrival,route).await.map_err(debug)?,Some(arrival));
            let pending_progress: (i16,bool,bool)=sqlx::query_as("SELECT step,hunting,corruptor FROM game_character_tutorial WHERE character_id=encode($1,'hex')::uuid")
                .bind(current.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(pending_progress,(2,true,true));
            let mut record=placed(20,x)?;
            record.record.cell=arrival.destination;
            assert_eq!(h.root.write_character_last_position(&authority,&h.node,current,record.write_sequence,record.record).await.map_err(debug)?,LastPositionWrite::PendingObligation);
            assert_eq!(h.root.consume_npc_pending_arrival_placed(&authority,&h.node,current,arrival.occurrence,record).await.map_err(debug)?,ArrivalConsume::Consumed);
            assert_eq!(h.root.consume_npc_pending_arrival_placed(&authority,&h.node,current,arrival.occurrence,record).await.map_err(debug)?,ArrivalConsume::NoOp);
            assert_eq!(h.root.commit_newhaven_travel(&authority,&h.node,current,command,arrival,route).await.map_err(debug)?,None);
            let progress: (i16,bool,bool)=sqlx::query_as("SELECT step,hunting,corruptor FROM game_character_tutorial WHERE character_id=encode($1,'hex')::uuid")
                .bind(current.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(progress,(3,false,false));
            let again=CommandRef::new(current.game_session_id,CommandId::new(2).map_err(debug)?);
            let again_arrival=PendingArrival {occurrence:occurrence(71)?,..arrival};
            assert!(matches!(h.root.commit_newhaven_travel(&authority,&h.node,current,again,again_arrival,route).await,Err(CharacterProgressionError::InvalidInput)));
            assert_eq!(h.count("game_newhaven_travel_receipts").await?,1);
            assert_eq!(h.count("game_character_pending_arrivals").await?,0);
            assert_eq!(h.root_revision().await?,"1");
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
