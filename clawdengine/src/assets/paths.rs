use std::path::PathBuf;
use std::sync::OnceLock;

static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Initialize asset path resolution. Call once at startup before any asset loading.
/// Detects .app bundle (macOS) vs dev mode (cargo run).
pub fn init() {
    BASE_DIR.get_or_init(|| {
        // Detect .app bundle: exe lives in Contents/MacOS/
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe) = exe.canonicalize().ok() {
                if let Some(macos_dir) = exe.parent() {
                    if macos_dir.file_name().is_some_and(|n| n == "MacOS") {
                        if let Some(contents) = macos_dir.parent() {
                            let resources = contents.join("Resources");
                            if resources.is_dir() {
                                log::info!("Running from .app bundle: {}", resources.display());
                                return resources;
                            }
                        }
                    }
                }
            }
        }
        // Fallback: CWD (dev mode / cargo run)
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    });
}

/// Resolve a relative path against the base directory.
/// In .app bundle: relative to Contents/Resources/
/// In dev mode: relative to CWD at startup
pub fn resolve(relative: &str) -> PathBuf {
    BASE_DIR
        .get()
        .map(|base| base.join(relative))
        .unwrap_or_else(|| PathBuf::from(relative))
}

/// Get the base directory (Contents/Resources/ or CWD).
#[allow(dead_code)]
pub fn base_dir() -> PathBuf {
    BASE_DIR.get().cloned().unwrap_or_else(|| PathBuf::from("."))
}

/// Returns true if running inside a .app bundle.
#[allow(dead_code)]
pub fn is_bundled() -> bool {
    BASE_DIR
        .get()
        .and_then(|p| p.file_name())
        .is_some_and(|n| n == "Resources")
}
