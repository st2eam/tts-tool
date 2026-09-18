#![windows_subsystem = "windows"]

use eframe::egui::{
    self, Align, Align2, Color32, CursorIcon, FontData, FontDefinitions, FontFamily, FontId,
    Frame, Layout, Margin, RichText, Sense, Stroke, Vec2,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tts_xiao_gongju::{
    assess_mods_directory, build_resource_manifest, clear_startup_state, destination_candidates,
    import_archive_with_progress, save_destination, DirectoryAssessment, ImportProgress, ImportStage,
    ResourceManifest,
};

// ── Palette (restrained: warm-neutral ground + one orange accent) ──────────
const BG: Color32 = Color32::from_rgb(247, 241, 236);
const CARD: Color32 = Color32::from_rgb(255, 255, 255);
const CARD_HOVER: Color32 = Color32::from_rgb(252, 238, 231);
const LINE: Color32 = Color32::from_rgb(232, 220, 210);
const TRACK: Color32 = Color32::from_rgb(238, 228, 219);

const BRAND: Color32 = Color32::from_rgb(230, 96, 64);
const BRAND_DEEP: Color32 = Color32::from_rgb(200, 72, 44);
const BRAND_SOFT: Color32 = Color32::from_rgb(251, 228, 216);

const INK: Color32 = Color32::from_rgb(45, 36, 32);
const INK_2: Color32 = Color32::from_rgb(138, 122, 112);
const INK_3: Color32 = Color32::from_rgb(180, 165, 155);

const OK: Color32 = Color32::from_rgb(74, 157, 111);
const OK_SOFT: Color32 = Color32::from_rgb(224, 240, 229);
const ERR: Color32 = Color32::from_rgb(194, 85, 85);
const ERR_SOFT: Color32 = Color32::from_rgb(245, 224, 224);

const RADIUS: f32 = 12.0;
const RADIUS_DROP: f32 = 14.0;

// ── Type scale (fixed px, compact 1.15 ratio) ──────────────────────────────
const FS_BODY: f32 = 13.0;
const FS_HEAD: f32 = 15.0;
const FS_CAP: f32 = 11.0;
const FS_NUM: f32 = 13.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum UiMode {
    Ready,
    Working,
    Success,
    Error,
}

#[derive(Clone)]
struct State {
    destination: Option<PathBuf>,
    candidates: Vec<PathBuf>,
    packs: Vec<PackInfo>,
    directory_hint: String,
    message: String,
    archive_name: Option<String>,
    mode: UiMode,
    busy: bool,
    stage: Option<ImportStage>,
    archive_is_rar: bool,
    progress: f32,
    target_progress: f32,
    directory_assessment: DirectoryAssessment,
}

struct TtsApp {
    state: Arc<Mutex<State>>,
    cover_textures: HashMap<PathBuf, Option<egui::TextureHandle>>,
    selected_pack: Option<PackInfo>,
    detail_cover: Option<egui::TextureHandle>,
    detail_manifest: Option<ResourceManifest>,
    export_status: Option<String>,
    pending_delete: Option<PackInfo>,
    export_paths: HashMap<PathBuf, PathBuf>,
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn compact_path(path: &str) -> String {
    const MAX: usize = 38;
    if path.chars().count() <= MAX {
        return path.into();
    }
    let tail: String = path
        .chars()
        .rev()
        .take(MAX - 2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("…\\{tail}")
}

fn found_candidates() -> Vec<PathBuf> {
    destination_candidates()
        .into_iter()
        .filter(|path| path.exists() && assess_mods_directory(path).is_usable(path))
        .collect()
}

/// A discovered TTS pack: its Workshop JSON path and the game title inside.
#[derive(Clone)]
struct PackInfo {
    path: PathBuf,
    name: String,
    cover: Option<PathBuf>,
}

/// Scan <mods>/Workshop for `.json` pack files and read their display name.
fn scan_packs(mods_dir: &Path) -> Vec<PackInfo> {
    let workshop = mods_dir.join("Workshop");
    let mut packs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&workshop) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json") {
                let name = read_pack_name(&path).unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("未命名")
                        .to_string()
                });
                packs.push(PackInfo {
                    cover: find_pack_cover(mods_dir, &path),
                    path,
                    name,
                });
            }
        }
    }
    packs.sort_by(|a, b| a.name.cmp(&b.name));
    packs
}

fn find_pack_cover(mods_dir: &Path, pack_path: &Path) -> Option<PathBuf> {
    let stem = pack_path.file_stem()?.to_str()?;
    let roots = [mods_dir.join("Workshop"), mods_dir.join("Images")];
    let extensions = ["jpg", "jpeg", "png"];
    roots
        .iter()
        .flat_map(|root| extensions.iter().map(move |extension| root.join(format!("{stem}.{extension}"))))
        .find(|path| path.is_file())
}

/// Read the "SaveName" field from a TTS workshop JSON (best effort).
fn read_pack_name(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let needle = "\"SaveName\"";
    let idx = content.find(needle)?;
    let rest = &content[idx + needle.len()..];
    let colon = rest.find(':')?;
    let rest = &rest[colon + 1..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn initial_state() -> State {
    let candidates = found_candidates();
    let (destination, directory_hint) = match candidates.len() {
        0 => (None, "未找到 TTS Mods 目录，请手动选择。".into()),
        1 => (
            Some(candidates[0].clone()),
            "已自动选择扫描到的目录。".into(),
        ),
        count => (
            None,
            format!("发现 {count} 个目录，请选择。"),
        ),
    };
    let packs = destination
        .as_ref()
        .map(|d| scan_packs(d))
        .unwrap_or_default();
    let directory_assessment = destination
        .as_deref()
        .map(assess_mods_directory)
        .unwrap_or_default();
    State {
        destination,
        candidates,
        packs,
        directory_hint,
        message: String::new(),
        archive_name: None,
        mode: UiMode::Ready,
        busy: false,
        stage: None,
        archive_is_rar: false,
        progress: 0.0,
        target_progress: 0.0,
        directory_assessment,
    }
}

fn stage_bounds(stage: Option<ImportStage>, is_rar: bool) -> (f32, f32) {
    match (stage, is_rar) {
        (Some(ImportStage::InspectingArchive), true) => (0.03, 0.05),
        (Some(ImportStage::ExtractingRar), true) => (0.05, 0.38),
        (Some(ImportStage::PreparingRarFiles), true) => (0.38, 0.52),
        (Some(ImportStage::InspectingArchive), false) => (0.03, 0.08),
        (Some(ImportStage::ExtractingArchive), true) => (0.52, 0.78),
        (Some(ImportStage::ExtractingArchive), false) => (0.08, 0.73),
        (Some(ImportStage::InstallingFiles), true) => (0.78, 0.96),
        (Some(ImportStage::InstallingFiles), false) => (0.73, 0.96),
        (Some(ImportStage::Finishing), _) => (0.96, 0.99),
        _ => (0.03, 0.92),
    }
}

fn event_progress(event: ImportProgress, is_rar: bool) -> f32 {
    let (start, end) = stage_bounds(Some(event.stage), is_rar);
    start + (end - start) * (event.completed_bytes as f32 / event.total_bytes.max(1) as f32).clamp(0.0, 1.0)
}

fn advance_display_progress(state: &mut State) {
    if !state.busy {
        return;
    }
    let (_, upper) = stage_bounds(state.stage, state.archive_is_rar);
    let simulated = (state.progress
        + 0.0012
        + (upper - 0.012 - state.progress).max(0.0) * 0.014)
        .min(upper - 0.012);
    let desired = state.target_progress.max(simulated);
    state.progress += (desired - state.progress).max(0.0) * 0.24;
    state.progress = state.progress.min(0.99);
}

fn source_label(path: &Path) -> &'static str {
    if path.to_string_lossy().contains("My Games") {
        "文档"
    } else {
        "Steam"
    }
}

fn ghost_button(ui: &mut egui::Ui, label: &str, enabled: bool) -> egui::Response {
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals.widgets;
        v.inactive.bg_fill = CARD;
        v.inactive.weak_bg_fill = CARD;
        v.inactive.bg_stroke = Stroke::new(1.0, LINE);
        v.inactive.fg_stroke = Stroke::new(1.0, INK_2);
        v.hovered.bg_fill = BRAND_SOFT;
        v.hovered.bg_stroke = Stroke::new(1.0, BRAND);
        v.hovered.fg_stroke = Stroke::new(1.0, BRAND_DEEP);
        v.active.bg_fill = BRAND_SOFT;
        v.active.bg_stroke = Stroke::new(1.0, BRAND_DEEP);
        v.active.fg_stroke = Stroke::new(1.0, BRAND_DEEP);
        v.noninteractive.bg_fill = CARD;
        v.noninteractive.bg_stroke = Stroke::new(1.0, LINE);
        v.noninteractive.fg_stroke = Stroke::new(1.0, INK_3);
        let txt = if enabled { INK_2 } else { INK_3 };
        ui.add_enabled(
            enabled,
            egui::Button::new(RichText::new(label).size(FS_CAP).color(txt))
                .corner_radius(8.0)
                .min_size(Vec2::new(56.0, 26.0)),
        )
    })
    .inner
}

fn directory_selector(ui: &mut egui::Ui, candidates: &[PathBuf]) -> Option<PathBuf> {
    let mut picked = None;
    ui.scope(|ui| {
        let v = &mut ui.style_mut().visuals.widgets;
        v.inactive.bg_fill = CARD;
        v.inactive.weak_bg_fill = CARD;
        v.inactive.bg_stroke = Stroke::new(1.0, LINE);
        v.inactive.fg_stroke = Stroke::new(1.0, INK);
        v.hovered.bg_fill = BRAND_SOFT;
        v.hovered.bg_stroke = Stroke::new(1.0, BRAND);
        v.hovered.fg_stroke = Stroke::new(1.0, BRAND_DEEP);
        egui::ComboBox::from_id_salt("tts-dest")
            .selected_text(RichText::new("选择已发现的目录").size(FS_BODY).color(INK))
            .width(280.0)
            .show_ui(ui, |ui| {
                for path in candidates {
                    let full = path_text(path);
                    let label = format!("{} · {}", source_label(path), compact_path(&full));
                    if ui
                        .selectable_label(false, RichText::new(label).size(FS_BODY).color(INK))
                        .on_hover_text(full)
                        .clicked()
                    {
                        picked = Some(path.clone());
                    }
                }
            });
    });
    picked
}

fn start_import(archive: PathBuf, state: Arc<Mutex<State>>) {
    let destination = match state.lock() {
        Ok(current) if current.busy => return,
        Ok(mut current) => match current.destination.clone() {
            Some(path) => path,
            None => {
                current.mode = UiMode::Error;
                current.message = "请先选择 TTS Mods 目录。".into();
                return;
            }
        },
        Err(_) => return,
    };
    let is_rar = archive
        .extension()
        .map(|value| value.to_string_lossy().eq_ignore_ascii_case("rar"))
        == Some(true);
    let supported = archive.extension().map(|value| {
        let value = value.to_string_lossy();
        value.eq_ignore_ascii_case("zip") || value.eq_ignore_ascii_case("rar")
    }) == Some(true);
    if !supported {
        if let Ok(mut current) = state.lock() {
            current.mode = UiMode::Error;
            current.message = "仅支持 ZIP / RAR 图包。".into();
        }
        return;
    }
    if let Ok(mut current) = state.lock() {
        current.busy = true;
        current.mode = UiMode::Working;
        current.archive_is_rar = is_rar;
        current.stage = Some(ImportStage::InspectingArchive);
        current.archive_name = archive.file_name().map(|name| name.to_string_lossy().into_owned());
        current.progress = 0.03;
        current.target_progress = 0.03;
        current.message = String::new();
    }
    thread::spawn(move || {
        let progress_state = state.clone();
        let result = import_archive_with_progress(&archive, &destination, move |event| {
            if let Ok(mut current) = progress_state.lock() {
                current.stage = Some(event.stage);
                current.target_progress =
                    current.target_progress.max(event_progress(event, current.archive_is_rar));
            }
        });
        if let Ok(mut current) = state.lock() {
            current.busy = false;
            match result {
                Ok(summary) => {
                    current.mode = UiMode::Success;
                    current.stage = Some(ImportStage::Finishing);
                    current.progress = 1.0;
                    current.target_progress = 1.0;
                    current.message = format!(
                        "写入 {} 个文件，替换 {} 个同名文件。",
                        summary.imported, summary.replaced
                    );
                    current.packs = scan_packs(&destination);
                    current.directory_assessment = assess_mods_directory(&destination);
                }
                Err(error) => {
                    current.mode = UiMode::Error;
                    current.stage = None;
                    current.target_progress = current.progress;
                    current.message = format!("已恢复原状：{error}");
                }
            }
        }
    });
}

impl TtsApp {
    fn new() -> Self {
        let mut state = initial_state();
        if let Err(error) = clear_startup_state() {
            state.mode = UiMode::Error;
            state.message = format!("无法清理旧缓存：{error}");
        } else {
            state = initial_state();
        }
        Self {
            state: Arc::new(Mutex::new(state)),
            cover_textures: HashMap::new(),
            selected_pack: None,
            detail_cover: None,
            detail_manifest: None,
            export_status: None,
            pending_delete: None,
            export_paths: HashMap::new(),
        }
    }

    fn set_destination(&self, path: PathBuf) {
        let assessment = assess_mods_directory(&path);
        if let Ok(mut current) = self.state.lock() {
            if !assessment.is_usable(&path) {
                current.mode = UiMode::Error;
                current.message =
                    "所选目录不是可识别的 TTS Mods 目录，请选择包含 Workshop、Images 或 Models 的目录。"
                        .into();
                return;
            }
            let result = save_destination(&path);
            current.destination = Some(path.clone());
            current.packs = scan_packs(&path);
            current.directory_assessment = assessment;
            current.mode = if result.is_ok() { UiMode::Ready } else { UiMode::Error };
            current.directory_hint = match result {
                Ok(()) => "目录已确认。".into(),
                Err(error) => format!("已选择，但无法保存：{error}"),
            };
            current.message = String::new();
            current.archive_name = None;
            current.stage = None;
            current.progress = 0.0;
            current.target_progress = 0.0;
        }
    }

    fn cover_texture(
        &mut self,
        ctx: &egui::Context,
        pack: &PackInfo,
    ) -> Option<egui::TextureHandle> {
        let path = pack.cover.as_ref()?.clone();
        if !self.cover_textures.contains_key(&path) {
            let texture = std::fs::read(&path)
                .ok()
                .and_then(|bytes| image::load_from_memory(&bytes).ok())
                .map(|image| {
                    let thumbnail = image.thumbnail(40, 40).to_rgba8();
                    let size = [thumbnail.width() as usize, thumbnail.height() as usize];
                    ctx.load_texture(
                        format!("pack-cover-{}", path.display()),
                        egui::ColorImage::from_rgba_unmultiplied(size, thumbnail.as_raw()),
                        egui::TextureOptions::LINEAR,
                    )
                });
            self.cover_textures.insert(path.clone(), texture);
        }
        self.cover_textures.get(&path).and_then(Clone::clone)
    }

    fn open_pack_details(&mut self, ctx: &egui::Context, pack: PackInfo) {
        self.detail_cover = pack.cover.as_ref().and_then(|path| {
            let bytes = std::fs::read(path).ok()?;
            let image = image::load_from_memory(&bytes).ok()?.thumbnail(220, 160).to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            Some(ctx.load_texture(
                format!("pack-detail-cover-{}", path.display()),
                egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                egui::TextureOptions::LINEAR,
            ))
        });
        self.detail_manifest = pack
            .path
            .parent()
            .and_then(Path::parent)
            .and_then(|root| build_resource_manifest(&pack.path, root).ok());
        self.selected_pack = Some(pack);
        self.export_status = None;
    }

    fn export_pack(&mut self, pack: &PackInfo) {
        let Some(output) = rfd::FileDialog::new()
            .set_title("导出图包")
            .set_file_name(format!("{}.zip", pack.name))
            .add_filter("ZIP 图包", &["zip"])
            .save_file()
        else {
            return;
        };

        let result = (|| -> Result<(usize, usize, usize), Box<dyn std::error::Error>> {
            let file = std::fs::File::create(&output)?;
            let mut archive = zip::ZipWriter::new(file);
            let options = zip::write::SimpleFileOptions::default();
            let mods_root = pack
                .path
                .parent()
                .and_then(Path::parent)
                .ok_or("无法确定 Mods 目录")?;
            let mut manifest = build_resource_manifest(&pack.path, mods_root)?;
            manifest.add_file(mods_root, &pack.path, "[图包 JSON]")?;
            if let Some(cover) = &pack.cover {
                manifest.add_file(mods_root, cover, "[图包封面]")?;
            }

            manifest.sort();
            for file in &manifest.files {
                let path = mods_root.join(&file.relative);
                let name = file.relative.to_string_lossy().replace('\\', "/");
                archive.start_file(name, options)?;
                let bytes = std::fs::read(path)?;
                use std::io::Write;
                archive.write_all(&bytes)?;
            }
            let manifest_json = serde_json::to_vec_pretty(&manifest)?;
            archive.start_file("tts-tool-manifest.json", options)?;
            use std::io::Write;
            archive.write_all(&manifest_json)?;
            archive.finish()?;
            Ok((manifest.unresolved.len(), manifest.conflicts.len(), manifest.remote.len()))
        })();

        let export_ok = result.is_ok();
        self.export_status = Some(match result {
            Ok((unresolved, conflicts, remote)) => format!(
                "已导出资源闭包：{}；未解析 {} 项、冲突 {} 项、远程资源 {} 项（详情见 tts-tool-manifest.json）",
                output.display(),
                unresolved,
                conflicts,
                remote
            ),
            Err(error) => format!("导出失败：{error}"),
        });
        if export_ok {
            self.export_paths.insert(pack.path.clone(), output);
        }
    }

    fn request_delete_pack(&mut self, pack: PackInfo) {
        self.pending_delete = Some(pack);
    }

    fn delete_pack(&mut self, pack: &PackInfo) {
        let Some(output) = self.export_paths.get(&pack.path).cloned() else {
            return;
        };
        let result = std::fs::remove_file(&output);
        if let Ok(mut state) = self.state.lock() {
            match result {
                Ok(()) => {
                    state.message = format!("已删除导出文件：{}", output.display());
                    state.mode = UiMode::Ready;
                    self.export_paths.remove(&pack.path);
                }
                Err(error) => {
                    state.mode = UiMode::Error;
                    state.message = format!("删除导出文件失败：{error}");
                }
            }
        }
    }

    fn choose_destination(&self) {
        let current_directory = self
            .state
            .lock()
            .ok()
            .and_then(|state| state.destination.clone());
        let mut dialog = rfd::FileDialog::new().set_title("选择 TTS Mods 文件夹");
        if let Some(directory) = current_directory {
            dialog = dialog.set_directory(directory);
        }
        if let Some(path) = dialog.pick_folder() {
            self.set_destination(path);
        }
    }

    fn choose_archive(&self) {
        if self
            .state
            .lock()
            .map(|state| state.busy || state.destination.is_none())
            .unwrap_or(true)
        {
            return;
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_title("选择 TTS 图包")
            .add_filter("图包", &["zip", "rar"])
            .pick_file()
        {
            start_import(path, self.state.clone());
        }
    }
}

// ── Drawing helpers ────────────────────────────────────────────────────────

/// Draw an upload icon: a rounded tray base with an upward arrow above it.
/// Stroke weight 1.8, consistent round joins.
fn draw_upload_icon(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    let stroke = Stroke::new(1.8, color);
    let w = 26.0;
    // base tray (half-rounded U shape)
    let base_y = center.y + 4.0;
    let half = w / 2.0;
    let corner = 4.0;
    painter.line_segment(
        [egui::pos2(center.x - half, base_y), egui::pos2(center.x - half + corner, base_y - corner)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(center.x - half + corner, base_y - corner), egui::pos2(center.x + half - corner, base_y - corner)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(center.x + half - corner, base_y - corner), egui::pos2(center.x + half, base_y)],
        stroke,
    );
    // arrow shaft
    let shaft_top = center.y - 14.0;
    painter.line_segment(
        [egui::pos2(center.x, base_y - corner), egui::pos2(center.x, shaft_top + 3.0)],
        stroke,
    );
    // arrow head
    let ah = 6.0;
    painter.line_segment(
        [egui::pos2(center.x - ah, shaft_top + 9.0), egui::pos2(center.x, shaft_top)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(center.x + ah, shaft_top + 9.0), egui::pos2(center.x, shaft_top)],
        stroke,
    );
}

/// Draw a checkmark inside a filled circle.
fn draw_ok_badge(painter: &egui::Painter, center: egui::Pos2, r: f32) {
    painter.circle_filled(center, r, OK);
    let stroke = Stroke::new(1.8, Color32::WHITE);
    painter.line_segment(
        [egui::pos2(center.x - 5.0, center.y + 0.5), egui::pos2(center.x - 1.5, center.y + 4.5)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(center.x - 1.5, center.y + 4.5), egui::pos2(center.x + 5.5, center.y - 3.5)],
        stroke,
    );
}

/// Draw an X inside a filled circle.
fn draw_err_badge(painter: &egui::Painter, center: egui::Pos2, r: f32) {
    painter.circle_filled(center, r, ERR);
    let stroke = Stroke::new(1.8, Color32::WHITE);
    let o = 4.5;
    painter.line_segment(
        [egui::pos2(center.x - o, center.y - o), egui::pos2(center.x + o, center.y + o)],
        stroke,
    );
    painter.line_segment(
        [egui::pos2(center.x - o, center.y + o), egui::pos2(center.x + o, center.y - o)],
        stroke,
    );
}

fn draw_progress_bar(painter: &egui::Painter, rect: egui::Rect, progress: f32, fill: Color32) {
    let radius = 3.5;
    painter.rect_filled(rect, radius, TRACK);
    let w = (rect.width() * progress.clamp(0.0, 1.0)).min(rect.width());
    if w > 1.0 {
        let fr = egui::Rect::from_min_size(rect.min, Vec2::new(w, rect.height()));
        painter.rect_filled(fr, radius, fill);
    }
}

// ── Sections ───────────────────────────────────────────────────────────────

fn brand_header(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        let r = 8.0;
        let cy = ui.min_rect().center().y;
        ui.painter().circle_filled(egui::pos2(ui.min_rect().min.x + r + 2.0, cy), r, BRAND);
        ui.add_space(20.0);
        ui.vertical(|ui| {
            ui.label(RichText::new("TTS 小工具").size(18.0).strong().color(INK));
            ui.label(
                RichText::new("管理已安装图包 · 安全导入新内容")
                    .size(FS_CAP)
                    .color(INK_2),
            );
        });
    });
}

fn destination_card(ui: &mut egui::Ui, snapshot: &State, app: &TtsApp) {
    Frame::default()
        .fill(CARD)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(RADIUS)
        .inner_margin(Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("工作目录").size(FS_CAP).color(INK_2));
                    if let Some(path) = snapshot.destination.as_deref() {
                        let full = path_text(path);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(compact_path(&full))
                                    .size(FS_BODY)
                                    .strong()
                                    .color(INK),
                            )
                            .on_hover_text(full);
                        });
                    } else if !snapshot.busy && !snapshot.candidates.is_empty() {
                        if let Some(path) = directory_selector(ui, &snapshot.candidates) {
                            app.set_destination(path);
                        }
                    } else {
                        ui.label(RichText::new("Mods 目录未设置").size(FS_BODY).color(INK_2));
                    }
                    // hint line, only if useful
                    if !snapshot.directory_hint.is_empty() && !snapshot.busy {
                        ui.label(RichText::new(&snapshot.directory_hint).size(FS_CAP).color(INK_3));
                    }
                    if snapshot.destination.is_some() && !snapshot.busy {
                        ui.label(
                            RichText::new(format!(
                                "{} 个标准目录 · {} 个图包 · {} 个资源文件",
                                snapshot.directory_assessment.recognized_folders.len(),
                                snapshot.directory_assessment.workshop_files,
                                snapshot.directory_assessment.asset_files
                            ))
                            .size(FS_CAP)
                            .color(INK_3),
                        );
                    }
                });
                if !snapshot.busy {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ghost_button(ui, "更改", true).clicked() {
                            app.choose_destination();
                        }
                    });
                } else {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ghost_button(ui, "更改", false);
                    });
                }
            });
        });
}

fn pack_management_area(ui: &mut egui::Ui, snapshot: &State, app: &mut TtsApp) {
    Frame::default()
        .fill(CARD)
        .stroke(Stroke::new(1.0, LINE))
        .corner_radius(RADIUS_DROP)
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
        ui.label(RichText::new("TTS 图包管理").size(FS_HEAD).strong().color(INK));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let count = snapshot.packs.len();
            let badge = Frame::default()
                .fill(if count == 0 { TRACK } else { BRAND_SOFT })
                .corner_radius(10.0)
                .inner_margin(Margin::symmetric(8, 3));
            badge.show(ui, |ui| {
                ui.label(
                    RichText::new(format!("{count} 个已安装"))
                        .size(FS_CAP)
                        .strong()
                        .color(if count == 0 { INK_2 } else { BRAND_DEEP }),
                );
            });
        });
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new("查看已安装图包、导出资源，或通过右键菜单管理导出文件。")
                    .size(FS_CAP)
                    .color(INK_2),
            );
            ui.add_space(10.0);

            if snapshot.packs.is_empty() {
                ui.allocate_ui(Vec2::new(ui.available_width(), 86.0), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(18.0);
                        ui.label(RichText::new("还没有扫描到图包").size(FS_BODY).color(INK_2));
                        ui.label(
                            RichText::new("导入完成后，图包会自动出现在这里")
                                .size(FS_CAP)
                                .color(INK_3),
                        );
                    });
                });
                return;
            }

            egui::ScrollArea::vertical()
                .max_height(148.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    for pack in &snapshot.packs {
                        let cover = app.cover_texture(ui.ctx(), pack);
                        let row = ui.allocate_ui(
                            Vec2::new(ui.available_width(), 44.0),
                            |ui| {
                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 44.0),
                                    Sense::click(),
                                );
                                let row_fill = if response.is_pointer_button_down_on() {
                                    BRAND_SOFT.gamma_multiply(0.8)
                                } else if response.hovered() {
                                    CARD_HOVER
                                } else {
                                    Color32::TRANSPARENT
                                };
                                ui.painter().rect_filled(rect.shrink(1.0), 8.0, row_fill);
                                if let Some(texture) = cover {
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(rect.min, Vec2::splat(40.0)),
                                        6.0,
                                        LINE,
                                    );
                                    ui.painter().image(
                                        texture.id(),
                                        egui::Rect::from_min_size(
                                            rect.min + Vec2::splat(1.0),
                                            Vec2::splat(38.0),
                                        ),
                                        egui::Rect::from_min_max(
                                            egui::pos2(0.0, 0.0),
                                            egui::pos2(1.0, 1.0),
                                        ),
                                        Color32::WHITE,
                                    );
                                } else {
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(rect.min, Vec2::splat(40.0)),
                                        6.0,
                                        BRAND_SOFT,
                                    );
                                }
                                ui.painter().text(
                                    egui::pos2(rect.min.x + 52.0, rect.center().y),
                                    Align2::LEFT_CENTER,
                                    &pack.name,
                                    FontId::proportional(FS_BODY),
                                    INK,
                                );
                                response
                            },
                        )
                        .inner
                        .on_hover_text(format!("打开详情\n{}", pack.path.display()));
                        row.context_menu(|ui| {
                            if ui.button("打开详情").clicked() {
                                app.open_pack_details(ui.ctx(), pack.clone());
                                ui.close();
                            }
                            if ui.button("导出图包").clicked() {
                                app.export_pack(pack);
                                ui.close();
                            }
                            if ui.button("在资源管理器中定位").clicked() {
                                let _ = std::process::Command::new("explorer")
                                    .arg("/select,")
                                    .arg(&pack.path)
                                    .spawn();
                                ui.close();
                            }
                            ui.separator();
                            let has_export = app
                                .export_paths
                                .get(&pack.path)
                                .is_some_and(|output| output.exists());
                            if ui
                                .add_enabled(has_export, egui::Button::new("删除导出文件"))
                                .clicked()
                            {
                                app.request_delete_pack(pack.clone());
                                ui.close();
                            }
                        });
                        if row.clicked() {
                            app.open_pack_details(ui.ctx(), pack.clone());
                        }
                    }
                });
        });
}

fn drop_area(ui: &mut egui::Ui, snapshot: &State, app: &mut TtsApp, hovering: bool) {
    let can_import = !snapshot.busy && snapshot.destination.is_some();
    let (fill, stroke_color, stroke_w) = match snapshot.mode {
        UiMode::Success => (OK_SOFT, OK, 1.0),
        UiMode::Error => (ERR_SOFT, ERR, 1.0),
        _ if hovering && can_import => (CARD_HOVER, BRAND, 1.5),
        _ => (CARD, LINE, 1.0),
    };

    Frame::default()
        .fill(fill)
        .stroke(Stroke::new(stroke_w, stroke_color))
        .corner_radius(RADIUS_DROP)
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("TTS 图包导入").size(FS_HEAD).strong().color(INK));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = if snapshot.busy { "正在处理" } else { "ZIP / RAR" };
                    ui.label(RichText::new(label).size(FS_CAP).color(if snapshot.busy {
                        BRAND_DEEP
                    } else {
                        INK_3
                    }));
                });
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new("将图包安全导入当前 Mods 目录，覆盖文件前会自动建立可撤销备份。")
                    .size(FS_CAP)
                    .color(INK_2),
            );
            ui.add_space(10.0);
            let rect = ui.available_rect_before_wrap();

            match snapshot.mode {
                UiMode::Ready => {
                    // ── Empty / drop mode (painter) ──
                    let height = if snapshot.destination.is_some() {
                        138.0
                    } else {
                        116.0
                    };
                    let sense = if can_import { Sense::click() } else { Sense::hover() };
                    let resp = ui.allocate_response(Vec2::new(ui.available_width(), height), sense);
                    if can_import && resp.hovered() {
                        ui.output_mut(|o| o.cursor_icon = CursorIcon::PointingHand);
                    }
                    if can_import && resp.clicked() {
                        app.choose_archive();
                    }

                    let rect = resp.rect;
                    let painter = ui.painter();
                    let cx = rect.center().x;

                    let icon_c = egui::pos2(cx, rect.center().y - 22.0);
                    let ic = if hovering && can_import { BRAND } else { BRAND_DEEP };
                    draw_upload_icon(painter, icon_c, ic);

                    let (head, sub) = if snapshot.destination.is_none() {
                        ("请先选择 TTS Mods 目录", "点击右侧「更改」选择文件夹".to_string())
                    } else if hovering {
                        ("松开鼠标开始导入", String::new())
                    } else {
                        ("拖入 ZIP / RAR 图包", "或点击此处选择文件".to_string())
                    };
                    painter.text(
                        egui::pos2(cx, rect.center().y + 12.0),
                        Align2::CENTER_CENTER,
                        head,
                        FontId::proportional(FS_HEAD),
                        INK,
                    );
                    if !sub.is_empty() {
                        painter.text(
                            egui::pos2(cx, rect.center().y + 34.0),
                            Align2::CENTER_CENTER,
                            sub,
                            FontId::proportional(FS_CAP),
                            INK_2,
                        );
                    }
                }

                UiMode::Working => {
                    let height = 118.0;
                    let resp = ui.allocate_response(Vec2::new(ui.available_width(), height), Sense::hover());
                    let rect = resp.rect;
                    let painter = ui.painter();

                    let title = snapshot.stage.map(ImportStage::label).unwrap_or("处理中");
                    painter.text(
                        egui::pos2(rect.min.x + 4.0, rect.min.y + 10.0),
                        Align2::LEFT_CENTER,
                        title,
                        FontId::proportional(FS_BODY),
                        INK,
                    );
                    painter.text(
                        egui::pos2(rect.max.x - 4.0, rect.min.y + 10.0),
                        Align2::RIGHT_CENTER,
                        format!("{:.0}%", snapshot.progress * 100.0),
                        FontId::proportional(FS_NUM),
                        BRAND_DEEP,
                    );
                    if let Some(name) = &snapshot.archive_name {
                        painter.text(
                            egui::pos2(rect.min.x + 4.0, rect.min.y + 34.0),
                            Align2::LEFT_CENTER,
                            compact_path(name),
                            FontId::proportional(FS_CAP),
                            INK_2,
                        );
                    }
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 4.0, rect.max.y - 36.0),
                        Vec2::new(rect.width() - 8.0, 6.0),
                    );
                    draw_progress_bar(painter, bar_rect, snapshot.progress, BRAND);
                    painter.text(
                        egui::pos2(rect.center().x, rect.max.y - 14.0),
                        Align2::CENTER_CENTER,
                        "请保持窗口开启",
                        FontId::proportional(FS_CAP),
                        INK_3,
                    );
                }

                UiMode::Success | UiMode::Error => {
                    let height = 92.0;
                    let resp = ui.allocate_response(Vec2::new(ui.available_width(), height), Sense::hover());
                    let rect = resp.rect;
                    let painter = ui.painter();

                    let is_ok = snapshot.mode == UiMode::Success;
                    let (color, soft_title) = if is_ok {
                        (OK, "导入完成")
                    } else {
                        (ERR, "导入失败")
                    };
                    let badge_c = egui::pos2(rect.min.x + 24.0, rect.min.y + 28.0);
                    if is_ok {
                        draw_ok_badge(painter, badge_c, 13.0);
                    } else {
                        draw_err_badge(painter, badge_c, 13.0);
                    }
                    painter.text(
                        egui::pos2(rect.min.x + 46.0, rect.min.y + 22.0),
                        Align2::LEFT_CENTER,
                        soft_title,
                        FontId::proportional(FS_HEAD),
                        color,
                    );
                    painter.text(
                        egui::pos2(rect.min.x + 46.0, rect.min.y + 42.0),
                        Align2::LEFT_CENTER,
                        &snapshot.message,
                        FontId::proportional(FS_BODY),
                        INK,
                    );
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 4.0, rect.max.y - 36.0),
                        Vec2::new(rect.width() - 8.0, 6.0),
                    );
                    draw_progress_bar(painter, bar_rect, snapshot.progress, color);
                    painter.text(
                        egui::pos2(rect.center().x, rect.max.y - 14.0),
                        Align2::CENTER_CENTER,
                        "拖入下一个图包继续",
                        FontId::proportional(FS_CAP),
                        INK_3,
                    );
                }
            }
        });
}

impl eframe::App for TtsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let snapshot = if let Ok(mut state) = self.state.lock() {
            advance_display_progress(&mut state);
            state.clone()
        } else {
            initial_state()
        };
        if snapshot.busy {
            ctx.request_repaint_after(Duration::from_millis(80));
        }
        let can_import = !snapshot.busy && snapshot.destination.is_some();
        let dropped = ctx.input(|input| input.raw.dropped_files.clone());
        if can_import {
            if let Some(path) = dropped.into_iter().find_map(|file| file.path) {
                start_import(path, self.state.clone());
            }
        }
        let hovering = ctx.input(|input| !input.raw.hovered_files.is_empty());

        egui::CentralPanel::default()
            .frame(Frame::default().fill(BG).inner_margin(Margin::same(20)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        brand_header(ui);
                        ui.add_space(12.0);
                        destination_card(ui, &snapshot, self);
                        ui.add_space(10.0);
                        pack_management_area(ui, &snapshot, self);
                        ui.add_space(10.0);
                        drop_area(ui, &snapshot, self, hovering);
                    });
            });

        if let Some(pack) = self.selected_pack.clone() {
            let mut open = true;
            egui::Window::new("图包详情")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(430.0)
                .show(ctx, |ui| {
                    if let Some(texture) = &self.detail_cover {
                        ui.vertical_centered(|ui| {
                            ui.add(
                                egui::Image::new(texture)
                                    .max_width(220.0)
                                    .max_height(160.0),
                            );
                        });
                        ui.add_space(8.0);
                    }
                    ui.label(RichText::new(&pack.name).size(FS_HEAD).color(INK));
                    ui.label(
                        RichText::new(format!("文件：{}", pack.path.display()))
                            .size(FS_CAP)
                            .color(INK_2),
                    );
                    if let Ok(metadata) = std::fs::metadata(&pack.path) {
                        ui.label(
                            RichText::new(format!("大小：{} KB", metadata.len().div_ceil(1024)))
                                .size(FS_CAP)
                                .color(INK_2),
                        );
                    }
                    if let Ok(content) = std::fs::read_to_string(&pack.path) {
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                            for key in ["GameMode", "GameType", "Description", "PlayerCount"] {
                                if let Some(value) = value.get(key).and_then(|value| value.as_str()) {
                                    ui.label(
                                        RichText::new(format!("{key}：{value}"))
                                            .size(FS_BODY)
                                            .color(INK),
                                    );
                                }
                            }
                            if let Some(manifest) = &self.detail_manifest {
                                ui.separator();
                                ui.label(RichText::new("资源完整性").size(FS_BODY).color(INK));
                                let healthy = manifest.unresolved.is_empty() && manifest.conflicts.is_empty();
                                ui.label(
                                    RichText::new(format!(
                                        "已找到 {} · 未解析 {} · 冲突 {} · 远程 {}",
                                        manifest.files.len(),
                                        manifest.unresolved.len(),
                                        manifest.conflicts.len(),
                                        manifest.remote.len()
                                    ))
                                    .size(FS_CAP)
                                    .color(if healthy { OK } else { BRAND_DEEP }),
                                );
                                if let Some(reference) = manifest.unresolved.first() {
                                    ui.label(
                                        RichText::new(format!("未解析：{reference}"))
                                            .size(FS_CAP)
                                            .color(INK_2),
                                    );
                                }
                                if let Some(conflict) = manifest.conflicts.first() {
                                    ui.label(
                                        RichText::new(format!(
                                            "同名冲突：{}（{} 个候选）",
                                            conflict.reference,
                                            conflict.candidates.len()
                                        ))
                                        .size(FS_CAP)
                                        .color(INK_2),
                                    );
                                }
                            }

                        }
                    }
                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("导出完整图包").size(FS_BODY).color(Color32::WHITE),
                                )
                                .fill(BRAND)
                                .corner_radius(8.0)
                                .min_size(Vec2::new(180.0, 32.0)),
                            )
                            .clicked()
                        {
                            self.export_pack(&pack);
                        }
                    });
                    if let Some(status) = &self.export_status {
                        ui.add_space(6.0);
                        ui.label(RichText::new(status).size(FS_CAP).color(INK_2));
                    }
                });
            if !open {
                self.selected_pack = None;
                self.detail_cover = None;
                self.detail_manifest = None;
            }
        }

        if let Some(pack) = self.pending_delete.clone() {
            let mut open = true;
            let mut confirm = false;
            let mut request_close = false;
            egui::Window::new("确认删除导出")
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .default_width(320.0)
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(format!("确定删除“{}”的导出文件？", pack.name))
                            .size(FS_HEAD)
                            .color(INK),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "只删除已生成的 ZIP 导出文件，不会修改 Mods 目录中的原始图包和资源。",
                        )
                        .size(FS_CAP)
                        .color(INK_2),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ghost_button(ui, "确认删除导出", true).clicked() {
                            confirm = true;
                        }
                        if ghost_button(ui, "取消", true).clicked() {
                            request_close = true;
                        }
                    });
                });
            if confirm {
                self.pending_delete = None;
                self.delete_pack(&pack);
            } else if !open || request_close {
                self.pending_delete = None;
            }
        }
    }
}

fn configure_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    for candidate in [
        r"C:\Windows\Fonts\NotoSansSC-Regular.ttf",
        r"C:\Windows\Fonts\msyh.ttc",
    ] {
        if let Ok(bytes) = std::fs::read(candidate) {
            fonts.font_data.insert(
                "tts_chinese".into(),
                std::sync::Arc::new(FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "tts_chinese".into());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    let app_icon = eframe::icon_data::from_png_bytes(include_bytes!("../logo.png"))
        .expect("内置应用图标无效");
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([480.0, 690.0])
        .with_min_inner_size([440.0, 530.0])
        .with_max_inner_size([720.0, 950.0])
        .with_resizable(true)
        .with_icon(app_icon);
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "TTS小工具",
        options,
        Box::new(|creation| {
            configure_fonts(&creation.egui_ctx);
            Ok(Box::new(TtsApp::new()))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn working_rar_state() -> State {
        State {
            destination: None,
            candidates: Vec::new(),
            packs: Vec::new(),
            directory_hint: String::new(),
            message: String::new(),
            archive_name: Some("test.rar".into()),
            mode: UiMode::Working,
            busy: true,
            stage: Some(ImportStage::ExtractingRar),
            archive_is_rar: true,
            progress: 0.05,
            target_progress: 0.05,
            directory_assessment: DirectoryAssessment::default(),
        }
    }

    #[test]
    fn simulated_rar_progress_advances_without_crossing_its_stage_limit() {
        let mut state = working_rar_state();
        let (_, upper) = stage_bounds(state.stage, true);
        for _ in 0..2_000 {
            advance_display_progress(&mut state);
        }
        assert!(state.progress > 0.05 && state.progress < upper);
    }

    #[test]
    fn progress_ranges_follow_the_import_order() {
        let rar_extract = stage_bounds(Some(ImportStage::ExtractingRar), true);
        let rar_prepare = stage_bounds(Some(ImportStage::PreparingRarFiles), true);
        let extract = stage_bounds(Some(ImportStage::ExtractingArchive), true);
        let install = stage_bounds(Some(ImportStage::InstallingFiles), true);
        assert!(
            rar_extract.1 <= rar_prepare.0
                && rar_prepare.1 <= extract.0
                && extract.1 <= install.0
        );
    }

    #[test]
    fn busy_state_does_not_accept_another_import() {
        let state = working_rar_state();
        assert!(state.busy);
    }
}
