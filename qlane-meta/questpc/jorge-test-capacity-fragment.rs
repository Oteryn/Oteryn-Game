    // A closed topic and a short or split token stack must never debit anything.
    drop_replica(owner,admitted,offers[0].0,21000).await?;
    super::classic_quests::talk(owner,admitted,jorge,11000,"abacus").await?;
    for (count,n) in [(19,500),(1,501)] { mint(owner,admitted,19083,count,n).await?; }
    super::classic_quests::talk(owner,admitted,jorge,11001,"souvenir").await?;
    let split=super::classic_quests::talk(owner,admitted,jorge,11002,"abacus").await?;
    assert!(split.iter().any(|line|line.contains("one stack")));
    assert_eq!(carried_tokens(h).await?,20);
    drop_replica(owner,admitted,19083,21001).await?;
    drop_replica(owner,admitted,19083,21002).await?;
    for n in 510..529 { mint(owner,admitted,3031,1,n).await?; }
    mint(owner,admitted,19083,20,530).await?;
    // A whole take frees its entry, so a20-entry backpack can accept the replica.
    super::classic_quests::talk(owner,admitted,jorge,11003,"souvenir").await?;
    super::classic_quests::talk(owner,admitted,jorge,11004,"abacus").await?;
    assert_eq!(carried_tokens(h).await?,0);
    drop_replica(owner,admitted,19151,21003).await?;
    mint(owner,admitted,19083,100,531).await?;
    super::classic_quests::talk(owner,admitted,jorge,11005,"souvenir").await?;
    let full=send(owner,admitted,jorge,11006,"abacus").await?;
    assert_eq!(full.disposition,oteryn_protocol_oteryn::npc_service::NpcIntentDisposition::Rejected);
    assert!(!full.unknown);
    assert_eq!(carried_tokens(h).await?,100,"a partial take needs a new entry; refusal must not burn20 tokens");
    drop_replica(owner,admitted,3031,21004).await?;
    super::classic_quests::talk(owner,admitted,jorge,11007,"abacus").await?;
    assert_eq!(carried_tokens(h).await?,80);
    let final_trades:i64=sqlx::query_scalar("SELECT count(*) FROM game_npc_trades WHERE side=3").fetch_one(&h.pool).await?;
    assert_eq!(final_trades,40);
    let final_experience:i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(final_experience,before_experience);
