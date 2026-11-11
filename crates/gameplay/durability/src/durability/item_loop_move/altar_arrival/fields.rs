//! Fresh durable field operands, read under the same item fence as the arrival.
use super::*;
use crate::durability::spell_item_transaction::{
    SpellItemAuthority, read_spell_field_in_transaction,
};

/// An observed durable charge. Its UUID is only a lookup hint until re-read.
#[derive(Clone, Debug)]
pub struct AltarFieldRead {
    pub item: [u8; 16],
    pub spatial_position: Vec<u8>,
}

impl AltarFieldRead {
    pub fn new(item: [u8; 16], spatial_position: Vec<u8>) -> Self {
        Self {
            item,
            spatial_position,
        }
    }
}

pub(super) fn valid(target: &AltarArrivalTarget) -> bool {
    let Some(rule) = elemental_spheres::rules()
        .iter()
        .find(|r| r.key == target.rule)
    else {
        return false;
    };
    let mut items = std::collections::BTreeSet::new();
    let mut positions = std::collections::BTreeSet::new();
    target.field_reads.len() <= 8
        && target.field_reads.iter().all(|field| {
            field.item != [0; 16]
                && field.spatial_position.len() == 10
                && items.insert(field.item)
                && positions.insert(field.spatial_position.clone())
                && rule.positions.iter().any(|p| {
                    field.spatial_position
                        == [
                            i32::from(p.x).to_be_bytes().as_slice(),
                            i32::from(p.y).to_be_bytes().as_slice(),
                            (-i16::from(p.floor)).to_be_bytes().as_slice(),
                        ]
                        .concat()
                })
        })
}

pub(super) async fn read(
    tx: &mut Type2Transaction<'_>,
    authority: &SpellItemAuthority,
    ground: &LoopGround,
    target: &AltarArrivalTarget,
    content: [u8; 32],
) -> LoopResult<Option<Vec<serde_json::Value>>> {
    let Some(rule) = elemental_spheres::rules()
        .iter()
        .find(|r| r.key == target.rule)
    else {
        return refuse(LoopMoveRefusal::Rejected);
    };
    if target.field_reads.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
            .fetch_one(&mut ***tx)
            .await?;
    let mut operands: Vec<_> = target.field_reads.iter().collect();
    operands.sort_by(|a, b| a.spatial_position.cmp(&b.spatial_position));
    let mut proof = Vec::with_capacity(operands.len());
    for observed in operands {
        let at = SpellGroundTarget {
            spatial_position: observed.spatial_position.clone(),
            map_revision: ground.map_revision.clone(),
            content_revision: ground.content_revision.clone(),
            placement_context: ground.placement_context.clone(),
        };
        let field = read_spell_field_in_transaction(tx, authority, &at, observed.item)
            .await
            .map_err(spell_error)?;
        let Some(field) = field.filter(|field| {
            field.item_instance_id == observed.item
                && field.definition.family == "Item"
                && field.definition.production_key == rule.charged
                && field
                    .expires_at_unix_ms
                    .is_none_or(|deadline| deadline > now)
                && (field
                    .field_origin
                    .as_ref()
                    .is_some_and(|origin| origin.content_digest == content)
                    || field
                        .creature_field_origin
                        .as_ref()
                        .is_some_and(|origin| origin.content_digest == content))
        }) else {
            return Ok(None);
        };
        proof.push(json!({"item":field.item_instance_id,"definition":field.definition.production_key,"state_revision":field.state_revision,"position":observed.spatial_position,"expires_at":field.expires_at_unix_ms}));
    }
    Ok(Some(proof))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn field_operands_require_unique_exact_source_positions() {
        let rule = &elemental_spheres::rules()[0];
        let p = rule.positions[0];
        let position = [
            i32::from(p.x).to_be_bytes().as_slice(),
            i32::from(p.y).to_be_bytes().as_slice(),
            (-i16::from(p.floor)).to_be_bytes().as_slice(),
        ]
        .concat();
        let target = AltarArrivalTarget::new(rule.key, (1..=8).map(|n| (n, 0)).collect(), 0);
        assert!(
            target
                .clone()
                .with_field_reads(vec![AltarFieldRead::new([1; 16], position.clone())])
                .valid()
        );
        assert!(
            !target
                .clone()
                .with_field_reads(vec![AltarFieldRead::new([0; 16], position.clone())])
                .valid()
        );
        assert!(
            !target
                .clone()
                .with_field_reads(vec![
                    AltarFieldRead::new([1; 16], position.clone()),
                    AltarFieldRead::new([2; 16], position)
                ])
                .valid()
        );
        assert!(
            !target
                .with_field_reads(vec![AltarFieldRead::new([1; 16], vec![0; 10])])
                .valid()
        );
    }
}
