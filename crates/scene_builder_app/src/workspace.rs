use scene_builder_core::project::package::Package;
use scene_builder_core::project::NanoID;

/// Document state: the pack, whether it differs from the last save/load, and the open scene.
pub struct Workspace {
    pub package: Package,
    pub dirty: bool,
    pub selected_scene: Option<NanoID>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            package: Package::new(),
            dirty: false,
            selected_scene: None,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn reset(&mut self) {
        self.package = Package::new();
        self.dirty = false;
        self.selected_scene = None;
    }

    pub fn set_package(&mut self, package: Package, dirty: bool) {
        self.package = package;
        self.dirty = dirty;
        self.selected_scene = None;
    }

    pub fn has_save_path(&self) -> bool {
        !self.package.pack_path.as_os_str().is_empty()
    }

    /// Status-bar document label. Empty when the pack is in memory only (e.g. SLAL import).
    pub fn document_status(&self) -> &'static str {
        if self.dirty {
            "Modified"
        } else if self.has_save_path() {
            "Saved"
        } else {
            ""
        }
    }

    pub fn pack_display_name(&self) -> &str {
        if self.package.pack_name.is_empty() {
            "Untitled"
        } else {
            self.package.pack_name.as_str()
        }
    }
}
