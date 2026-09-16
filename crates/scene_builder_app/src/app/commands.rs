use super::{ExportConfirm, SceneBuilderApp};
use crate::io::{self, DialogResult};
use crate::jobs::{ChannelProgress, JobEvent, JobUi};
use egui::Context;
use log::{error, info};
use scene_builder_core::project::package::{ExportKind, Package};
use scene_builder_core::Progress;
use std::path::PathBuf;
use std::thread;

impl SceneBuilderApp {
    pub(super) fn save_project(&mut self, save_as: bool) {
        if !save_as && self.ws.has_save_path() {
            let path = self.ws.package.pack_path.clone();
            match self.ws.package.write(path) {
                Ok(()) => {
                    self.ws.dirty = false;
                    self.status = "Saved".into();
                }
                Err(e) => {
                    self.status = format!("Save failed: {e}");
                    error!("{e}");
                }
            }
        } else {
            let suggested = if self.ws.package.pack_name.is_empty() {
                "project.slsb.json".into()
            } else {
                format!("{}.slsb.json", self.ws.package.pack_name)
            };
            io::spawn_save_as(self.dialog_tx.clone(), suggested);
        }
    }

    /// Show the Pandora clip tip before export unless the user dismissed it.
    pub(super) fn request_export(&mut self, kind: ExportKind) {
        if self.prefs.hide_export_clip_tip {
            io::spawn_export(self.dialog_tx.clone(), kind);
        } else {
            self.export_confirm = Some(ExportConfirm::Tip {
                kind,
                dont_show: false,
            });
        }
    }

    /// Warn when soft-merging into a non-empty export folder unless dismissed.
    pub(super) fn export_dir_chosen(&mut self, path: PathBuf, kind: ExportKind) {
        let (_, write_roots) = self.ws.package.resolve_export_paths(&path, kind);
        let would_merge = write_roots
            .iter()
            .any(|p| scene_builder_core::project::package::dir_nonempty(p));
        if would_merge && !self.prefs.hide_export_merge_warn {
            self.export_confirm = Some(ExportConfirm::Merge {
                path,
                kind,
                dont_show: false,
            });
        } else {
            self.start_export(path, kind);
        }
    }

    pub(super) fn start_export(&mut self, parent: PathBuf, kind: ExportKind) {
        let pack = self.ws.package.clone();
        let tx = self.job_tx.clone();
        self.job = JobUi {
            active: true,
            title: "Export".into(),
            message: "Starting…".into(),
            fraction: 0.0,
        };
        thread::spawn(move || {
            let progress = ChannelProgress::new(tx.clone());
            progress.set_title("Export");
            progress.set_message("Resolving paths…");
            progress.set_fraction(0.1);
            let (pack_root, _) = pack.resolve_export_paths(&parent, kind);
            progress.set_message(format!("Writing to {}…", pack_root.display()).as_str());
            progress.set_fraction(0.3);
            let result = pack.export_into(&pack_root, kind);
            match result {
                Ok(()) => {
                    progress.set_fraction(1.0);
                    progress.set_message("Done");
                    let _ = tx.send(JobEvent::Finished {
                        ok: true,
                        message: format!("Exported to {}", pack_root.display()),
                    });
                }
                Err(e) => {
                    let _ = tx.send(JobEvent::Finished {
                        ok: false,
                        message: e,
                    });
                }
            }
        });
    }

    pub(super) fn start_slal_pack_import(&mut self, dir: PathBuf) {
        let tx = self.job_tx.clone();
        self.job = JobUi {
            active: true,
            title: "Import SLAL pack".into(),
            message: "Scanning folder…".into(),
            fraction: 0.1,
        };
        thread::spawn(move || {
            let progress = ChannelProgress::new(tx.clone());
            progress.set_title("Import SLAL pack");
            progress.set_message("Reading pack…");
            match Package::from_slal_pack(dir, Some(&progress)) {
                Ok(pack) => {
                    let n = pack.scenes.len();
                    let _ = tx.send(JobEvent::PackageUpdated {
                        package: pack,
                        message: format!("Imported {n} scene(s) from SLAL pack"),
                        dirty: false,
                    });
                }
                Err(e) => {
                    let _ = tx.send(JobEvent::Finished {
                        ok: false,
                        message: e,
                    });
                }
            }
        });
    }

    pub(super) fn start_enrich_slanim(&mut self, paths: Vec<PathBuf>) {
        let mut pack = self.ws.package.clone();
        let tx = self.job_tx.clone();
        self.job = JobUi {
            active: true,
            title: "Enrich SLAnim".into(),
            message: "Reading sources…".into(),
            fraction: 0.2,
        };
        thread::spawn(move || {
            let progress = ChannelProgress::new(tx.clone());
            progress.set_title("Enrich SLAnim");
            progress.set_message("Applying…");
            match pack.enrich_from_slanim_paths(&paths) {
                Ok(summary) => {
                    let msg = summary.message();
                    let _ = tx.send(JobEvent::PackageUpdated {
                        package: pack,
                        message: msg,
                        dirty: true,
                    });
                }
                Err(e) => {
                    let _ = tx.send(JobEvent::Finished {
                        ok: false,
                        message: e,
                    });
                }
            }
        });
    }

    pub(super) fn start_enrich_fnis(&mut self, paths: Vec<PathBuf>) {
        let mut pack = self.ws.package.clone();
        let tx = self.job_tx.clone();
        self.job = JobUi {
            active: true,
            title: "Enrich FNIS".into(),
            message: "Reading AnimLists…".into(),
            fraction: 0.2,
        };
        thread::spawn(move || {
            let progress = ChannelProgress::new(tx.clone());
            progress.set_title("Enrich FNIS");
            progress.set_message("Applying…");
            match pack.enrich_from_fnis_paths(&paths) {
                Ok(summary) => {
                    let msg = summary.message_fnis();
                    let _ = tx.send(JobEvent::PackageUpdated {
                        package: pack,
                        message: msg,
                        dirty: true,
                    });
                }
                Err(e) => {
                    let _ = tx.send(JobEvent::Finished {
                        ok: false,
                        message: e,
                    });
                }
            }
        });
    }

    pub(super) fn poll_channels(&mut self, ctx: &Context) {
        while let Ok(ev) = self.job_rx.try_recv() {
            match ev {
                JobEvent::Progress {
                    title,
                    message,
                    fraction,
                } => {
                    self.job.active = true;
                    self.job.title = title;
                    self.job.message = message;
                    self.job.fraction = fraction;
                }
                JobEvent::Finished { ok, message } => {
                    self.job.active = false;
                    self.status = message.clone();
                    if !ok {
                        error!("{message}");
                    } else {
                        info!("{message}");
                    }
                }
                JobEvent::PackageUpdated {
                    package,
                    message,
                    dirty,
                } => {
                    self.ws.set_package(package, dirty);
                    self.graph.selected = None;
                    self.job.active = false;
                    self.stage_editor = None;
                    self.status = message;
                    if let Some(id) = self.ws.package.scenes.keys().next().cloned() {
                        self.select_scene(id);
                    }
                }
            }
            ctx.request_repaint();
        }

        while let Ok(ev) = self.dialog_rx.try_recv() {
            match ev {
                DialogResult::Open(path) => match Package::load_from_path(path) {
                    Ok(pack) => {
                        self.ws.package = pack;
                        self.ws.dirty = false;
                        self.stage_editor = None;
                        self.status = format!("Opened {}", self.ws.package.pack_path.display());
                        if let Some(id) = self.ws.package.scenes.keys().next().cloned() {
                            self.select_scene(id);
                        } else {
                            self.ws.selected_scene = None;
                            self.graph.selected = None;
                        }
                    }
                    Err(e) => {
                        self.status = format!("Open failed: {e}");
                        error!("{e}");
                    }
                },
                DialogResult::OpenSlal(path) => self.start_slal_pack_import(path),
                DialogResult::OpenOffset(path) => {
                    match self.ws.package.import_offset_from_path(path) {
                        Ok(()) => {
                            self.ws.dirty = true;
                            self.status = "Imported offsets".into();
                        }
                        Err(e) => {
                            self.status = format!("Offset import failed: {e}");
                            error!("{e}");
                        }
                    }
                }
                DialogResult::SaveAs(path) => match self.ws.package.write(path) {
                    Ok(()) => {
                        self.ws.dirty = false;
                        self.status = "Saved".into();
                    }
                    Err(e) => {
                        self.status = format!("Save failed: {e}");
                        error!("{e}");
                    }
                },
                DialogResult::ExportDir { path, kind } => self.export_dir_chosen(path, kind),
                DialogResult::EnrichSlanim(paths) => self.start_enrich_slanim(paths),
                DialogResult::EnrichFnis(paths) => self.start_enrich_fnis(paths),
                DialogResult::Cancelled => {}
            }
            ctx.request_repaint();
        }
    }
}
