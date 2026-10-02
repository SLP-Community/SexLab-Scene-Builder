use std::path::{Path, PathBuf};
use std::thread;

use serde_hkx_features::{convert, Format};

#[derive(Debug)]
pub enum HkxPackError {
    Failed(String),
}

impl std::fmt::Display for HkxPackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Failed(s) => write!(f, "HKX pack failed: {s}"),
        }
    }
}

/// Pack Havok XML → SSE (amd64) Behavior.hkx via serde-hkx.
///
/// Runs on a dedicated thread with a large stack because serde-hkx can overflow
/// the default stack in debug builds (known upstream issue).
pub fn xml_to_hkx(xml_path: &Path, hkx_path: &Path) -> Result<(), HkxPackError> {
    if let Some(parent) = hkx_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| HkxPackError::Failed(e.to_string()))?;
    }

    let xml = xml_path.to_path_buf();
    let hkx = hkx_path.to_path_buf();

    let handle = thread::Builder::new()
        .name("slsb-hkx-convert".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || pack_on_thread(xml, hkx))
        .map_err(|e| HkxPackError::Failed(format!("failed to spawn convert thread: {e}")))?;

    handle
        .join()
        .map_err(|_| HkxPackError::Failed("HKX convert thread panicked".into()))?
}

fn pack_on_thread(xml: PathBuf, hkx: PathBuf) -> Result<(), HkxPackError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| HkxPackError::Failed(format!("tokio runtime: {e}")))?;

    rt.block_on(async {
        convert(&xml, Some(hkx.clone()), Format::Amd64)
            .await
            .map_err(|e| HkxPackError::Failed(e.to_string()))
    })?;

    if !hkx.is_file() {
        return Err(HkxPackError::Failed(format!(
            "convert reported success but {} missing",
            hkx.display()
        )));
    }
    Ok(())
}
