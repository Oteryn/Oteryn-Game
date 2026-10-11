//! Stable contract for a native Windows settings-page audit.
//!
//! This module owns the exclusive CLI request, the audited page sequence, and
//! validation of native-renderer evidence. A valid manifest proves complete
//! viewport capture and non-mutation, not visual parity with private references.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fmt::{self, Display, Formatter},
    path::{Component, Path, PathBuf},
};

const SETTINGS_SCROLL_FRAME_ID: &str = "native-settings-audit-scroll-frame";
const SETTINGS_AUDIT_EXPAND_NESTED_ID: &str = "native-settings-audit-expand-nested";
const GEOMETRY_TOLERANCE: f32 = 1.0;
const GEOMETRY_SETTLE_TOLERANCE: f32 = 0.05;

#[derive(Clone, Copy, Debug)]
pub struct SettingsScrollFrame {
    pub page_id: &'static str,
    pub scroll_id: egui::Id,
    pub scroll_y: f32,
    pub inner_rect: egui::Rect,
    pub content_clip_rect: egui::Rect,
    pub content_size: egui::Vec2,
    completed_pass_nr: u64,
}

/// Records the production settings scroll area; audit code only observes this probe.
pub fn record_settings_scroll(
    context: &egui::Context,
    page_id: &'static str,
    content_clip_rect: egui::Rect,
    output: &egui::scroll_area::ScrollAreaOutput<()>,
) {
    // `cumulative_pass_nr` counts completed passes. This probe is recorded
    // during the current pass, so its output becomes observable after that
    // pass completes. Read it before taking the data lock below.
    let completed_pass_nr = context.cumulative_pass_nr().saturating_add(1);
    context.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(SETTINGS_SCROLL_FRAME_ID),
            SettingsScrollFrame {
                page_id,
                scroll_id: output.id,
                scroll_y: output.state.offset.y,
                inner_rect: output.inner_rect,
                // Egui's content clip can extend through panel margins. Keep it
                // only to identify shapes painted by this page; `inner_rect`
                // remains the authoritative fit and visibility boundary.
                content_clip_rect,
                content_size: output.content_size,
                completed_pass_nr,
            },
        );
    });
}

fn settings_scroll(context: &egui::Context) -> Option<SettingsScrollFrame> {
    let completed_pass_nr = context.cumulative_pass_nr();
    context.data(|data| {
        data.get_temp(egui::Id::new(SETTINGS_SCROLL_FRAME_ID))
            .filter(|frame: &SettingsScrollFrame| frame.completed_pass_nr == completed_pass_nr)
    })
}

fn expands_nested_settings_content(context: &egui::Context) -> bool {
    context.data(|data| {
        data.get_temp::<bool>(egui::Id::new(SETTINGS_AUDIT_EXPAND_NESTED_ID))
            .unwrap_or(false)
    })
}

/// Audit frames flatten bounded child lists so the driven outer scroll area
/// observes every production row. Normal frames retain the exact nested-scroll
/// composition.
pub fn show_bounded_vertical<R>(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    max_height: f32,
    max_width: Option<f32>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if expands_nested_settings_content(ui.ctx()) {
        add_contents(ui)
    } else {
        let mut scroll = egui::ScrollArea::vertical()
            .id_salt(id_salt)
            .max_height(max_height)
            .auto_shrink([false, false]);
        if let Some(max_width) = max_width {
            scroll = scroll.max_width(max_width);
        }
        scroll.show(ui, add_contents).inner
    }
}

pub const SETTINGS_AUDIT_ARGUMENT: &str = "--settings-audit";
pub const SETTINGS_AUDIT_SCHEMA: &str = "oteryn.settings-audit.v1";
pub const SETTINGS_AUDIT_MANIFEST_FILE: &str = "manifest.json";
pub const SETTINGS_AUDIT_TARGET: &str = "x86_64-pc-windows-msvc";
pub const SETTINGS_AUDIT_LOGICAL_SIZE: AuditDimensions = AuditDimensions {
    width: 900,
    height: 620,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceSettingsPage {
    pub id: &'static str,
    pub marker: &'static str,
}

const fn page(id: &'static str, marker: &'static str) -> ReferenceSettingsPage {
    ReferenceSettingsPage { id, marker }
}

/// Reference order and one visible English composition marker per audited page.
pub const REFERENCE_SETTINGS_PAGES: [ReferenceSettingsPage; 20] = [
    page("basic", "Gameplay"),
    page("controls", "Mouse Preset:"),
    page("general_hotkeys", "Auto-Switch Hotkey Preset"),
    page("action_hotkeys", "Bottom Action Bar: Action Button 1.01"),
    page("custom_hotkeys", "New Action"),
    page("interface", "Highlight Mouse Target"),
    page("hud", "Special conditions · 35"),
    page("console", "Show Info Messages"),
    page("game_window", "Show Textual Effects"),
    page("action_bars", "Show Bottom Action Bars:"),
    page("shortcuts", "Displayed Shortcuts:"),
    page("graphics", "Graphics Engine:"),
    page("effects", "Lighting"),
    page("sound", "Sound Device:"),
    page("battle_sounds", "Own Battle Sounds"),
    page("ui_sounds", "UI Volume: —"),
    page("miscellaneous", "Ask Before Buying Products"),
    page("gameplay", "Allow All to Inspect Me"),
    page("screenshots", "Only Capture Game Window"),
    page("help", "Client Help"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsAuditRequest {
    output_directory: PathBuf,
}

impl SettingsAuditRequest {
    #[must_use]
    pub fn output_directory(&self) -> &Path {
        &self.output_directory
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsAuditArgumentError {
    MissingOutputDirectory,
    AuditMustBeExclusive,
    OutputDirectoryMustBeAbsolute,
    OutputDirectoryMustNotBeRoot,
    OutputDirectoryMustBeNormalized,
}

impl Display for SettingsAuditArgumentError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingOutputDirectory => "--settings-audit requires one output directory",
            Self::AuditMustBeExclusive => "--settings-audit must be the only client mode",
            Self::OutputDirectoryMustBeAbsolute => "audit output must be an absolute path",
            Self::OutputDirectoryMustNotBeRoot => "audit output cannot be a filesystem root",
            Self::OutputDirectoryMustBeNormalized => "audit output cannot contain . or ..",
        })
    }
}

impl std::error::Error for SettingsAuditArgumentError {}

/// Parses arguments after the executable name. Other modes are untouched when
/// the audit switch is absent; audit mode itself is deliberately exclusive.
pub fn parse_settings_audit_request(
    arguments: impl IntoIterator<Item = OsString>,
) -> Result<Option<SettingsAuditRequest>, SettingsAuditArgumentError> {
    let arguments: Vec<_> = arguments.into_iter().collect();
    let switch = OsStr::new(SETTINGS_AUDIT_ARGUMENT);
    let count = arguments
        .iter()
        .filter(|argument| argument.as_os_str() == switch)
        .count();
    if count == 0 {
        return Ok(None);
    }
    if count > 1
        || arguments
            .first()
            .is_none_or(|arg| arg.as_os_str() != switch)
    {
        return Err(SettingsAuditArgumentError::AuditMustBeExclusive);
    }
    if arguments.len() < 2 || arguments[1].is_empty() {
        return Err(SettingsAuditArgumentError::MissingOutputDirectory);
    }
    if arguments.len() != 2 {
        return Err(SettingsAuditArgumentError::AuditMustBeExclusive);
    }
    let output_directory = PathBuf::from(&arguments[1]);
    if !output_directory.is_absolute() {
        return Err(SettingsAuditArgumentError::OutputDirectoryMustBeAbsolute);
    }
    if output_directory.file_name().is_none() {
        return Err(SettingsAuditArgumentError::OutputDirectoryMustNotBeRoot);
    }
    if output_directory
        .components()
        .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
    {
        return Err(SettingsAuditArgumentError::OutputDirectoryMustBeNormalized);
    }
    Ok(Some(SettingsAuditRequest { output_directory }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditFooterEvidence {
    pub reset: bool,
    pub ok: bool,
    pub apply: bool,
    pub cancel: bool,
}

impl AuditFooterEvidence {
    const fn complete(self) -> bool {
        self.reset && self.ok && self.apply && self.cancel
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsAuditCapture {
    pub index: u16,
    pub scroll_y: f32,
    pub file: String,
    pub sha256: String,
    pub dimensions: AuditDimensions,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsAuditPageEvidence {
    pub index: u8,
    pub id: String,
    pub marker: String,
    pub marker_visible: bool,
    /// Widest stable layout or painted right edge, measured from the page viewport's left edge.
    pub content_width: f32,
    pub content_height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub draft_unchanged: bool,
    pub footer: AuditFooterEvidence,
    pub captures: Vec<SettingsAuditCapture>,
}

/// Hashes provide artifact integrity only; they are intentionally not baselines.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsAuditManifest {
    pub schema: String,
    pub release_id: String,
    pub target: String,
    pub renderer: String,
    pub language: String,
    pub logical_size: AuditDimensions,
    pub physical_size: AuditDimensions,
    pub scale_factor: f32,
    pub pages: Vec<SettingsAuditPageEvidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsAuditValidationError {
    location: String,
    reason: &'static str,
}

impl SettingsAuditValidationError {
    fn new(location: impl Into<String>, reason: &'static str) -> Self {
        Self {
            location: location.into(),
            reason,
        }
    }
}

impl Display for SettingsAuditValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "settings audit {}: {}",
            self.location, self.reason
        )
    }
}

impl std::error::Error for SettingsAuditValidationError {}

impl SettingsAuditManifest {
    /// Validates completeness, order, scroll coverage, names, and integrity metadata.
    pub fn validate(&self) -> Result<(), SettingsAuditValidationError> {
        let invalid = |reason| SettingsAuditValidationError::new("manifest", reason);
        if self.schema != SETTINGS_AUDIT_SCHEMA {
            return Err(invalid("unknown schema"));
        }
        if self.release_id.is_empty()
            || self.release_id.trim() != self.release_id
            || self.release_id.len() > 256
        {
            return Err(invalid("invalid release identity"));
        }
        if self.target != SETTINGS_AUDIT_TARGET || self.renderer != "dx12" || self.language != "en"
        {
            return Err(invalid("unexpected native platform contract"));
        }
        if self.logical_size != SETTINGS_AUDIT_LOGICAL_SIZE
            || self.physical_size.width == 0
            || self.physical_size.height == 0
        {
            return Err(invalid("invalid viewport dimensions"));
        }
        if !self.scale_factor.is_finite() || !(0.5..=8.0).contains(&self.scale_factor) {
            return Err(invalid("invalid native scale factor"));
        }
        let expected = (
            self.logical_size.width as f32 * self.scale_factor,
            self.logical_size.height as f32 * self.scale_factor,
        );
        if (expected.0 - self.physical_size.width as f32).abs() > 2.0
            || (expected.1 - self.physical_size.height as f32).abs() > 2.0
        {
            return Err(invalid("logical, physical, and DPI dimensions disagree"));
        }
        if self.pages.len() != REFERENCE_SETTINGS_PAGES.len() {
            return Err(invalid("reference page count is incomplete"));
        }
        let mut files = BTreeSet::new();
        for (index, (evidence, reference)) in
            self.pages.iter().zip(REFERENCE_SETTINGS_PAGES).enumerate()
        {
            validate_page(self, evidence, reference, index, &mut files)?;
        }
        Ok(())
    }

    /// Reopens every listed capture and verifies that the manifest describes
    /// the exact regular WebP files present in `directory`.
    pub fn validate_artifacts(&self, directory: &Path) -> Result<(), SettingsAuditValidationError> {
        self.validate()?;
        for (page_index, page) in self.pages.iter().enumerate() {
            for (capture_index, capture) in page.captures.iter().enumerate() {
                validate_capture_artifact(directory, capture, page_index, capture_index)?;
            }
        }
        Ok(())
    }
}

fn write_manifest_create_new(
    directory: &Path,
    manifest: &SettingsAuditManifest,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    manifest.validate_artifacts(directory)?;
    let path = directory.join(SETTINGS_AUDIT_MANIFEST_FILE);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    serde_json::to_writer_pretty(&file, manifest)?;
    file.sync_all()?;
    Ok(())
}

fn validate_capture_artifact(
    directory: &Path,
    capture: &SettingsAuditCapture,
    page_index: usize,
    capture_index: usize,
) -> Result<(), SettingsAuditValidationError> {
    let invalid = |reason| {
        SettingsAuditValidationError::new(
            format!("page {page_index} capture {capture_index} artifact"),
            reason,
        )
    };
    let path = directory.join(&capture.file);
    let metadata = std::fs::symlink_metadata(&path)
        .map_err(|_| invalid("capture file cannot be inspected"))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(invalid("capture is not a regular non-symlink file"));
    }
    let encoded = std::fs::read(&path).map_err(|_| invalid("capture file cannot be read"))?;
    let actual_hash = {
        use sha2::{Digest, Sha256};
        lowercase_hex(&Sha256::digest(&encoded))
    };
    if actual_hash != capture.sha256 {
        return Err(invalid("capture SHA-256 does not match file bytes"));
    }
    let decoded = image::load_from_memory_with_format(&encoded, image::ImageFormat::WebP)
        .map_err(|_| invalid("capture is not a decodable WebP image"))?;
    if decoded.width() != capture.dimensions.width || decoded.height() != capture.dimensions.height
    {
        return Err(invalid("decoded capture dimensions differ from manifest"));
    }
    Ok(())
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(any(windows, test))]
#[derive(Debug)]
pub struct SettingsAuditRunError(String);

#[cfg(any(windows, test))]
impl Display for SettingsAuditRunError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(any(windows, test))]
impl std::error::Error for SettingsAuditRunError {}

#[cfg(any(windows, test))]
fn run_error(error: impl Display) -> SettingsAuditRunError {
    SettingsAuditRunError(error.to_string())
}

#[cfg(any(windows, test))]
#[derive(Clone, Copy)]
struct CaptureObservation {
    scroll: SettingsScrollFrame,
    painted_content_width: f32,
    marker_visible: bool,
    footer: AuditFooterEvidence,
    draft_unchanged: bool,
}

#[cfg(any(windows, test))]
#[derive(Clone, Copy)]
struct SettledGeometry {
    content_size: egui::Vec2,
    inner_rect: egui::Rect,
    content_clip_rect: egui::Rect,
}

#[cfg(any(windows, test))]
struct PendingPage {
    marker_visible: bool,
    footer: AuditFooterEvidence,
    draft_unchanged: bool,
    content_width: Option<f32>,
    content_height: Option<f32>,
    viewport_width: Option<f32>,
    viewport_height: Option<f32>,
    captures: Vec<SettingsAuditCapture>,
}

#[cfg(any(windows, test))]
impl PendingPage {
    const fn new() -> Self {
        Self {
            marker_visible: false,
            footer: AuditFooterEvidence {
                reset: true,
                ok: true,
                apply: true,
                cancel: true,
            },
            draft_unchanged: true,
            content_width: None,
            content_height: None,
            viewport_width: None,
            viewport_height: None,
            captures: Vec::new(),
        }
    }
}

/// Stateful driver for a bounded, fail-closed audit of the production native settings UI.
#[cfg(any(windows, test))]
pub struct NativeSettingsAudit {
    request: SettingsAuditRequest,
    baseline: oteryn_client::settings::ClientSettings,
    page_index: usize,
    desired_scroll: f32,
    settle_frames: u8,
    settled_geometry: Option<SettledGeometry>,
    stable_geometry_frames: u8,
    attempts: u16,
    pending_observation: Option<CaptureObservation>,
    pending_page: PendingPage,
    pages: Vec<SettingsAuditPageEvidence>,
    physical_size: Option<AuditDimensions>,
    scale_factor: Option<f32>,
}

#[cfg(any(windows, test))]
impl NativeSettingsAudit {
    pub fn start(
        request: SettingsAuditRequest,
        settings: &mut crate::settings_ui::SettingsPanel,
    ) -> Result<Self, SettingsAuditRunError> {
        if cfg!(windows) && !cfg!(all(target_arch = "x86_64", target_env = "msvc")) {
            return Err(run_error(
                "settings audit requires an x86_64-pc-windows-msvc build",
            ));
        }
        if cfg!(windows)
            && oteryn_client::RELEASE_ID
                == concat!(env!("CARGO_PKG_VERSION"), "+dev.local.g000000000000")
        {
            return Err(run_error(
                "settings audit requires an exact candidate release identity",
            ));
        }
        std::fs::create_dir(request.output_directory()).map_err(run_error)?;
        let baseline = settings.audit_draft().clone();
        if !baseline.english
            || baseline.fullscreen
            || baseline.window_width != SETTINGS_AUDIT_LOGICAL_SIZE.width as u16
            || baseline.window_height != SETTINGS_AUDIT_LOGICAL_SIZE.height as u16
            || (baseline.ui_scale - 1.0).abs() > f32::EPSILON
        {
            return Err(run_error(
                "settings audit defaults violate the viewport contract",
            ));
        }
        if !settings.open_complete_section(REFERENCE_SETTINGS_PAGES[0].id) {
            return Err(run_error("first settings audit page is unavailable"));
        }
        Ok(Self {
            request,
            baseline,
            page_index: 0,
            desired_scroll: 0.0,
            settle_frames: 3,
            settled_geometry: None,
            stable_geometry_frames: 0,
            attempts: 0,
            pending_observation: None,
            pending_page: PendingPage::new(),
            pages: Vec::with_capacity(REFERENCE_SETTINGS_PAGES.len()),
            physical_size: None,
            scale_factor: None,
        })
    }

    /// Applies the next requested scroll offset before production UI layout.
    pub fn prepare_frame(&mut self, context: &egui::Context) {
        context.data_mut(|data| {
            data.insert_temp(egui::Id::new(SETTINGS_AUDIT_EXPAND_NESTED_ID), true);
        });
        let reference = REFERENCE_SETTINGS_PAGES[self.page_index];
        if let Some(frame) = settings_scroll(context)
            && frame.page_id == reference.id
            && let Some(mut state) = egui::scroll_area::State::load(context, frame.scroll_id)
        {
            state.offset.y = self.desired_scroll;
            state.store(context, frame.scroll_id);
        }
    }

    /// Observes one real egui output and returns whether that exact frame must be captured.
    pub fn observe_frame(
        &mut self,
        context: &egui::Context,
        output: &egui::FullOutput,
        draft: &oteryn_client::settings::ClientSettings,
    ) -> Result<bool, SettingsAuditRunError> {
        self.attempts = self
            .attempts
            .checked_add(1)
            .ok_or_else(|| run_error("settings audit frame counter overflow"))?;
        if self.attempts > 180 {
            return Err(run_error(
                "settings audit page did not settle within 180 frames",
            ));
        }
        if draft != &self.baseline {
            return Err(run_error("settings UI mutated the isolated audit draft"));
        }
        let reference = REFERENCE_SETTINGS_PAGES[self.page_index];
        let Some(scroll) = settings_scroll(context).filter(|frame| frame.page_id == reference.id)
        else {
            return Ok(false);
        };
        if !scroll.scroll_y.is_finite()
            || !scroll.content_size.x.is_finite()
            || !scroll.content_size.y.is_finite()
            || scroll.content_size.x <= 0.0
            || scroll.content_size.y <= 0.0
            || !scroll.inner_rect.width().is_finite()
            || !scroll.inner_rect.height().is_finite()
            || scroll.inner_rect.width() <= 0.0
            || scroll.inner_rect.height() <= 0.0
            || !scroll.content_clip_rect.width().is_finite()
            || !scroll.content_clip_rect.height().is_finite()
            || scroll.content_clip_rect.width() <= 0.0
            || scroll.content_clip_rect.height() <= 0.0
        {
            return Err(run_error("settings audit received invalid scroll geometry"));
        }
        if self.settle_frames > 0 {
            self.settle_frames -= 1;
            context.request_repaint();
            return Ok(false);
        }
        let geometry = SettledGeometry {
            content_size: scroll.content_size,
            inner_rect: scroll.inner_rect,
            content_clip_rect: scroll.content_clip_rect,
        };
        if self
            .settled_geometry
            .is_some_and(|previous| geometry_approximately_equal(previous, geometry))
        {
            self.stable_geometry_frames = self.stable_geometry_frames.saturating_add(1);
        } else {
            self.stable_geometry_frames = 0;
        }
        self.settled_geometry = Some(geometry);
        if self.stable_geometry_frames < 2 {
            context.request_repaint();
            return Ok(false);
        }
        if (scroll.scroll_y - self.desired_scroll).abs() > 1.0 {
            context.request_repaint();
            return Ok(false);
        }
        let viewport = context.content_rect();
        self.pending_observation = Some(CaptureObservation {
            scroll,
            painted_content_width: painted_content_width(
                output,
                scroll.content_clip_rect,
                scroll.inner_rect.left(),
            )
            .ok_or_else(|| run_error("settings audit found no painted page content"))?,
            marker_visible: output_has_visible_text(output, reference.marker, scroll.inner_rect),
            footer: AuditFooterEvidence {
                reset: output_has_visible_text(output, "Reset", viewport),
                ok: output_has_visible_text(output, "OK", viewport),
                apply: output_has_visible_text(output, "Apply", viewport),
                cancel: output_has_visible_text(output, "Cancel", viewport),
            },
            draft_unchanged: true,
        });
        Ok(true)
    }

    /// Persists a lossless frame, advances scroll/page state, and writes the final manifest.
    pub fn accept_capture(
        &mut self,
        context: &egui::Context,
        settings: &mut crate::settings_ui::SettingsPanel,
        frame: oteryn_renderer::CapturedFrame,
        scale_factor: f32,
    ) -> Result<bool, SettingsAuditRunError> {
        let observation = self
            .pending_observation
            .take()
            .ok_or_else(|| run_error("renderer returned an unsolicited audit capture"))?;
        let physical_size = AuditDimensions {
            width: frame.width,
            height: frame.height,
        };
        if let Some(expected) = self.physical_size {
            if expected != physical_size {
                return Err(run_error("audit surface dimensions changed during capture"));
            }
        } else {
            self.physical_size = Some(physical_size);
            self.scale_factor = Some(scale_factor);
        }
        if self.scale_factor != Some(scale_factor) {
            return Err(run_error(
                "native scale factor changed during settings audit",
            ));
        }
        let reference = REFERENCE_SETTINGS_PAGES[self.page_index];
        let capture_index = self.pending_page.captures.len();
        let file = format!(
            "{:02}-{}-{capture_index:02}.webp",
            self.page_index, reference.id
        );
        let mut encoded = Vec::new();
        image::ImageEncoder::write_image(
            image::codecs::webp::WebPEncoder::new_lossless(&mut encoded),
            &frame.rgba,
            frame.width,
            frame.height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(run_error)?;
        let hash = {
            use sha2::{Digest, Sha256};
            lowercase_hex(&Sha256::digest(&encoded))
        };
        let artifact_path = self.request.output_directory().join(&file);
        let mut artifact = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(artifact_path)
            .map_err(run_error)?;
        std::io::Write::write_all(&mut artifact, &encoded).map_err(run_error)?;
        artifact.sync_all().map_err(run_error)?;
        self.pending_page.captures.push(SettingsAuditCapture {
            index: u16::try_from(capture_index).map_err(run_error)?,
            scroll_y: observation.scroll.scroll_y,
            file,
            sha256: hash,
            dimensions: physical_size,
        });
        self.pending_page.marker_visible |= observation.marker_visible;
        self.pending_page.footer.reset &= observation.footer.reset;
        self.pending_page.footer.ok &= observation.footer.ok;
        self.pending_page.footer.apply &= observation.footer.apply;
        self.pending_page.footer.cancel &= observation.footer.cancel;
        self.pending_page.draft_unchanged &= observation.draft_unchanged;
        let content_width = observation
            .painted_content_width
            .max(observation.scroll.content_size.x);
        let content_height = observation.scroll.content_size.y;
        let viewport_width = observation.scroll.inner_rect.width();
        let viewport_height = observation.scroll.inner_rect.height();
        keep_worst_horizontal_fit(
            &mut self.pending_page.content_width,
            &mut self.pending_page.viewport_width,
            content_width,
            viewport_width,
        );
        check_stable_dimension(
            "settings content height",
            &mut self.pending_page.content_height,
            content_height,
        )?;
        check_stable_dimension(
            "settings viewport height",
            &mut self.pending_page.viewport_height,
            viewport_height,
        )?;
        let maximum = (content_height - viewport_height).max(0.0);
        if observation.scroll.scroll_y < maximum - 1.0 {
            self.desired_scroll =
                (observation.scroll.scroll_y + viewport_height * 0.9).min(maximum);
            self.settle_frames = 2;
            self.settled_geometry = None;
            self.stable_geometry_frames = 0;
            self.attempts = 0;
            self.prepare_frame(context);
            return Ok(false);
        }
        self.finish_page(settings)?;
        if self.page_index < REFERENCE_SETTINGS_PAGES.len() {
            self.prepare_frame(context);
            return Ok(false);
        }
        let manifest = SettingsAuditManifest {
            schema: SETTINGS_AUDIT_SCHEMA.into(),
            release_id: oteryn_client::RELEASE_ID.into(),
            target: SETTINGS_AUDIT_TARGET.into(),
            renderer: "dx12".into(),
            language: "en".into(),
            logical_size: SETTINGS_AUDIT_LOGICAL_SIZE,
            physical_size: self
                .physical_size
                .ok_or_else(|| run_error("settings audit captured no native surface"))?,
            scale_factor: self
                .scale_factor
                .ok_or_else(|| run_error("settings audit captured no DPI scale"))?,
            pages: std::mem::take(&mut self.pages),
        };
        write_manifest_create_new(self.request.output_directory(), &manifest).map_err(run_error)?;
        Ok(true)
    }

    fn finish_page(
        &mut self,
        settings: &mut crate::settings_ui::SettingsPanel,
    ) -> Result<(), SettingsAuditRunError> {
        let reference = REFERENCE_SETTINGS_PAGES[self.page_index];
        let pending = std::mem::replace(&mut self.pending_page, PendingPage::new());
        self.pages.push(SettingsAuditPageEvidence {
            index: u8::try_from(self.page_index).map_err(run_error)?,
            id: reference.id.into(),
            marker: reference.marker.into(),
            marker_visible: pending.marker_visible,
            content_width: pending
                .content_width
                .ok_or_else(|| run_error("settings page has no content width"))?,
            content_height: pending
                .content_height
                .ok_or_else(|| run_error("settings page has no content geometry"))?,
            viewport_width: pending
                .viewport_width
                .ok_or_else(|| run_error("settings page has no viewport width"))?,
            viewport_height: pending
                .viewport_height
                .ok_or_else(|| run_error("settings page has no viewport geometry"))?,
            draft_unchanged: pending.draft_unchanged,
            footer: pending.footer,
            captures: pending.captures,
        });
        self.page_index += 1;
        self.desired_scroll = 0.0;
        self.settle_frames = 3;
        self.settled_geometry = None;
        self.stable_geometry_frames = 0;
        self.attempts = 0;
        if let Some(next) = REFERENCE_SETTINGS_PAGES.get(self.page_index)
            && !settings.open_complete_section(next.id)
        {
            return Err(run_error("next settings audit page is unavailable"));
        }
        Ok(())
    }
}

#[cfg(any(windows, test))]
fn check_stable_dimension(
    label: &str,
    stored: &mut Option<f32>,
    received: f32,
) -> Result<(), SettingsAuditRunError> {
    if let Some(expected) = *stored
        && (expected - received).abs() > GEOMETRY_TOLERANCE
    {
        return Err(run_error(format!(
            "{label} changed between captures: expected {expected}, received {received}",
        )));
    }
    stored.get_or_insert(received);
    Ok(())
}

#[cfg(any(windows, test))]
fn keep_worst_horizontal_fit(
    stored_content: &mut Option<f32>,
    stored_viewport: &mut Option<f32>,
    content: f32,
    viewport: f32,
) {
    let received_overflow = content - viewport;
    let stored_overflow = stored_content
        .zip(*stored_viewport)
        .map(|(content, viewport)| content - viewport);
    if stored_overflow.is_none_or(|stored| received_overflow > stored) {
        *stored_content = Some(content);
        *stored_viewport = Some(viewport);
    }
}

#[cfg(any(windows, test))]
fn geometry_approximately_equal(left: SettledGeometry, right: SettledGeometry) -> bool {
    (left.content_size.x - right.content_size.x).abs() <= GEOMETRY_SETTLE_TOLERANCE
        && (left.content_size.y - right.content_size.y).abs() <= GEOMETRY_SETTLE_TOLERANCE
        && (left.inner_rect.min.x - right.inner_rect.min.x).abs() <= GEOMETRY_SETTLE_TOLERANCE
        && (left.inner_rect.min.y - right.inner_rect.min.y).abs() <= GEOMETRY_SETTLE_TOLERANCE
        && (left.inner_rect.max.x - right.inner_rect.max.x).abs() <= GEOMETRY_SETTLE_TOLERANCE
        && (left.inner_rect.max.y - right.inner_rect.max.y).abs() <= GEOMETRY_SETTLE_TOLERANCE
        && (left.content_clip_rect.min.x - right.content_clip_rect.min.x).abs()
            <= GEOMETRY_SETTLE_TOLERANCE
        && (left.content_clip_rect.min.y - right.content_clip_rect.min.y).abs()
            <= GEOMETRY_SETTLE_TOLERANCE
        && (left.content_clip_rect.max.x - right.content_clip_rect.max.x).abs()
            <= GEOMETRY_SETTLE_TOLERANCE
        && (left.content_clip_rect.max.y - right.content_clip_rect.max.y).abs()
            <= GEOMETRY_SETTLE_TOLERANCE
}

#[cfg(any(windows, test))]
fn painted_content_width(
    output: &egui::FullOutput,
    page_clip: egui::Rect,
    page_left: f32,
) -> Option<f32> {
    output
        .shapes
        .iter()
        // Nested controls can own smaller, intentional clip rectangles. Only
        // shapes painted through the production page viewport establish page
        // fit; layout cursor width alone is not visible content.
        .filter(|shape| rect_approximately_equal(shape.clip_rect, page_clip))
        .map(|shape| shape.shape.visual_bounding_rect().right() - page_left)
        .filter(|width| width.is_finite() && *width > 0.0)
        .reduce(f32::max)
}

#[cfg(any(windows, test))]
fn rect_approximately_equal(left: egui::Rect, right: egui::Rect) -> bool {
    (left.min.x - right.min.x).abs() <= GEOMETRY_TOLERANCE
        && (left.min.y - right.min.y).abs() <= GEOMETRY_TOLERANCE
        && (left.max.x - right.max.x).abs() <= GEOMETRY_TOLERANCE
        && (left.max.y - right.max.y).abs() <= GEOMETRY_TOLERANCE
}

#[cfg(any(windows, test))]
fn output_has_visible_text(output: &egui::FullOutput, needle: &str, viewport: egui::Rect) -> bool {
    fn shape_has_text(shape: &egui::epaint::Shape, needle: &str, visible: egui::Rect) -> bool {
        match shape {
            egui::epaint::Shape::Text(text) => {
                text.galley.job.text == needle && visible.contains_rect(text.visual_bounding_rect())
            }
            egui::epaint::Shape::Vec(shapes) => shapes
                .iter()
                .any(|shape| shape_has_text(shape, needle, visible)),
            _ => false,
        }
    }
    output.shapes.iter().any(|clipped| {
        shape_has_text(
            &clipped.shape,
            needle,
            clipped.clip_rect.intersect(viewport),
        )
    })
}

fn validate_page<'a>(
    manifest: &SettingsAuditManifest,
    evidence: &'a SettingsAuditPageEvidence,
    reference: ReferenceSettingsPage,
    index: usize,
    files: &mut BTreeSet<&'a str>,
) -> Result<(), SettingsAuditValidationError> {
    let invalid = |reason| SettingsAuditValidationError::new(format!("page {index}"), reason);
    if usize::from(evidence.index) != index
        || evidence.id != reference.id
        || evidence.marker != reference.marker
    {
        return Err(invalid(
            "identifier, marker, or order differs from contract",
        ));
    }
    if !evidence.marker_visible || !evidence.draft_unchanged || !evidence.footer.complete() {
        return Err(invalid(
            "marker, non-mutation, or fixed-footer evidence is incomplete",
        ));
    }
    if !evidence.content_width.is_finite()
        || !evidence.content_height.is_finite()
        || !evidence.viewport_width.is_finite()
        || !evidence.viewport_height.is_finite()
        || evidence.content_width <= 0.0
        || evidence.content_height <= 0.0
        || evidence.viewport_width <= 0.0
        || evidence.viewport_height <= 0.0
        || evidence.captures.is_empty()
    {
        return Err(invalid("invalid scroll geometry or empty capture set"));
    }
    if evidence.content_width > evidence.viewport_width + GEOMETRY_TOLERANCE {
        return Err(invalid(
            "page content overflows the audited viewport horizontally",
        ));
    }
    let maximum = (evidence.content_height - evidence.viewport_height).max(0.0);
    if maximum <= 1.0 && evidence.captures.len() != 1 {
        return Err(invalid("an unscrolled page must have exactly one capture"));
    }
    for (capture_index, capture) in evidence.captures.iter().enumerate() {
        validate_capture(manifest, capture, index, capture_index, maximum, files)?;
        if capture_index > 0 {
            let previous = evidence.captures[capture_index - 1].scroll_y;
            if capture.scroll_y <= previous + 0.5
                || capture.scroll_y - previous > evidence.viewport_height + 1.0
            {
                return Err(invalid("capture offsets overlap backwards or leave a gap"));
            }
        }
    }
    if evidence.captures[0].scroll_y.abs() > 1.0
        || (maximum > 1.0
            && evidence
                .captures
                .last()
                .is_none_or(|capture| capture.scroll_y < maximum - 1.0))
    {
        return Err(invalid("captures do not cover the page from top to bottom"));
    }
    Ok(())
}

fn validate_capture<'a>(
    manifest: &SettingsAuditManifest,
    capture: &'a SettingsAuditCapture,
    page_index: usize,
    index: usize,
    maximum_scroll: f32,
    files: &mut BTreeSet<&'a str>,
) -> Result<(), SettingsAuditValidationError> {
    let invalid = |reason| {
        SettingsAuditValidationError::new(format!("page {page_index} capture {index}"), reason)
    };
    if usize::from(capture.index) != index
        || !capture.scroll_y.is_finite()
        || capture.scroll_y < 0.0
        || capture.scroll_y > maximum_scroll + 2.0
    {
        return Err(invalid("invalid index or scroll position"));
    }
    if capture.dimensions != manifest.physical_size {
        return Err(invalid("capture dimensions differ from native surface"));
    }
    let path = Path::new(&capture.file);
    let mut parts = path.components();
    if !matches!(parts.next(), Some(Component::Normal(_)))
        || parts.next().is_some()
        || path.extension() != Some(OsStr::new("webp"))
        || path.file_stem().is_none_or(OsStr::is_empty)
        || !files.insert(&capture.file)
    {
        return Err(invalid("artifact filename is unsafe or duplicated"));
    }
    if capture.sha256.len() != 64
        || !capture
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("SHA-256 is not lowercase hexadecimal"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    fn valid_manifest() -> SettingsAuditManifest {
        let pages = REFERENCE_SETTINGS_PAGES
            .iter()
            .enumerate()
            .map(|(index, reference)| SettingsAuditPageEvidence {
                index: index as u8,
                id: reference.id.into(),
                marker: reference.marker.into(),
                marker_visible: true,
                content_width: 700.0,
                content_height: 700.0,
                viewport_width: 700.0,
                viewport_height: 500.0,
                draft_unchanged: true,
                footer: AuditFooterEvidence {
                    reset: true,
                    ok: true,
                    apply: true,
                    cancel: true,
                },
                captures: [0.0, 200.0]
                    .into_iter()
                    .enumerate()
                    .map(|(capture_index, scroll_y)| SettingsAuditCapture {
                        index: capture_index as u16,
                        scroll_y,
                        file: format!("{index:02}-{}-{capture_index}.webp", reference.id),
                        sha256: "a".repeat(64),
                        dimensions: SETTINGS_AUDIT_LOGICAL_SIZE,
                    })
                    .collect(),
            })
            .collect();
        SettingsAuditManifest {
            schema: SETTINGS_AUDIT_SCHEMA.into(),
            release_id: "0.1.0+dev.ci.g0123456789ab".into(),
            target: SETTINGS_AUDIT_TARGET.into(),
            renderer: "dx12".into(),
            language: "en".into(),
            logical_size: SETTINGS_AUDIT_LOGICAL_SIZE,
            physical_size: SETTINGS_AUDIT_LOGICAL_SIZE,
            scale_factor: 1.0,
            pages,
        }
    }

    fn temporary_directory(tag: &str) -> std::io::Result<PathBuf> {
        let serial = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "oteryn-settings-audit-{tag}-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir(&directory)?;
        Ok(directory)
    }

    fn encode_test_webp(
        dimensions: AuditDimensions,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        let pixels = u64::from(dimensions.width)
            .checked_mul(u64::from(dimensions.height))
            .and_then(|value| value.checked_mul(4))
            .ok_or_else(|| std::io::Error::other("test image dimensions overflow"))?;
        let rgba = vec![0_u8; usize::try_from(pixels)?];
        let mut encoded = Vec::new();
        image::ImageEncoder::write_image(
            image::codecs::webp::WebPEncoder::new_lossless(&mut encoded),
            &rgba,
            dimensions.width,
            dimensions.height,
            image::ExtendedColorType::Rgba8,
        )?;
        Ok(encoded)
    }

    fn test_hash(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        lowercase_hex(&Sha256::digest(bytes))
    }

    fn materialize_artifacts(
        directory: &Path,
        manifest: &mut SettingsAuditManifest,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let encoded = encode_test_webp(manifest.physical_size)?;
        let hash = test_hash(&encoded);
        for page in &mut manifest.pages {
            for capture in &mut page.captures {
                capture.sha256.clone_from(&hash);
                std::fs::write(directory.join(&capture.file), &encoded)?;
            }
        }
        Ok(())
    }

    fn rendered_page_geometry(
        section: &str,
        expand_nested: bool,
    ) -> Result<SettingsScrollFrame, &'static str> {
        let context = egui::Context::default();
        crate::client_chrome::install(&context, false);
        if expand_nested {
            context.data_mut(|data| {
                data.insert_temp(egui::Id::new(SETTINGS_AUDIT_EXPAND_NESTED_ID), true);
            });
        }
        let mut browser = crate::preferences_browser::PreferencesBrowser::new();
        if !browser.open_section(section) {
            return Err("missing settings page");
        }
        let mut draft = oteryn_client::settings::ClientSettings {
            english: true,
            ..Default::default()
        };
        for _ in 0..4 {
            let _ = crate::preferences_browser::test_support::frame(
                &context,
                egui::vec2(900.0, 620.0),
                Vec::new(),
                |context| browser.show(context, &mut draft, None),
            );
        }
        settings_scroll(&context).ok_or("missing settings scroll geometry")
    }

    #[test]
    fn page_contract_is_unique_and_parser_is_exclusive() -> Result<(), Box<dyn std::error::Error>> {
        let ids: BTreeSet<_> = REFERENCE_SETTINGS_PAGES
            .iter()
            .map(|entry| entry.id)
            .collect();
        assert_eq!(ids.len(), 20);
        assert_eq!(parse_settings_audit_request(args(&["--smoke"]))?, None);
        let output = std::env::current_dir()?.join("native-settings-audit-evidence");
        let request = parse_settings_audit_request(vec![
            SETTINGS_AUDIT_ARGUMENT.into(),
            output.clone().into_os_string(),
        ])?;
        assert_eq!(
            request.as_ref().map(SettingsAuditRequest::output_directory),
            Some(output.as_path())
        );
        assert_eq!(
            parse_settings_audit_request(args(&[SETTINGS_AUDIT_ARGUMENT, "relative", "--smoke"])),
            Err(SettingsAuditArgumentError::AuditMustBeExclusive)
        );
        assert_eq!(
            parse_settings_audit_request(args(&[SETTINGS_AUDIT_ARGUMENT, "relative"])),
            Err(SettingsAuditArgumentError::OutputDirectoryMustBeAbsolute)
        );
        let filesystem_root = std::env::current_dir()?
            .ancestors()
            .last()
            .ok_or("current directory has no filesystem root")?
            .to_path_buf();
        assert_eq!(
            parse_settings_audit_request(vec![
                SETTINGS_AUDIT_ARGUMENT.into(),
                filesystem_root.into_os_string(),
            ]),
            Err(SettingsAuditArgumentError::OutputDirectoryMustNotBeRoot)
        );
        let path = std::env::current_dir()?.join("parent/../audit");
        assert_eq!(
            parse_settings_audit_request(vec![
                SETTINGS_AUDIT_ARGUMENT.into(),
                path.into_os_string(),
            ]),
            Err(SettingsAuditArgumentError::OutputDirectoryMustBeNormalized)
        );
        Ok(())
    }

    #[test]
    fn scroll_probe_only_describes_the_just_completed_ui_pass() {
        let context = egui::Context::default();
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 620.0),
            )),
            ..Default::default()
        };
        let _ = context.run_ui(input(), |ui| {
            let context = ui.ctx().clone();
            egui::CentralPanel::default().show(ui, |ui| {
                let mut viewport = egui::Rect::NOTHING;
                let output = egui::ScrollArea::vertical()
                    .id_salt("audit-probe-test")
                    .show(ui, |ui| {
                        viewport = ui.clip_rect();
                        ui.label("page body");
                    });
                record_settings_scroll(&context, "basic", viewport, &output);
            });
        });
        assert_eq!(
            settings_scroll(&context).map(|frame| frame.page_id),
            Some("basic")
        );

        let _ = context.run_ui(input(), |_ui| {});
        assert!(settings_scroll(&context).is_none());
    }

    #[test]
    fn horizontal_fit_keeps_the_worst_capture_pair() {
        let mut content = None;
        let mut viewport = None;
        keep_worst_horizontal_fit(&mut content, &mut viewport, 640.0, 630.0);
        keep_worst_horizontal_fit(&mut content, &mut viewport, 635.0, 634.0);
        assert_eq!((content, viewport), (Some(640.0), Some(630.0)));

        keep_worst_horizontal_fit(&mut content, &mut viewport, 646.5, 634.0);
        assert_eq!((content, viewport), (Some(646.5), Some(634.0)));
    }

    #[test]
    fn geometry_settle_rejects_scrollbar_animation_drift() {
        let baseline = SettledGeometry {
            content_size: egui::vec2(640.0, 900.0),
            inner_rect: egui::Rect::from_min_size(egui::pos2(20.0, 30.0), egui::vec2(630.0, 500.0)),
            content_clip_rect: egui::Rect::from_min_size(
                egui::pos2(10.0, 27.0),
                egui::vec2(650.0, 506.0),
            ),
        };
        let mut close = baseline;
        close.content_size.x += GEOMETRY_SETTLE_TOLERANCE / 2.0;
        assert!(geometry_approximately_equal(baseline, close));

        let mut drifting = baseline;
        drifting.inner_rect.max.x += GEOMETRY_SETTLE_TOLERANCE * 2.0;
        assert!(!geometry_approximately_equal(baseline, drifting));
    }

    #[test]
    fn painted_width_ignores_cursor_only_layout_overflow_but_detects_shapes()
    -> Result<(), &'static str> {
        fn render(overdraw: f32) -> (egui::FullOutput, egui::Rect) {
            let context = egui::Context::default();
            let viewport = std::cell::Cell::new(egui::Rect::NOTHING);
            let output = context.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(320.0, 120.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default()
                        .frame(egui::Frame::new().inner_margin(12))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                let clip = ui.clip_rect();
                                viewport.set(clip);
                                ui.label("painted page content");
                                // Layout cursor extent is not visual content and must not
                                // manufacture a horizontal-overflow failure.
                                ui.allocate_space(egui::vec2(clip.width() + 40.0, 1.0));
                                if overdraw > 0.0 {
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_max(
                                            clip.left_top(),
                                            egui::pos2(clip.right() + overdraw, clip.top() + 4.0),
                                        ),
                                        0.0,
                                        egui::Color32::WHITE,
                                    );
                                } else {
                                    let nested_clip = clip.shrink(16.0);
                                    ui.painter().with_clip_rect(nested_clip).rect_filled(
                                        egui::Rect::from_min_max(
                                            nested_clip.left_top(),
                                            egui::pos2(
                                                clip.right() + 40.0,
                                                nested_clip.top() + 4.0,
                                            ),
                                        ),
                                        0.0,
                                        egui::Color32::WHITE,
                                    );
                                }
                            });
                        });
                },
            );
            (output, viewport.get())
        }

        let (output, viewport) = render(0.0);
        let width = painted_content_width(&output, viewport, viewport.left())
            .ok_or("missing painted label")?;
        assert!(width <= viewport.width() + GEOMETRY_TOLERANCE);

        let (output, viewport) = render(12.0);
        let width = painted_content_width(&output, viewport, viewport.left())
            .ok_or("missing painted rectangle")?;
        assert!(width >= viewport.width() + 11.0);
        Ok(())
    }

    #[test]
    fn every_audited_page_paints_within_the_native_viewport() -> Result<(), &'static str> {
        for reference in REFERENCE_SETTINGS_PAGES {
            let context = egui::Context::default();
            crate::client_chrome::install(&context, false);
            context.data_mut(|data| {
                data.insert_temp(egui::Id::new(SETTINGS_AUDIT_EXPAND_NESTED_ID), true);
            });
            let mut browser = crate::preferences_browser::PreferencesBrowser::new();
            if !browser.open_section(reference.id) {
                return Err("missing settings page");
            }
            let mut draft = oteryn_client::settings::ClientSettings {
                english: true,
                ..Default::default()
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..14 {
                output = crate::preferences_browser::test_support::frame(
                    &context,
                    egui::vec2(900.0, 620.0),
                    Vec::new(),
                    |context| browser.show(context, &mut draft, None),
                )
                .1;
            }
            let scroll = settings_scroll(&context).ok_or("missing settings scroll geometry")?;
            assert!(
                scroll.content_size.x <= scroll.inner_rect.width() + GEOMETRY_TOLERANCE,
                "{} has stable horizontal layout overflow: {} in {}",
                reference.id,
                scroll.content_size.x,
                scroll.inner_rect.width(),
            );
            let width =
                painted_content_width(&output, scroll.content_clip_rect, scroll.inner_rect.left())
                    .ok_or("missing painted settings content")?;
            assert!(
                width <= scroll.inner_rect.width() + GEOMETRY_TOLERANCE,
                "{} paints beyond the native page viewport: {width} in {}",
                reference.id,
                scroll.inner_rect.width(),
            );
        }
        Ok(())
    }

    #[test]
    fn audit_expands_each_bounded_settings_list_into_the_outer_scroll() -> Result<(), &'static str>
    {
        for page in ["hud", "shortcuts"] {
            let normal = rendered_page_geometry(page, false)?;
            let expanded = rendered_page_geometry(page, true)?;
            assert!(
                expanded.content_size.y > normal.content_size.y + 100.0,
                "audit did not flatten the nested list on {page}"
            );
        }
        Ok(())
    }

    #[test]
    fn text_evidence_requires_the_entire_visual_bounds_to_be_visible() {
        let context = egui::Context::default();
        let size = egui::vec2(240.0, 80.0);
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.add_space(80.0);
                        ui.label("fully-visible-marker");
                    });
                });
            },
        );
        assert!(output_has_visible_text(
            &output,
            "fully-visible-marker",
            egui::Rect::from_min_size(egui::Pos2::ZERO, size),
        ));
        assert!(!output_has_visible_text(
            &output,
            "fully-visible-marker",
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(90.0, 80.0)),
        ));
    }

    #[test]
    fn complete_manifest_round_trips_and_validates() -> Result<(), Box<dyn std::error::Error>> {
        let manifest = valid_manifest();
        manifest.validate()?;
        let decoded: SettingsAuditManifest =
            serde_json::from_slice(&serde_json::to_vec(&manifest)?)?;
        assert_eq!(decoded, manifest);
        decoded.validate()?;
        Ok(())
    }

    #[test]
    fn manifest_file_is_create_new_and_reads_back()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let directory = temporary_directory("manifest")?;
        let mut manifest = valid_manifest();
        materialize_artifacts(&directory, &mut manifest)?;
        write_manifest_create_new(&directory, &manifest)?;
        let decoded: SettingsAuditManifest = serde_json::from_slice(&std::fs::read(
            directory.join(SETTINGS_AUDIT_MANIFEST_FILE),
        )?)?;
        assert_eq!(decoded, manifest);
        assert!(write_manifest_create_new(&directory, &manifest).is_err());
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn artifact_validator_recomputes_hash_and_decodes_exact_dimensions()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let directory = temporary_directory("artifacts")?;
        let mut manifest = valid_manifest();
        materialize_artifacts(&directory, &mut manifest)?;
        manifest.validate_artifacts(&directory)?;

        manifest.pages[0].captures[0].sha256 = "b".repeat(64);
        assert!(manifest.validate_artifacts(&directory).is_err());

        let capture_file = manifest.pages[0].captures[0].file.clone();
        let invalid = b"not a WebP image";
        std::fs::write(directory.join(&capture_file), invalid)?;
        manifest.pages[0].captures[0].sha256 = test_hash(invalid);
        assert!(manifest.validate_artifacts(&directory).is_err());

        let wrong_size = encode_test_webp(AuditDimensions {
            width: 1,
            height: 1,
        })?;
        std::fs::write(directory.join(&capture_file), &wrong_size)?;
        manifest.pages[0].captures[0].sha256 = test_hash(&wrong_size);
        assert!(manifest.validate_artifacts(&directory).is_err());
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn artifact_validator_rejects_non_regular_capture()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let directory = temporary_directory("non-regular")?;
        let mut manifest = valid_manifest();
        materialize_artifacts(&directory, &mut manifest)?;
        let capture_path = directory.join(&manifest.pages[0].captures[0].file);
        std::fs::remove_file(&capture_path)?;
        std::fs::create_dir(&capture_path)?;
        assert!(manifest.validate_artifacts(&directory).is_err());
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn artifact_validator_rejects_symlink_capture()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let directory = temporary_directory("symlink")?;
        let mut manifest = valid_manifest();
        materialize_artifacts(&directory, &mut manifest)?;
        let capture_path = directory.join(&manifest.pages[0].captures[0].file);
        let target_path = directory.join("symlink-target.webp");
        std::fs::rename(&capture_path, &target_path)?;
        std::os::unix::fs::symlink(&target_path, &capture_path)?;
        assert!(manifest.validate_artifacts(&directory).is_err());
        std::fs::remove_dir_all(directory)?;
        Ok(())
    }

    #[test]
    fn validator_rejects_mutation_scroll_gaps_unsafe_names_and_wrong_dpi() {
        let mut manifest = valid_manifest();
        manifest.pages[7].draft_unchanged = false;
        assert!(manifest.validate().is_err());

        let mut manifest = valid_manifest();
        manifest.pages[3].content_height = 1_500.0;
        manifest.pages[3].captures[1].scroll_y = 1_000.0;
        assert!(manifest.validate().is_err());

        let mut manifest = valid_manifest();
        manifest.pages[4].captures[0].file = "nested/page.webp".into();
        assert!(manifest.validate().is_err());

        let mut manifest = valid_manifest();
        manifest.scale_factor = 1.25;
        assert!(manifest.validate().is_err());

        let mut manifest = valid_manifest();
        manifest.pages[12].content_width = manifest.pages[12].viewport_width + 2.0;
        assert!(manifest.validate().is_err());
    }
}
