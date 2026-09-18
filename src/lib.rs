use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io,
    path::{Component, Path, PathBuf},
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use thiserror::Error;
use unrar_rs::{ExtractOptions, RarArchive};

// UnRAR source code may be used in any software to handle RAR archives without
// limitations free of charge, but cannot be used to develop RAR (WinRAR)
// compatible archiver and to re-create RAR compression algorithm, which is
// proprietary. Distribution of modified UnRAR source code in separate form or
// as a part of other software is permitted, provided that full text of this
// paragraph, starting from “UnRAR source code” words, is included in license,
// or in documentation if license is not available, and in source code comments
// of resulting package.
use zip::ZipArchive;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

const TTS_FOLDERS: &[&str] = &[
    "Images", "Images Raw", "Models", "Models Raw", "Assetbundles", "Audio", "PDF", "Text", "Video", "Workshop",
];

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("无法读取 ZIP：{0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("文件操作失败：{0}")]
    Io(#[from] io::Error),
    #[error("压缩包内含不安全路径：{0}")]
    UnsafePath(String),
    #[error("未找到 TTS 的标准目录（例如 Images、Models 或 Workshop）")]
    NoTtsContent,
    #[error("压缩包内存在多个无法判断的 TTS 根目录")]
    AmbiguousRoot,
    #[error("无法撤销：没有最近一次导入记录")]
    NoUndo,
    #[error("导入记录损坏：{0}")]
    Manifest(#[from] serde_json::Error),
    #[error("压缩包解压失败：{0}")]
    Extract(String),
    #[error("暂不支持该压缩包格式：{0}")]
    UnsupportedFormat(String),
}

pub type Result<T> = std::result::Result<T, ToolError>;

/// A human-readable import stage.  The desktop UI uses these events to map
/// real work into a smooth, honest overall progress indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStage {
    InspectingArchive,
    ExtractingRar,
    PreparingRarFiles,
    ExtractingArchive,
    InstallingFiles,
    Finishing,
}

impl ImportStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::InspectingArchive => "正在检查压缩包",
            Self::ExtractingRar => "正在解压 RAR 图包",
            Self::PreparingRarFiles => "正在整理 RAR 文件",
            Self::ExtractingArchive => "正在解压图包",
            Self::InstallingFiles => "正在写入 TTS Mods",
            Self::Finishing => "正在保存备份",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportProgress {
    pub stage: ImportStage,
    pub completed_bytes: u64,
    pub total_bytes: u64,
}

impl ImportProgress {
    fn started(stage: ImportStage) -> Self { Self { stage, completed_bytes: 0, total_bytes: 1 } }
}

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    index: usize,
    relative: PathBuf,
    size: u64,
}

#[derive(Debug, Clone)]
pub struct ArchivePlan {
    pub entries: Vec<ArchiveEntry>,
    pub ignored_files: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    destination: PathBuf,
    created: Vec<PathBuf>,
    replaced: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ImportSummary {
    pub imported: usize,
    pub replaced: usize,
    pub ignored: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceFile {
    pub relative: PathBuf,
    pub references: Vec<String>,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceConflict {
    pub reference: String,
    pub candidates: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ResourceManifest {
    pub files: Vec<ResourceFile>,
    pub unresolved: Vec<String>,
    pub remote: Vec<String>,
    pub conflicts: Vec<ResourceConflict>,
}

#[derive(Debug, Clone, Default)]
pub struct DirectoryAssessment {
    pub recognized_folders: Vec<String>,
    pub workshop_files: usize,
    pub asset_files: usize,
}

impl DirectoryAssessment {
    pub fn is_usable(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("Mods"))
            .unwrap_or(false)
            || !self.recognized_folders.is_empty()
    }
}

pub fn assess_mods_directory(path: &Path) -> DirectoryAssessment {
    let mut assessment = DirectoryAssessment::default();
    for folder in TTS_FOLDERS {
        let candidate = path.join(folder);
        if !candidate.is_dir() {
            continue;
        }
        assessment.recognized_folders.push((*folder).to_string());
        let mut stack = vec![candidate];
        while let Some(current) = stack.pop() {
            let Ok(entries) = fs::read_dir(current) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(metadata) = fs::symlink_metadata(&path) else {
                    continue;
                };
                if metadata.is_dir() {
                    stack.push(path);
                } else if metadata.is_file() {
                    if folder.eq_ignore_ascii_case("Workshop")
                        && path.extension().is_some_and(|value| value.eq_ignore_ascii_case("json"))
                    {
                        assessment.workshop_files += 1;
                    } else {
                        assessment.asset_files += 1;
                    }
                }
            }
        }
    }
    assessment
}

impl ResourceManifest {
    pub fn add_file(&mut self, root: &Path, path: &Path, reference: &str) -> Result<()> {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| ToolError::UnsafePath(path.display().to_string()))?
            .to_path_buf();
        if let Some(file) = self.files.iter_mut().find(|file| file.relative == relative) {
            if !file.references.iter().any(|item| item == reference) {
                file.references.push(reference.to_string());
            }
            return Ok(());
        }
        let bytes = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        self.files.push(ResourceFile {
            relative,
            references: vec![reference.to_string()],
            size: bytes.len() as u64,
            sha256: format!("{:x}", hasher.finalize()),
        });
        Ok(())
    }

    pub fn sort(&mut self) {
        self.files.sort_by(|a, b| a.relative.cmp(&b.relative));
        self.unresolved.sort();
        self.remote.sort();
        self.conflicts.sort_by(|a, b| a.reference.cmp(&b.reference));
    }
}

const RESOURCE_EXTENSIONS: &[&str] = &[
    "assetbundle", "bundle", "bytes", "fbx", "glb", "gltf", "jpg", "jpeg", "json",
    "lua", "mp3", "mp4", "obj", "ogg", "pdf", "png", "shader", "txt", "unity3d",
    "wav", "webm", "xml",
];

fn looks_like_resource_reference(value: &str) -> bool {
    let value = value.split(['?', '#']).next().unwrap_or(value);
    let path = value.replace('\\', "/");
    let Some(extension) = Path::new(&path).extension().and_then(|item| item.to_str()) else {
        return false;
    };
    RESOURCE_EXTENSIONS.iter().any(|item| extension.eq_ignore_ascii_case(item))
}

fn collect_resource_references(value: &serde_json::Value, references: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::String(text)
            if looks_like_resource_reference(text) || text.contains("://") =>
        {
            references.insert(text.to_string());
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_resource_references(value, references);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values() {
                collect_resource_references(value, references);
            }
        }
        _ => {}
    }
}

fn scan_resource_files(current: &Path, files: &mut BTreeMap<String, Vec<PathBuf>>) -> Result<()> {
    for entry in fs::read_dir(current)? {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            scan_resource_files(&path, files)?;
        } else if metadata.is_file() {
            if let Some(name) = path.file_name().and_then(|item| item.to_str()) {
                files.entry(name.to_ascii_lowercase()).or_default().push(path);
            }
        }
    }
    Ok(())
}

fn normalized_local_reference(reference: &str) -> Option<PathBuf> {
    let value = reference
        .split(['?', '#'])
        .next()
        .unwrap_or(reference)
        .replace('\\', "/");
    let path = Path::new(&value);
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => components.push(value.to_owned()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if components.is_empty() {
        None
    } else {
        Some(components.iter().collect())
    }
}

/// Builds an explainable resource closure for one TTS JSON file.
pub fn build_resource_manifest(pack_path: &Path, mods_root: &Path) -> Result<ResourceManifest> {
    let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(pack_path)?)?;
    let mut references = BTreeSet::new();
    collect_resource_references(&value, &mut references);
    let mut by_name = BTreeMap::new();
    scan_resource_files(mods_root, &mut by_name)?;
    let mut manifest = ResourceManifest::default();
    let pack_parent = pack_path.parent().unwrap_or(mods_root);

    for reference in references {
        if reference.contains("://") {
            manifest.remote.push(reference);
            continue;
        }
        let Some(normalized) = normalized_local_reference(&reference) else {
            manifest.unresolved.push(reference);
            continue;
        };
        let mut exact = Vec::new();
        for candidate in [mods_root.join(&normalized), pack_parent.join(&normalized)] {
            if candidate.is_file() && !exact.contains(&candidate) {
                exact.push(candidate);
            }
        }
        if exact.len() == 1 {
            manifest.add_file(mods_root, &exact[0], &reference)?;
            continue;
        }
        if exact.len() > 1 {
            manifest.conflicts.push(ResourceConflict {
                reference,
                candidates: exact
                    .iter()
                    .filter_map(|path| path.strip_prefix(mods_root).ok().map(PathBuf::from))
                    .collect(),
            });
            continue;
        }
        let name = normalized.file_name().and_then(|item| item.to_str()).unwrap_or_default();
        let candidates = by_name.get(&name.to_ascii_lowercase()).cloned().unwrap_or_default();
        match candidates.as_slice() {
            [candidate] => manifest.add_file(mods_root, candidate, &reference)?,
            [] => manifest.unresolved.push(reference),
            _ => manifest.conflicts.push(ResourceConflict {
                reference,
                candidates: candidates
                    .iter()
                    .filter_map(|path| path.strip_prefix(mods_root).ok().map(PathBuf::from))
                    .collect(),
            }),
        }
    }
    manifest.sort();
    Ok(manifest)
}

fn is_tts_folder(component: &str) -> bool {
    TTS_FOLDERS.iter().any(|folder| folder.eq_ignore_ascii_case(component))
}

fn safe_components(name: &str) -> Result<Vec<String>> {
    let normalized = name.replace('\\', "/");
    let path = Path::new(&normalized);
    let mut parts = Vec::new();
    for part in path.components() {
        match part {
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
            Component::CurDir => {},
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return Err(ToolError::UnsafePath(name.into())),
        }
    }
    Ok(parts)
}

/// Determines which directory layer in a downloaded ZIP corresponds to Mods.
pub fn inspect_archive(path: &Path) -> Result<ArchivePlan> {
    let file = fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut candidates: Vec<(usize, Vec<String>, usize, u64)> = Vec::new();
    let mut ignored = 0;

    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.is_dir() { continue; }
        let parts = safe_components(entry.name())?;
        if let Some(anchor) = parts.iter().position(|part| is_tts_folder(part)) {
            candidates.push((index, parts, anchor, entry.size()));
        } else {
            ignored += 1;
        }
    }
    if candidates.is_empty() { return Err(ToolError::NoTtsContent); }

    let roots: BTreeSet<Vec<String>> = candidates.iter()
        .map(|(_, parts, anchor, _)| parts[..*anchor].to_vec())
        .collect();
    if roots.len() != 1 { return Err(ToolError::AmbiguousRoot); }
    let _root = roots.into_iter().next().unwrap();
    let entries = candidates.into_iter().map(|(index, parts, anchor, size)| ArchiveEntry {
        index,
        relative: parts[anchor..].iter().collect(),
        size,
    }).collect();
    Ok(ArchivePlan { entries, ignored_files: ignored })
}

fn app_root_path() -> PathBuf {
    dirs_next::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("TTS小工具")
}

pub fn app_root() -> Result<PathBuf> {
    let root = app_root_path();
    fs::create_dir_all(&root)?;
    Ok(root)
}

fn clear_app_root(root: &Path) -> Result<()> {
    if root.exists() {
        fs::remove_dir_all(root)?;
    }
    fs::create_dir_all(root)?;
    Ok(())
}

/// Starts every app session without saved directories, undo history, or old work files.
pub fn clear_startup_state() -> Result<()> {
    clear_app_root(&app_root_path())
}

fn backup_root() -> Result<PathBuf> {
    let path = app_root()?.join("backup");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn latest_backup() -> Result<PathBuf> { Ok(backup_root()?.join("latest")) }

fn valid_undo_record(latest: &Path) -> bool {
    let manifest_path = latest.join("manifest.json");
    let Ok(bytes) = fs::read(manifest_path) else { return false; };
    let Ok(manifest) = serde_json::from_slice::<Manifest>(&bytes) else { return false; };
    manifest.replaced.iter().all(|relative| latest.join("files").join(relative).is_file())
}

/// Returns whether the most recent successful import can still be undone.
pub fn has_undo_record() -> bool {
    latest_backup().map(|latest| valid_undo_record(&latest)).unwrap_or(false)
}

fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    if let Some(parent) = destination.parent() { fs::create_dir_all(parent)?; }
    fs::copy(source, destination)?;
    Ok(())
}

fn remove_empty_parents(mut path: PathBuf, stop: &Path) {
    while path.starts_with(stop) && path != stop {
        if fs::remove_dir(&path).is_err() { break; }
        if !path.pop() { break; }
    }
}

fn restore(manifest: &Manifest, backup: &Path) -> Result<()> {
    for relative in &manifest.created {
        let destination = manifest.destination.join(relative);
        if destination.exists() { fs::remove_file(&destination)?; }
        if let Some(parent) = destination.parent() { remove_empty_parents(parent.to_path_buf(), &manifest.destination); }
    }
    for relative in &manifest.replaced {
        copy_file(&backup.join("files").join(relative), &manifest.destination.join(relative))?;
    }
    Ok(())
}

fn copy_with_progress<R: io::Read, W: io::Write>(source: &mut R, output: &mut W, mut copied: impl FnMut(u64)) -> Result<u64> {
    let mut buffer = [0_u8; 128 * 1024];
    let mut total = 0_u64;
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 { break; }
        output.write_all(&buffer[..read])?;
        total += read as u64;
        copied(total);
    }
    Ok(total)
}

fn extract_to_staging(zip_path: &Path, plan: &ArchivePlan, staging: &Path, progress: &mut impl FnMut(u64, u64)) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    let total = plan.entries.iter().map(|item| item.size).sum::<u64>().max(1);
    let mut completed = 0_u64;
    for item in &plan.entries {
        let mut source = archive.by_index(item.index)?;
        let target = staging.join(&item.relative);
        if !target.starts_with(staging) { return Err(ToolError::UnsafePath(item.relative.display().to_string())); }
        if let Some(parent) = target.parent() { fs::create_dir_all(parent)?; }
        let mut output = fs::File::create(target)?;
        let item_start = completed;
        let copied = copy_with_progress(&mut source, &mut output, |current| progress(item_start + current, total))?;
        completed += copied;
        progress(completed, total);
    }
    Ok(())
}

/// Imports all recognized TTS assets in one rollback-capable transaction.
pub fn import_zip(zip_path: &Path, destination: &Path) -> Result<ImportSummary> {
    import_zip_with_progress(zip_path, destination, |_| {})
}

fn rar_file_bytes(current: &Path) -> Result<u64> {
    let mut total = 0_u64;
    for item in fs::read_dir(current)? {
        let item = item?;
        let metadata = fs::symlink_metadata(item.path())?;
        if metadata.file_type().is_symlink() { return Err(ToolError::UnsafePath(item.path().display().to_string())); }
        if metadata.is_dir() { total += rar_file_bytes(&item.path())?; }
        if metadata.is_file() { total += metadata.len(); }
    }
    Ok(total)
}

fn collect_rar_files(root: &Path, current: &Path, writer: &mut zip::ZipWriter<fs::File>, completed: &mut u64, total: u64, progress: &mut impl FnMut(u64, u64)) -> Result<()> {
    for item in fs::read_dir(current)? {
        let item = item?;
        let path = item.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() { return Err(ToolError::UnsafePath(path.display().to_string())); }
        if metadata.is_dir() {
            collect_rar_files(root, &path, writer, completed, total, progress)?;
        } else if metadata.is_file() {
            let relative = path.strip_prefix(root).map_err(|_| ToolError::UnsafePath(path.display().to_string()))?;
            let name = relative.to_string_lossy().replace('\\', "/");
            safe_components(&name)?;
            writer.start_file(name, zip::write::SimpleFileOptions::default())?;
            let mut source = fs::File::open(path)?;
            let start = *completed;
            let copied = copy_with_progress(&mut source, writer, |current| progress(start + current, total))?;
            *completed += copied;
            progress(*completed, total);
        }
    }
    Ok(())
}

fn rar_as_safe_zip(rar_path: &Path, workspace: &TempDir, progress: &mut impl FnMut(ImportProgress)) -> Result<PathBuf> {
    let extracted = workspace.path().join("rar-extracted");
    fs::create_dir_all(&extracted)?;
    progress(ImportProgress::started(ImportStage::ExtractingRar));
    let mut archive = RarArchive::open(fs::File::open(rar_path)?)
        .map_err(|error| ToolError::Extract(format!("无法读取 RAR：{error}")))?;
    if archive.metadata().is_encrypted {
        return Err(ToolError::Extract("该 RAR 的文件列表已加密；请先解压或提供未加密的图包。".into()));
    }
    let members = archive.metadata().members.clone();
    let total = members.iter().filter(|member| !member.is_directory).map(|member| member.unpacked_size.unwrap_or(member.compressed_size)).sum::<u64>().max(1);
    let options = ExtractOptions::default();
    let mut completed = 0_u64;
    for (index, member) in members.iter().enumerate() {
        if member.is_directory { continue; }
        if member.is_symlink || member.is_hardlink || member.is_file_copy {
            return Err(ToolError::UnsafePath(member.raw_name.clone()));
        }
        let parts = safe_components(&member.raw_name)?;
        if parts.is_empty() { continue; }
        let target = parts.iter().collect::<PathBuf>();
        let output = extracted.join(target);
        if !output.starts_with(&extracted) { return Err(ToolError::UnsafePath(member.raw_name.clone())); }
        if let Some(parent) = output.parent() { fs::create_dir_all(parent)?; }
        archive.extract_member_to_file(index, &options, None, &output)
            .map_err(|error| ToolError::Extract(format!("无法解压 RAR：{error}")))?;
        completed += member.unpacked_size.unwrap_or(member.compressed_size);
        progress(ImportProgress { stage: ImportStage::ExtractingRar, completed_bytes: completed.min(total), total_bytes: total });
    }
    let zip_path = workspace.path().join("rar-converted.zip");
    let file = fs::File::create(&zip_path)?;
    let mut writer = zip::ZipWriter::new(file);
    let total = rar_file_bytes(&extracted)?.max(1);
    progress(ImportProgress { stage: ImportStage::PreparingRarFiles, completed_bytes: 0, total_bytes: total });
    let mut completed = 0_u64;
    collect_rar_files(&extracted, &extracted, &mut writer, &mut completed, total, &mut |current, total| {
        progress(ImportProgress { stage: ImportStage::PreparingRarFiles, completed_bytes: current, total_bytes: total });
    })?;
    writer.finish()?;
    Ok(zip_path)
}

/// Imports a ZIP directly, or a RAR through the bundled RAR decoder.
pub fn import_archive_with_progress<F>(archive_path: &Path, destination: &Path, progress: F) -> Result<ImportSummary>
where
    F: FnMut(ImportProgress),
{
    let mut progress = progress;
    let extension = archive_path.extension().and_then(|value| value.to_str()).unwrap_or_default();
    if extension.eq_ignore_ascii_case("zip") { return import_zip_with_progress(archive_path, destination, progress); }
    if extension.eq_ignore_ascii_case("rar") {
        let workspace = TempDir::new_in(app_root()?)?;
        progress(ImportProgress::started(ImportStage::InspectingArchive));
        let converted = rar_as_safe_zip(archive_path, &workspace, &mut progress)?;
        return import_zip_with_progress(&converted, destination, progress);
    }
    Err(ToolError::UnsupportedFormat(extension.into()))
}

/// Imports a ZIP and reports byte-level progress for each import stage.
pub fn import_zip_with_progress<F>(zip_path: &Path, destination: &Path, mut progress: F) -> Result<ImportSummary>
where
    F: FnMut(ImportProgress),
{
    progress(ImportProgress::started(ImportStage::InspectingArchive));
    let plan = inspect_archive(zip_path)?;
    fs::create_dir_all(destination)?;
    let scratch = TempDir::new_in(app_root()?)?;
    let staging = scratch.path().join("staging");
    progress(ImportProgress { stage: ImportStage::ExtractingArchive, completed_bytes: 0, total_bytes: plan.entries.iter().map(|item| item.size).sum::<u64>().max(1) });
    extract_to_staging(zip_path, &plan, &staging, &mut |current, total| {
        progress(ImportProgress { stage: ImportStage::ExtractingArchive, completed_bytes: current, total_bytes: total });
    })?;

    let transaction = scratch.path().join("transaction");
    fs::create_dir_all(transaction.join("files"))?;
    let mut manifest = Manifest { destination: destination.to_path_buf(), created: Vec::new(), replaced: Vec::new() };
    let mut installed = 0;
    let staged_bytes = plan.entries.iter().map(|item| item.size).sum::<u64>();
    let backup_bytes = plan.entries.iter().filter_map(|item| fs::metadata(destination.join(&item.relative)).ok().map(|meta| meta.len())).sum::<u64>();
    let install_total = (staged_bytes + backup_bytes).max(1);
    let mut install_completed = 0_u64;
    progress(ImportProgress { stage: ImportStage::InstallingFiles, completed_bytes: 0, total_bytes: install_total });

    let result: Result<()> = (|| {
        for item in &plan.entries {
            let target = destination.join(&item.relative);
            let staged = staging.join(&item.relative);
            if target.exists() {
                let backup_target = transaction.join("files").join(&item.relative);
                if let Some(parent) = backup_target.parent() { fs::create_dir_all(parent)?; }
                let mut source = fs::File::open(&target)?;
                let mut output = fs::File::create(&backup_target)?;
                let start = install_completed;
                let copied = copy_with_progress(&mut source, &mut output, |current| {
                    progress(ImportProgress { stage: ImportStage::InstallingFiles, completed_bytes: start + current, total_bytes: install_total });
                })?;
                install_completed += copied;
                manifest.replaced.push(item.relative.clone());
            } else {
                manifest.created.push(item.relative.clone());
            }
            if let Some(parent) = target.parent() { fs::create_dir_all(parent)?; }
            let mut source = fs::File::open(&staged)?;
            let mut output = fs::File::create(&target)?;
            let start = install_completed;
            let copied = copy_with_progress(&mut source, &mut output, |current| {
                progress(ImportProgress { stage: ImportStage::InstallingFiles, completed_bytes: start + current, total_bytes: install_total });
            })?;
            install_completed += copied;
            installed += 1;
            progress(ImportProgress { stage: ImportStage::InstallingFiles, completed_bytes: install_completed, total_bytes: install_total });
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = restore(&manifest, &transaction);
        return Err(error);
    }

    progress(ImportProgress::started(ImportStage::Finishing));
    let final_backup = latest_backup()?;
    if final_backup.exists() { fs::remove_dir_all(&final_backup)?; }
    fs::rename(&transaction, &final_backup)?;
    fs::write(final_backup.join("manifest.json"), serde_json::to_vec_pretty(&manifest)?)?;
    progress(ImportProgress { stage: ImportStage::Finishing, completed_bytes: 1, total_bytes: 1 });
    Ok(ImportSummary { imported: installed, replaced: manifest.replaced.len(), ignored: plan.ignored_files })
}

pub fn undo_last_import() -> Result<()> {
    let latest = latest_backup()?;
    undo_from(&latest)
}

fn undo_from(latest: &Path) -> Result<()> {
    let manifest_path = latest.join("manifest.json");
    if !valid_undo_record(&latest) { return Err(ToolError::NoUndo); }
    let manifest: Manifest = serde_json::from_slice(&fs::read(manifest_path)?)?;
    restore(&manifest, &latest)?;
    fs::remove_dir_all(latest)?;
    Ok(())
}

pub fn saved_destination() -> Result<Option<PathBuf>> {
    let setting = app_root()?.join("settings.json");
    if !setting.exists() { return Ok(None); }
    #[derive(Deserialize)] struct Settings { destination: PathBuf }
    Ok(Some(serde_json::from_slice::<Settings>(&fs::read(setting)?)?.destination))
}

pub fn save_destination(destination: &Path) -> Result<()> {
    #[derive(Serialize)] struct Settings<'a> { destination: &'a Path }
    fs::write(app_root()?.join("settings.json"), serde_json::to_vec_pretty(&Settings { destination })?)?;
    Ok(())
}

/// Finds the normal Documents location and installed Steam library locations.
pub fn destination_candidates() -> Vec<PathBuf> {
    let mut candidates = BTreeSet::new();
    if let Some(documents) = dirs_next::document_dir() {
        candidates.insert(documents.join("My Games").join("Tabletop Simulator").join("Mods"));
    }
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(steam) = hkcu.open_subkey("Software\\Valve\\Steam") {
        if let Ok(path) = steam.get_value::<String, _>("SteamPath") {
            add_steam_library_candidates(Path::new(&path), &mut candidates);
        }
    }
    candidates.into_iter().collect()
}

fn add_steam_library_candidates(steam: &Path, candidates: &mut BTreeSet<PathBuf>) {
    let mut libraries = vec![steam.to_path_buf()];
    let vdf = steam.join("steamapps").join("libraryfolders.vdf");
    if let Ok(text) = fs::read_to_string(vdf) {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.contains("\"path\"") {
                let quoted: Vec<_> = trimmed.split('"').collect();
                if quoted.len() >= 4 {
                    libraries.push(PathBuf::from(quoted[3].replace("\\\\", "\\")));
                }
            }
        }
    }
    for library in libraries {
        let manifest = library.join("steamapps").join("appmanifest_286160.acf");
        let game = library.join("steamapps").join("common").join("Tabletop Simulator").join("Tabletop Simulator_Data").join("Mods");
        if manifest.exists() || game.exists() { candidates.insert(game); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_with(files: &[&str]) -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut writer = zip::ZipWriter::new(file.reopen().unwrap());
        for name in files { writer.start_file(name, zip::write::SimpleFileOptions::default()).unwrap(); writer.write_all(b"x").unwrap(); }
        writer.finish().unwrap();
        file
    }

    fn zip_with_payload(name: &str, bytes: usize) -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut writer = zip::ZipWriter::new(file.reopen().unwrap());
        writer.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(&vec![7_u8; bytes]).unwrap();
        writer.finish().unwrap();
        file
    }

    #[test]
    fn accepts_wrapped_mods() {
        let zip = zip_with(&["Downloaded Mod/Mods/Images/a.png", "Downloaded Mod/Mods/Models/b.obj"]);
        let plan = inspect_archive(zip.path()).unwrap();
        assert_eq!(plan.entries.len(), 2);
        assert_eq!(plan.entries[0].relative, PathBuf::from("Images/a.png"));
    }

    #[test]
    fn rejects_path_traversal() {
        let zip = zip_with(&["Images/../../bad.txt"]);
        assert!(matches!(inspect_archive(zip.path()), Err(ToolError::UnsafePath(_))));
    }

    #[test]
    fn extraction_reports_multiple_byte_updates_for_one_large_file() {
        let zip = zip_with_payload("Images/large.png", 512 * 1024);
        let plan = inspect_archive(zip.path()).unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let mut updates = Vec::new();
        extract_to_staging(zip.path(), &plan, scratch.path(), &mut |current, total| updates.push((current, total))).unwrap();
        assert!(updates.len() > 2, "large files should update before their extraction finishes");
        assert_eq!(updates.last(), Some(&(512 * 1024, 512 * 1024)));
    }

    #[test]
    fn valid_undo_record_requires_a_readable_manifest_and_backup_files() {
        let root = tempfile::tempdir().unwrap();
        let latest = root.path().join("latest");
        fs::create_dir_all(latest.join("files/Images")).unwrap();
        fs::write(latest.join("files/Images/old.png"), b"old").unwrap();
        let manifest = Manifest { destination: root.path().join("mods"), created: vec![], replaced: vec![PathBuf::from("Images/old.png")] };
        fs::write(latest.join("manifest.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(valid_undo_record(&latest));
        fs::remove_file(latest.join("files/Images/old.png")).unwrap();
        assert!(!valid_undo_record(&latest));
    }

    #[test]
    fn undo_consumes_its_backup_record() {
        let root = tempfile::tempdir().unwrap();
        let latest = root.path().join("latest");
        let mods = root.path().join("mods");
        fs::create_dir_all(latest.join("files/Images")).unwrap();
        fs::create_dir_all(mods.join("Images")).unwrap();
        fs::write(latest.join("files/Images/old.png"), b"before").unwrap();
        fs::write(mods.join("Images/old.png"), b"after").unwrap();
        let manifest = Manifest { destination: mods.clone(), created: vec![], replaced: vec![PathBuf::from("Images/old.png")] };
        fs::write(latest.join("manifest.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
        undo_from(&latest).unwrap();
        assert_eq!(fs::read(mods.join("Images/old.png")).unwrap(), b"before");
        assert!(!latest.exists());
    }

    #[test]
    fn clearing_app_state_removes_settings_backups_and_old_work() {
        let root = tempfile::tempdir().unwrap();
        let app = root.path().join("TTS小工具");
        fs::create_dir_all(app.join("backup/latest/files")).unwrap();
        fs::create_dir_all(app.join("stale-work")).unwrap();
        fs::write(app.join("settings.json"), b"old setting").unwrap();
        fs::write(app.join("backup/latest/files/old.png"), b"old backup").unwrap();

        clear_app_root(&app).unwrap();

        assert!(app.is_dir());
        assert!(!app.join("settings.json").exists());
        assert!(!app.join("backup").exists());
        assert!(!app.join("stale-work").exists());
    }

    #[test]
    fn resource_manifest_resolves_exact_paths_and_records_remote_urls() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("Workshop")).unwrap();
        fs::create_dir_all(root.path().join("Images")).unwrap();
        let pack = root.path().join("Workshop/pack.json");
        fs::write(
            &pack,
            br#"{"CustomAssetbundle":"Assetbundles/table.unity3d","URL":"https://example.com/table.png"}"#,
        )
        .unwrap();
        fs::create_dir_all(root.path().join("Assetbundles")).unwrap();
        fs::write(root.path().join("Assetbundles/table.unity3d"), b"bundle").unwrap();

        let manifest = build_resource_manifest(&pack, root.path()).unwrap();

        assert_eq!(manifest.files.len(), 1);
        assert_eq!(manifest.files[0].relative, PathBuf::from("Assetbundles/table.unity3d"));
        assert_eq!(manifest.remote, vec!["https://example.com/table.png"]);
        assert!(manifest.unresolved.is_empty());
    }

    #[test]
    fn resource_manifest_reports_ambiguous_basename_instead_of_guessing() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("Workshop")).unwrap();
        fs::create_dir_all(root.path().join("Images/A")).unwrap();
        fs::create_dir_all(root.path().join("Images/B")).unwrap();
        let pack = root.path().join("Workshop/pack.json");
        fs::write(&pack, br#"{"Image":"shared.png"}"#).unwrap();
        fs::write(root.path().join("Images/A/shared.png"), b"a").unwrap();
        fs::write(root.path().join("Images/B/shared.png"), b"b").unwrap();

        let manifest = build_resource_manifest(&pack, root.path()).unwrap();

        assert!(manifest.files.is_empty());
        assert_eq!(manifest.conflicts.len(), 1);
        assert_eq!(manifest.conflicts[0].reference, "shared.png");
    }
}
