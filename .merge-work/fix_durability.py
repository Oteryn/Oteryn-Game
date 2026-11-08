from pathlib import Path
import sys
sys.path.insert(0,'.merge-work');from rust_structs import fix
p=Path('crates/gameplay/durability/src/durability/quest_state/writer.rs');old=p.read_text();s=Path('.merge-work/writer-main.rs').read_text()
# Retain main's transaction participant tuple and add the gate-aware sideC entry points.
start=old.index('    /// A content-bound world callback uses',old.index('pub async fn commit_character_world_callback_with_native('))
end=old.index('    /// World value grants',start)
wrapper=old[start:end].replace('Some((permit, context, native, admission))','Some((permit, context, native, None))')
# The compatibility API keeps its optional admitted participant; gate APIs reuse the same inner writer.
reward_start=s.index('    /// World value grants');reward_end=s.index("    /// Bloodshade",reward_start)
reward_wrapper=s[reward_start:reward_end].replace('pub(crate) async fn commit_character_world_reward','pub async fn commit_character_world_callback_with_reward_and_gate')
s=s[:reward_start]+wrapper+reward_wrapper+s[reward_start:]
# SideC requires read-only prerequisites and native party membership in the same transaction.
req_start=old.index('                    if let Some(requirements) = reward_callback')
req_end=old.index('                    if request.transition_key==',req_start)
req=old[req_start:req_end].replace('context.as_ref().and_then(|(_,_,native)|*native)','context.as_ref().and_then(|(_,_,native,_)|*native)').replace('reward_admission.as_ref().map(|admission| admission.native)','reward_admission.as_ref().map(|admission| admission.native).or_else(|| context.as_ref().and_then(|(_,_,_,admission)| admission.map(|a|a.native)))')
anchor='                    if request.transition_key==';pos=s.index(anchor);s=s[:pos]+req+s[pos:]
anchor='                    let id = character.as_bytes().as_slice();'
extra='''                    if let Some(callback) = &callback
                        && !super::super::quest_world_callback::completion_claims_are_current(&mut tx, fence, callback).await?
                    { return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))); }
                    if let (Some(callback), Some((_, context, _, _))) = (&callback, &context)
                        && !super::super::quest_world_callback::party_is_current(&mut tx, &node, fence, callback, context).await?
                    { return Ok(Ok(QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))); }
'''
s=s.replace(anchor,extra+anchor)
# Both authored item reward representations share the same MINT admission.
s=s.replace('c.effects.item_reward.as_ref()','c.item_reward()').replace('c.effects.item_reward.is_some()','c.item_reward().is_some()').replace('world.effects.item_reward.is_some()','world.item_reward().is_some()')
# Grant sideC achievements in the existing physical transaction before persisting the revision.
anchor='                    let committed_revision = root'
extra='''                    if let Some(world) = &callback && !world.achievements.is_empty() {
                        let (cause, ordinal) = request.cause.key();
                        let occurrence = serde_json::to_vec(&(cause, ordinal, &world.key)).map_err(|_| DurabilityError::InvalidStoredState)?;
                        let occurrence = sha2::Sha256::digest(&occurrence);
                        let keys: Vec<_> = world.achievements.iter().map(String::as_str).collect();
                        super::super::account_achievement::grant_missing(&mut tx, fence.character_id, &keys, "oteryn:quest-world-reward", &occurrence).await?;
                    }
'''
s=s.replace(anchor,extra+anchor)
p.write_text(s)
# Shared MINT helper: retain main's consumption/clock participant and sideC's bounded counter and key state.
p=Path('crates/gameplay/durability/src/durability/reward_claim_mint/quest.rs');old=p.read_text();s=old[:old.rindex('\npub(crate) async fn give_world(')]
s=s.replace('(request.cause, callback.effects.item_reward.as_ref())','(request.cause, callback.item_reward())')
s=s.replace('let claim_key = if callback.is_paid_exchange() {','let claim_key = if callback.counter.is_some() {\n        questpc7_counter_claim::key(&callback.key, command.game_session_id().as_bytes(), command.command_id().get())\n    } else if callback.repeatable || callback.is_paid_exchange() {')
s=s.replace('                key_number: None,\n                text: None,\n                charges_remaining: None,','                key_number: reward.key_number,\n                text: None,\n                charges_remaining: None,')
p.write_text(s)
p=Path('crates/gameplay/durability/src/durability/reward_claim_mint.rs');s=p.read_text().replace('    /// Source-resolved fluid subtype; no row means an empty container.\n','    /// Source-resolved fluid subtype; no row means an empty container.\n    pub fluid_subtype: Option<u8>,\n',1);p.write_text(s)
# Remove duplicate same-valued fields introduced by additive merges, preserving all initializers.
for root in ['crates/gameplay/durability','crates/gameplay/dispatch','apps/game-server']:
 for p in Path(root).rglob('*.rs'):
  s=p.read_text();n=fix(s,{'RewardClaimMintRequest':{},'RewardClaimMintContent':{'fluid_subtype':'None'}})
  if n!=s:p.write_text(n)
# Exchange uses its own offer owner and has no ordinary quest shop/travel offer.
for name in ['quest_shop','quest_access']:
 p=Path(f'crates/gameplay/durability/src/durability/npc_trade/{name}.rs');s=p.read_text().replace('        NpcTradeSide::Sell =>','        NpcTradeSide::Exchange => return Ok(false),\n        NpcTradeSide::Sell =>');p.write_text(s)
p=Path('crates/gameplay/quests/src/quest/death_objectives.rs');s=p.read_text();start=s.index('                || objective\n                    .guardian') if '                || objective\n                    .guardian' in s else -1
if start>=0:
 end=s.index('                || objective.sources',start);s=s[:start]+'                || objective.guardian.as_ref().is_some_and(|guardian| !guardian.valid_for(objective))\n'+s[end:]
p.write_text(s)
