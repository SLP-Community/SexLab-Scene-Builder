pub trait Progress: Send {
    fn set_title(&self, title: &str);
    fn set_message(&self, msg: &str);
    fn set_fraction(&self, fraction: f32); // 0.0..=1.0
}

pub struct NullProgress;

impl Progress for NullProgress {
    fn set_title(&self, _title: &str) {}
    fn set_message(&self, _msg: &str) {}
    fn set_fraction(&self, _fraction: f32) {}
}
