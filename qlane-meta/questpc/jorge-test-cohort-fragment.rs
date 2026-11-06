    let offers = oteryn_gameplay_quests::quest::raid_tokens::offers()?;
    for (i, (item, name)) in offers.iter().enumerate().skip(1)
        .chain(std::iter::once((offers.len(), &offers[0]))) {
        let previous=offers[(i-1).min(offers.len()-1)].0;
        drop_replica(owner, admitted, previous, 20000+i as u64).await?;
        let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
        super::quest_world_callbacks::input_item(owner,fence.item_fence(),&Offering {
            position:Position {x:33293,y:31677,floor:-15},item:"oteryn:item.tibia.i19083".into(),count:20,
        },400+i as u64,true).await?;
        let command=10010+i as u64*3;
        super::classic_quests::talk(owner,admitted,jorge,command,"souvenir").await?;
        super::classic_quests::talk(owner,admitted,jorge,command+1,name).await?;
        let key=format!("oteryn:item.tibia.i{item}");
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
            .bind(&key).fetch_one(&h.pool).await?;
        assert_eq!(count,if i==offers.len() {2} else {1},"source souvenir: {name}");
        let tokens:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19083'")
            .fetch_one(&h.pool).await?;
        assert_eq!(tokens,0,"exact source cost for {name}");
    }
    let trades:i64=sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE side=3").fetch_one(&h.pool).await?;
    assert_eq!(trades,38);
    super::classic_quests::talk(owner,admitted,jorge,10122,&offers[0].1).await?;
    let replayed:i64=sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE side=3").fetch_one(&h.pool).await?;
    assert_eq!(replayed,trades);
