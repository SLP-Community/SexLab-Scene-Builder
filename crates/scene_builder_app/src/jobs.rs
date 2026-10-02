use scene_builder_core::Progress;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub enum JobEvent {
    Progress {
        title: String,
        message: String,
        fraction: f32,
    },
    Finished {
        ok: bool,
        message: String,
    },
    PackageUpdated {
        package: scene_builder_core::project::package::Package,
        message: String,
        /// False for a fresh SLAL import (new baseline). True for enrich-in-place.
        dirty: bool,
    },
}

pub struct ChannelProgress {
    tx: Sender<JobEvent>,
    title: std::sync::Mutex<String>,
    message: std::sync::Mutex<String>,
}

impl ChannelProgress {
    pub fn new(tx: Sender<JobEvent>) -> Self {
        Self {
            tx,
            title: std::sync::Mutex::new(String::new()),
            message: std::sync::Mutex::new(String::new()),
        }
    }

    fn emit(&self, fraction: f32) {
        let title = self.title.lock().map(|g| g.clone()).unwrap_or_default();
        let message = self.message.lock().map(|g| g.clone()).unwrap_or_default();
        let _ = self.tx.send(JobEvent::Progress {
            title,
            message,
            fraction,
        });
    }
}

impl Progress for ChannelProgress {
    fn set_title(&self, title: &str) {
        if let Ok(mut g) = self.title.lock() {
            *g = title.to_string();
        }
        self.emit(0.0);
    }

    fn set_message(&self, msg: &str) {
        if let Ok(mut g) = self.message.lock() {
            *g = msg.to_string();
        }
        self.emit(0.0);
    }

    fn set_fraction(&self, fraction: f32) {
        self.emit(fraction.clamp(0.0, 1.0));
    }
}

#[derive(Debug, Default)]
pub struct JobUi {
    pub active: bool,
    pub title: String,
    pub message: String,
    pub fraction: f32,
}
