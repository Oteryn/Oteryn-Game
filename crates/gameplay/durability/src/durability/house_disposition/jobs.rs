use super::*;

impl DurabilityRoot {
    /// Reload due work using a current execution Channel in the same World, without an actor.
    /// Each house commits separately; a native error aborts its whole transaction, without retry.
    pub async fn drain_house_dispositions(
        &self,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<()> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseDispositionError::Rejected)?;
        let discovery_recovery = recovery.clone();
        let discovery_node = node.clone();
        let p = scope_payload(scope, generation)?;
        let discovery_p = p.clone();
        let mut context = permit;
        let candidates = self
            .try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let mut tx =
                        super::super::spell_item_transaction::begin_spell_owner_transaction(
                            holder, deadline,
                        )
                        .await?;
                    super::super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                        &mut tx,
                        permit,
                        &discovery_recovery,
                        &discovery_node,
                        scope,
                        generation,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    let result = port(&mut tx, &discovery_node, "candidates", &discovery_p).await?;
                    let candidates = result
                        .as_array()
                        .ok_or(DurabilityError::InvalidStoredState)?
                        .iter()
                        .map(|op| {
                            op.as_str()
                                .map(str::to_owned)
                                .ok_or(DurabilityError::InvalidStoredState)
                        })
                        .collect::<std::result::Result<Vec<_>, _>>()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(candidates)
                })
            })
            .await?;
        let mut failure = None;
        for operation in candidates {
            let record = recovery.clone();
            let node = node.clone();
            let mut p = p.clone();
            p["operation"] = operation.clone().into();
            let result=self.try_issue_semantic_pass()?.run_with_context(&mut context,move |holder,deadline,permit|Box::pin(async move {
                let mut tx=super::super::spell_item_transaction::begin_spell_owner_transaction(holder,deadline).await?;
                super::super::spell_item_transaction::assert_spell_item_scope_with_recovery(&mut tx,permit,&record,&node,scope,generation)
                    .await.map_err(|_|DurabilityError::Unavailable)?;
                let changed=port(&mut tx,&node,"advance",&p).await?;
                if changed.as_bool()==Some(true) { audit::write(&mut tx,&node,&p,&operation).await?; }
                commit_semantic_transaction(tx,deadline).await?;
                Ok(())
            })).await;
            if let Err(error) = result {
                failure = Some(error);
            }
        }
        failure.map_or(Ok(()), |e| Err(e.into()))
    }
}
