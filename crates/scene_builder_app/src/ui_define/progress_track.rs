use crate::jobs::JobUi;

pub fn show_job_progress(ctx: &egui::Context, job: &JobUi) {
    if !job.active {
        return;
    }
    egui::Window::new(&job.title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(&job.message);
            let progress = egui::ProgressBar::new(job.fraction)
                .show_percentage()
                .animate(true);
            ui.add(progress);
        });
}
