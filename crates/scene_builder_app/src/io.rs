use scene_builder_core::project::package::ExportKind;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;

pub enum DialogResult {
    Open(PathBuf),
    OpenSlal(PathBuf),
    OpenOffset(PathBuf),
    SaveAs(PathBuf),
    ExportDir { path: PathBuf, kind: ExportKind },
    EnrichSlanim(Vec<PathBuf>),
    EnrichFnis(Vec<PathBuf>),
    Cancelled,
}

pub fn spawn_open(tx: Sender<DialogResult>) {
    thread::spawn(move || {
        let path = rfd::FileDialog::new()
            .add_filter("SLSB Project", &["json"])
            .pick_file();
        let _ = match path {
            Some(p) => tx.send(DialogResult::Open(p)),
            None => tx.send(DialogResult::Cancelled),
        };
    });
}

pub fn spawn_slal(tx: Sender<DialogResult>) {
    thread::spawn(move || {
        let path = rfd::FileDialog::new()
            .set_title("Import SLAL pack (folder)")
            .pick_folder();
        let _ = match path {
            Some(p) => tx.send(DialogResult::OpenSlal(p)),
            None => tx.send(DialogResult::Cancelled),
        };
    });
}

pub fn spawn_offset(tx: Sender<DialogResult>) {
    thread::spawn(move || {
        let path = rfd::FileDialog::new()
            .add_filter("Offset YAML", &["yaml", "yml"])
            .pick_file();
        let _ = match path {
            Some(p) => tx.send(DialogResult::OpenOffset(p)),
            None => tx.send(DialogResult::Cancelled),
        };
    });
}

pub fn spawn_save_as(tx: Sender<DialogResult>, suggested: String) {
    thread::spawn(move || {
        let path = rfd::FileDialog::new()
            .add_filter("SLSB Project", &["json"])
            .set_file_name(&suggested)
            .save_file();
        let _ = match path {
            Some(p) => tx.send(DialogResult::SaveAs(p)),
            None => tx.send(DialogResult::Cancelled),
        };
    });
}

pub fn spawn_export(tx: Sender<DialogResult>, kind: ExportKind) {
    thread::spawn(move || {
        let path = rfd::FileDialog::new().pick_folder();
        let _ = match path {
            Some(p) => tx.send(DialogResult::ExportDir { path: p, kind }),
            None => tx.send(DialogResult::Cancelled),
        };
    });
}

pub fn spawn_enrich_slanim(tx: Sender<DialogResult>) {
    thread::spawn(move || {
        let paths = rfd::FileDialog::new()
            .add_filter("Source / text", &["txt", "json", "xml"])
            .pick_files();
        let _ = match paths {
            Some(p) if !p.is_empty() => tx.send(DialogResult::EnrichSlanim(p)),
            _ => tx.send(DialogResult::Cancelled),
        };
    });
}

pub fn spawn_enrich_fnis(tx: Sender<DialogResult>) {
    thread::spawn(move || {
        let paths = rfd::FileDialog::new()
            .add_filter("FNIS AnimList", &["txt"])
            .pick_files();
        let _ = match paths {
            Some(p) if !p.is_empty() => tx.send(DialogResult::EnrichFnis(p)),
            _ => tx.send(DialogResult::Cancelled),
        };
    });
}
