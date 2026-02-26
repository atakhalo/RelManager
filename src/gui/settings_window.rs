use eframe::egui;
use crate::app::model::Settings;

pub struct SettingsWindow {
    settings: Settings,
    open: bool,
}

impl SettingsWindow {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            open: true,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<Settings> {
        let mut result = None;
        if self.open {
            egui::Window::new("设置")
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label("GitHub Token (可选，提高 API 限额):");
                    ui.text_edit_singleline(self.settings.github_token.get_or_insert_with(String::new));

                    ui.label("自动检查更新间隔 (小时，0=禁用):");
					ui.add(egui::DragValue::new(&mut self.settings.auto_check_interval_hours));

                    ui.label("下载目录 (可选):");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(self.settings.download_dir.get_or_insert_with(String::new));
                        if ui.button("浏览...").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                *self.settings.download_dir.get_or_insert_with(String::new) = path.display().to_string();
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("保存").clicked() {
                            result = Some(self.settings.clone());
                            self.open = false;
                        }
                        if ui.button("取消").clicked() {
                            self.open = false;
                        }
                    });
                });
        }
        result
    }
}
