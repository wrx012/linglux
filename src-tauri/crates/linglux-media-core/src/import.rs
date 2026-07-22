use crate::{sanitize_path_component, ProjectStore, TaskHandle};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, Metadata};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

const COPY_BUFFER_SIZE: usize = 1024 * 1024;
const COPY_PROGRESS_INTERVAL_BYTES: u64 = 16 * 1024 * 1024;
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedMediaFile {
    pub source_file_name: String,
    pub managed_path: String,
    pub url: String,
    pub byte_size: u64,
    pub fingerprint: String,
}

pub fn import_media_paths(
    store: &ProjectStore,
    project_id: &str,
    source_paths: &[PathBuf],
    task: &TaskHandle,
) -> Result<Vec<ImportedMediaFile>, String> {
    let sources = source_paths
        .iter()
        .filter(|path| path.is_file() && is_supported_media_path(path))
        .cloned()
        .collect::<Vec<_>>();

    if sources.is_empty() {
        return Err("没有可导入的本地媒体文件。".to_string());
    }

    let total_bytes = sources
        .iter()
        .filter_map(|path| fs::metadata(path).ok())
        .map(|metadata| metadata.len())
        .sum::<u64>()
        .max(1);
    let media_dir = store.media_dir(project_id)?;
    let mut completed_bytes = 0_u64;
    let mut imported = Vec::with_capacity(sources.len());

    for (index, source) in sources.iter().enumerate() {
        if task.is_cancel_requested() {
            task.mark_cancelled();
            return Err("任务已取消。".to_string());
        }

        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("linglux-media.bin");
        task.set_progress(
            completed_bytes as f64 / total_bytes as f64 * 100.0,
            format!("正在导入 {} ({}/{})", file_name, index + 1, sources.len()),
        );

        let result = copy_managed_file(source, &media_dir, task, completed_bytes, total_bytes)?;
        completed_bytes = completed_bytes.saturating_add(result.byte_size);
        imported.push(result);
    }

    task.set_progress(100.0, "素材导入完成");
    Ok(imported)
}

fn is_supported_media_path(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    matches!(
        extension.as_str(),
        "mp4"
            | "mov"
            | "mkv"
            | "webm"
            | "avi"
            | "m4v"
            | "jpg"
            | "jpeg"
            | "png"
            | "webp"
            | "gif"
            | "wav"
            | "mp3"
            | "m4a"
            | "aac"
            | "flac"
            | "ogg"
            | "srt"
            | "vtt"
            | "txt"
    )
}

fn copy_managed_file(
    source: &Path,
    media_dir: &Path,
    task: &TaskHandle,
    completed_before: u64,
    total_bytes: u64,
) -> Result<ImportedMediaFile, String> {
    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("linglux-media.bin");
    let safe_name = sanitize_path_component(source_name, "linglux-media.bin");
    let source_metadata = fs::metadata(source).map_err(|error| error.to_string())?;
    let source_size = source_metadata.len();
    let fingerprint = source_fingerprint(source, &source_metadata);
    let final_path = unique_managed_path(media_dir, &fingerprint, &safe_name);

    if final_path.is_file() {
        task.set_progress(
            completed_before.saturating_add(source_size) as f64 / total_bytes as f64 * 100.0,
            format!("复用已导入素材 {source_name}"),
        );
        return Ok(imported_media_file(
            source_name,
            final_path,
            source_size,
            fingerprint,
        ));
    }

    let temporary_path = unique_temporary_path(media_dir, &safe_name);

    // Same-volume desktop imports should be metadata-only. APFS clone-on-write
    // keeps an independent project copy; other filesystems use a hard link when
    // possible. Both avoid copying multi-gigabyte media before it can appear.
    if try_fast_managed_file(source, &temporary_path) {
        publish_managed_file(&temporary_path, &final_path)?;
        task.set_progress(
            completed_before.saturating_add(source_size) as f64 / total_bytes as f64 * 100.0,
            format!("已快速导入 {source_name}"),
        );
        return Ok(imported_media_file(
            source_name,
            final_path,
            source_size,
            fingerprint,
        ));
    }

    // Removable drives and filesystems that do not support hard links fall back
    // to a durable streaming copy. Progress updates are throttled so a large
    // import does not write thousands of task journal entries per file.
    let source_file = File::open(source).map_err(|error| error.to_string())?;
    let target_file = File::create(&temporary_path).map_err(|error| error.to_string())?;
    let mut reader = BufReader::with_capacity(COPY_BUFFER_SIZE, source_file);
    let mut writer = BufWriter::with_capacity(COPY_BUFFER_SIZE, target_file);
    let mut buffer = vec![0_u8; COPY_BUFFER_SIZE];
    let mut copied = 0_u64;
    let mut next_progress_update = COPY_PROGRESS_INTERVAL_BYTES;

    let copy_result = (|| -> Result<(), String> {
        loop {
            if task.is_cancel_requested() {
                return Err("任务已取消。".to_string());
            }

            let read = reader
                .read(&mut buffer)
                .map_err(|error| error.to_string())?;

            if read == 0 {
                break;
            }

            writer
                .write_all(&buffer[..read])
                .map_err(|error| error.to_string())?;
            copied = copied.saturating_add(read as u64);

            if copied >= next_progress_update || copied >= source_size {
                let overall = completed_before.saturating_add(copied) as f64 / total_bytes as f64;
                task.set_progress(overall * 100.0, format!("正在复制 {source_name}"));
                next_progress_update = copied.saturating_add(COPY_PROGRESS_INTERVAL_BYTES);
            }
        }

        writer.flush().map_err(|error| error.to_string())?;
        writer
            .get_ref()
            .sync_all()
            .map_err(|error| error.to_string())
    })();

    if let Err(error) = copy_result {
        let _ = fs::remove_file(&temporary_path);

        if task.is_cancel_requested() {
            task.mark_cancelled();
        }

        return Err(error);
    }

    publish_managed_file(&temporary_path, &final_path)?;
    Ok(imported_media_file(
        source_name,
        final_path,
        source_size,
        fingerprint,
    ))
}

fn source_fingerprint(source: &Path, metadata: &Metadata) -> String {
    let canonical_source = fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
    let modified_nanos = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let mut fingerprint = FNV_OFFSET_BASIS;

    update_fingerprint(
        &mut fingerprint,
        canonical_source.to_string_lossy().as_bytes(),
    );
    update_fingerprint(&mut fingerprint, &metadata.len().to_le_bytes());
    update_fingerprint(&mut fingerprint, &modified_nanos.to_le_bytes());
    format!("{fingerprint:016x}")
}

fn try_fast_managed_file(source: &Path, target: &Path) -> bool {
    if try_clone_on_write(source, target) {
        return true;
    }

    let _ = fs::remove_file(target);
    fs::hard_link(source, target).is_ok()
}

#[cfg(target_os = "macos")]
fn try_clone_on_write(source: &Path, target: &Path) -> bool {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_int};
    use std::os::unix::ffi::OsStrExt;

    extern "C" {
        fn clonefile(source: *const c_char, target: *const c_char, flags: u32) -> c_int;
    }

    let Ok(source) = CString::new(source.as_os_str().as_bytes()) else {
        return false;
    };
    let Ok(target) = CString::new(target.as_os_str().as_bytes()) else {
        return false;
    };

    // SAFETY: both pointers come from live CString values and remain valid for
    // the duration of the synchronous clonefile call.
    unsafe { clonefile(source.as_ptr(), target.as_ptr(), 0) == 0 }
}

#[cfg(not(target_os = "macos"))]
fn try_clone_on_write(_source: &Path, _target: &Path) -> bool {
    false
}

fn update_fingerprint(fingerprint: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *fingerprint ^= u64::from(*byte);
        *fingerprint = fingerprint.wrapping_mul(FNV_PRIME);
    }
}

fn publish_managed_file(temporary_path: &Path, final_path: &Path) -> Result<(), String> {
    if final_path.exists() {
        let _ = fs::remove_file(temporary_path);
        return Ok(());
    }

    fs::rename(temporary_path, final_path).map_err(|error| {
        let _ = fs::remove_file(temporary_path);
        error.to_string()
    })
}

fn imported_media_file(
    source_name: &str,
    final_path: PathBuf,
    source_size: u64,
    fingerprint: String,
) -> ImportedMediaFile {
    ImportedMediaFile {
        source_file_name: source_name.to_string(),
        managed_path: final_path.to_string_lossy().to_string(),
        url: path_to_file_url(&final_path),
        byte_size: source_size,
        fingerprint,
    }
}

fn unique_temporary_path(media_dir: &Path, safe_name: &str) -> PathBuf {
    for index in 0..10_000_u32 {
        let candidate = media_dir.join(format!(".import-{index}-{safe_name}.part"));

        if !candidate.exists() {
            return candidate;
        }
    }

    media_dir.join(format!(".import-{safe_name}.part"))
}

fn unique_managed_path(media_dir: &Path, fingerprint: &str, safe_name: &str) -> PathBuf {
    let candidate = media_dir.join(format!("{fingerprint}-{safe_name}"));

    if !candidate.exists() {
        return candidate;
    }

    if fs::metadata(&candidate).is_ok() {
        return candidate;
    }

    media_dir.join(format!("{fingerprint}-copy-{safe_name}"))
}

fn path_to_file_url(path: &Path) -> String {
    let path_string = path.to_string_lossy().replace('\\', "/");
    let mut encoded = String::new();

    for byte in path_string.as_bytes() {
        let character = *byte as char;

        if character.is_ascii_alphanumeric()
            || matches!(character, '/' | ':' | '-' | '_' | '.' | '~')
        {
            encoded.push(character);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if encoded.starts_with("//") {
            return format!("file:{encoded}");
        }

        return format!("file:///{}", encoded.trim_start_matches('/'));
    }

    #[cfg(not(target_os = "windows"))]
    format!("file://{encoded}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{unix_millis, TaskKind, TaskManager};

    #[test]
    fn imports_media_into_the_managed_project_package() {
        let root = std::env::temp_dir().join(format!("linglux-import-test-{}", unix_millis()));
        let source_dir = root.join("source");
        fs::create_dir_all(&source_dir).expect("create source directory");
        let source_path = source_dir.join("large sample.mp4");
        let source_bytes = vec![0x5a; COPY_BUFFER_SIZE * 2 + 37];
        fs::write(&source_path, &source_bytes).expect("write source fixture");
        let store = ProjectStore::new(root.join("projects")).expect("create project store");
        let task =
            TaskManager::new().create(TaskKind::Import, Some("project-1".into()), "import", None);
        task.start("import");

        let imported = import_media_paths(&store, "project-1", &[source_path], &task)
            .expect("import succeeds");

        assert_eq!(imported.len(), 1);
        assert_eq!(imported[0].byte_size, source_bytes.len() as u64);
        assert_eq!(
            fs::read(&imported[0].managed_path).expect("read managed media"),
            source_bytes
        );
        assert_eq!(task.snapshot().progress, 100.0);
        let _ = fs::remove_dir_all(root);
    }
}
