//! Optional native-client proof export. Every frame is from a real transport epoch.
use super::*;
use crate::gameplay_transport::connection::AdmittedSession;

pub(super) fn core_use_payload(
    frames: &[Vec<u8>],
    intent: &UseIntent,
    definition: u32,
    equip_drop: bool,
) -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::{MessageType, decode_snapshot_chunk, decode_wire_envelope};
    let mut core = None;
    for frame in frames {
        let envelope = decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() != MessageType::SnapshotChunk {
            continue;
        }
        let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
        for domain in domains {
            if domain.domain_id
                != oteryn_protocol_oteryn::item_view::STATE_DOMAIN_CHARACTER_INVENTORY
            {
                continue;
            }
            let inventory =
                oteryn_protocol_oteryn::item_view::decode_character_inventory_with_equip_drop(
                    domain.payload,
                    equip_drop,
                )
                .map_err(debug)?;
            for entry in inventory.entries {
                if entry.item_definition_ref.get() == definition
                    && core.replace(entry.handle).is_some()
                {
                    return Err("ambiguous native inventory core".into());
                }
            }
        }
    }
    let mut intent = intent.clone();
    let handle = core.ok_or("actual initial inventory Soul Core handle")?;
    eprintln!(
        "SRX3_SOULPIT PREFLIGHT stage=native_inventory_handle handle={handle:?} definition={definition} equip_drop={equip_drop}"
    );
    intent.target = UseTarget::Item(handle);
    encode_item_use_intent(&intent).map_err(debug)
}

pub(super) struct WaveStage {
    pub wave: u8,
    pub admission: AdmittedSession,
    pub frames: Vec<Vec<u8>>,
    pub position: [i32; 3],
}

pub(super) struct NativeCapture {
    pub source: AdmittedSession,
    pub entry_command: Vec<u8>,
    pub entry_frames: Vec<Vec<u8>>,
    pub stages: Vec<WaveStage>,
    pub before_return: Option<AdmittedSession>,
    pub return_frames: Vec<Vec<u8>>,
    pub returned: Option<AdmittedSession>,
}
impl NativeCapture {
    pub async fn wave(
        &mut self,
        owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
        admitted: &mut AdmittedSession,
        wave: u8,
    ) -> TestResult {
        let admission = *admitted;
        let position = owner
            .observe(admitted.runtime_actor.ok_or("wave observer")?)
            .await
            .ok_or("actual wave observation")?
            .actor_position;
        let mut frames = Vec::new();
        super::super::boss_activity::capture_current_snapshot(owner, admitted, &mut frames).await?;
        self.stages.push(WaveStage {
            wave,
            admission,
            frames,
            position: [position.x, position.y, i32::from(position.floor)],
        });
        Ok(())
    }
    pub fn write(&self) -> TestResult {
        let Ok(path) = std::env::var("SRX3_SOULPIT_CAPTURE_PATH") else {
            return Ok(());
        };
        if !std::path::Path::new(&path).starts_with("/tmp") {
            return Err("native proof export must target /tmp".into());
        }
        use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope};
        let boundary = self
            .entry_frames
            .iter()
            .position(|raw| {
                decode_wire_envelope(raw)
                    .is_ok_and(|frame| frame.message_type() == MessageType::SnapshotCommit)
            })
            .ok_or("actual source initial boundary")?
            + 1;
        let encode = |raw: &[u8]| {
            raw.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        let metadata = |s: AdmittedSession| {
            serde_json::json!({
                "game_session_id":encode(s.game_session_id.as_bytes()),"world_id":encode(s.world_id.as_bytes()),"channel_id":encode(s.channel_id.as_bytes()),
                "connection_generation":s.continuity.connection_generation,"current_server_sequence":s.continuity.server_sequence,"next_command_id":s.continuity.next_command_id,
                "schema_revision":crate::gameplay_transport::connection::SERVER_SCHEMA_REVISION,"selected_capabilities":s.continuity.selected_capabilities.as_slice(),
            })
        };
        let last = self.stages.last().ok_or("actual native wave stages")?;
        let before_return = self.before_return.ok_or("actual return epoch admission")?;
        let returned = self.returned.ok_or("actual returned admission")?;
        let character = self
            .source
            .item_fence
            .ok_or("actual capture Character fence")?
            .character_id;
        let value = serde_json::json!({
            "schema":"OTERYN_NATIVE_SCOPE_REPLAY/v1","activity":"soulpit","character_id":encode(character.as_bytes()),
            "transport_epochs":{"entry":"one continuous source USE-to-Instance stream","updates":"each wave is a separate cold Instance observation stream","return":"one continuous Instance autonomous completion-to-Channel stream"},
            "source_admission":metadata(self.source),"update_admission":metadata(last.admission),"before_return_admission":metadata(before_return),"returned_admission":metadata(returned),
            "capabilities":self.source.continuity.selected_capabilities.as_slice(),
            "initial_frames":self.entry_frames[..boundary].iter().map(|raw|encode(raw)).collect::<Vec<_>>(),
            "entry_command":{"kind":oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT,"payload":encode(&self.entry_command)},
            "entry_frames":self.entry_frames[boundary..].iter().map(|raw|encode(raw)).collect::<Vec<_>>(),
            "update_frames":last.frames.iter().map(|raw|encode(raw)).collect::<Vec<_>>(),
            "wave_stages":self.stages.iter().map(|stage|serde_json::json!({"wave":stage.wave,"admission":metadata(stage.admission),"frames":stage.frames.iter().map(|raw|encode(raw)).collect::<Vec<_>>(),"position":stage.position})).collect::<Vec<_>>(),
            "return_command":null,"autonomous_return":true,"return_frames":self.return_frames.iter().map(|raw|encode(raw)).collect::<Vec<_>>(),
            "source_position":[32375,31158,-8],"destination_position":[32373,31151,-8],"return_position":[32373,31158,-8],
            "physical_runtime_positions":{"source":[32375,31158,8],"destination":[32373,31151,8],"return":[32373,31158,8]},"expected_status":["Accepted"],
        });
        std::fs::write(path, serde_json::to_vec_pretty(&value)?)?;
        Ok(())
    }
}
