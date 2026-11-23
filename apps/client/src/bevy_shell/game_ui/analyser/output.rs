//! Exports take their measurements from the activated analyser window.
use super::*;
use oteryn_client_ui::analyser::Panel;
use std::{
    io::{self, Write},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Event)]
pub(super) struct CopyReport(pub(super) String);

pub(super) fn copy(
    event: On<CopyReport>,
    mut clipboard: ResMut<bevy::clipboard::Clipboard>,
    owner: NonSend<HostOwner>,
) {
    if let Some(play) = &owner.play {
        let status = if clipboard.set_text(event.0.clone()).is_ok() {
            "Report copied"
        } else {
            "Clipboard unavailable"
        };
        play.link.analyser_feedback(status);
    }
}

#[derive(Resource, Default)]
pub(super) struct ExportTask(Option<(u64, bevy_tasks::Task<io::Result<()>>)>);
impl ExportTask {
    pub(super) fn submit(
        &mut self,
        generation: u64,
        panel: Panel,
        path: std::path::PathBuf,
        json: bool,
    ) -> bool {
        if self.0.is_some() {
            return false;
        }
        let seconds = panel.clock.seconds();
        let end = SystemTime::now();
        let language = oteryn_client_ui::localization::current();
        self.0 = Some((
            generation,
            bevy_tasks::IoTaskPool::get().spawn(async move {
                oteryn_client_ui::localization::with_language(language, || {
                    save(&panel, &path, json, seconds, end)
                })
            }),
        ));
        true
    }
}
pub(super) fn finish(mut job: ResMut<ExportTask>, owner: NonSend<HostOwner>) {
    let Some((generation, task)) = &mut job.0 else {
        return;
    };
    let Some(result) = bevy_tasks::futures::check_ready(task) else {
        return;
    };
    if *generation == owner.generation()
        && let Some(play) = &owner.play
    {
        play.link.analyser_feedback(if result.is_ok() {
            "Hunt report saved"
        } else {
            "Choose a new writable file path"
        });
    }
    job.0 = None;
}
fn save(panel: &Panel, path: &Path, json: bool, seconds: u64, end: SystemTime) -> io::Result<()> {
    let content = if json {
        let elapsed = seconds.saturating_sub(
            panel.reset_at[Tool::ALL
                .iter()
                .position(|tool| *tool == Tool::Hunting)
                .ok_or_else(|| io::Error::other("Hunting interval unavailable"))?],
        );
        let hourly = |value: u64| {
            (u128::from(value) * 3600)
                .checked_div(elapsed.into())
                .unwrap_or(0)
                .min(u128::from(u64::MAX)) as u64
        };
        let end = end
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_secs();
        let loot = panel.loot_value();
        let supplies = panel.supply_cost();
        let items: Vec<_> = panel.loot.iter().map(|(item, row)| serde_json::json!({"Item":item,"Name":panel.item_info.get(item).map(|info| &info.name),"Count":row.count})).collect();
        let kills: Vec<_> = panel
            .kills_by_race
            .iter()
            .map(|(race, count)| serde_json::json!({"Race":race,"Count":count}))
            .collect();
        serde_json::to_string_pretty(&serde_json::json!({
            "SessionStart":end.saturating_sub(elapsed),"SessionEnd":end,"SessionLength":elapsed,
            "Balance":(i128::from(loot) - i128::from(supplies)).to_string(),
            "Loot":loot,"Supplies":supplies,"KilledMonsters":kills,"LootedItems":items,
            "Damage":panel.damage,"DamageHour":hourly(panel.damage),
            "Healing":panel.healing,"HealingHour":hourly(panel.healing),
            "RawXPGain":panel.raw_xp,"RawXPGainHour":hourly(panel.raw_xp),
            "XPGain":panel.gained_xp,"XPGainHour":hourly(panel.gained_xp)
        }))?
    } else {
        oteryn_client_ui::analyser::session_text_at(panel, seconds)
    };
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.write_all(content.as_bytes())?;
    file.persist_noclobber(path)?;
    Ok(())
}
