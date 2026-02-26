use eframe::egui;
use crate::app::model::SoftwareEntry;
use rfd::FileDialog;

pub struct EditDialog {
    entry: SoftwareEntry,
    open: bool,
    tags_string: String,
    github_url: String,
}

impl EditDialog {
    pub fn new(entry: SoftwareEntry) -> Self {
        let tags_string = entry.tags.join(", ");
        let github_url = format!("https://github.com/{}/{}", entry.repo_owner, entry.repo_name);
        Self {
            entry,
            open: true,
            tags_string,
            github_url,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<SoftwareEntry> {
        let mut result = None;
        if self.open {
            egui::Window::new("编辑软件信息")
                .default_width(400.0)
                .show(ctx, |ui| {
                    ui.label("别名:");
                    ui.text_edit_singleline(&mut self.entry.alias);

                    ui.horizontal(|ui| {
                        ui.label("GitHub 仓库:");
                        ui.hyperlink(&self.github_url);
                        if ui.button("打开").clicked() {
                            let url = self.github_url.clone();
                            std::thread::spawn(move || {
                                let _ = open::that(url);
                            });
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("当前版本:");
                        ui.text_edit_singleline(&mut self.entry.current_version);
                    });

                    ui.horizontal(|ui| {
                        ui.label("软件包:");
                        ui.text_edit_singleline(&mut self.entry.asset_name);
                    });

                    ui.label("安装路径:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(self.entry.install_path.get_or_insert_with(String::new));
                        if ui.button("浏览...").clicked() {
                            if let Some(path) = FileDialog::new().pick_folder() {
                                *self.entry.install_path.get_or_insert_with(String::new) = path.display().to_string();
                            }
                        }
                    });

                    ui.label("可执行文件:");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(self.entry.executable_path.get_or_insert_with(String::new));
                        if ui.button("浏览...").clicked() {
                            if let Some(path) = FileDialog::new().add_filter("exe", &["exe"]).pick_file() {
                                *self.entry.executable_path.get_or_insert_with(String::new) = path.display().to_string();
                            }
                        }
                    });

                    ui.label("备注:");
                    ui.text_edit_multiline(&mut self.entry.notes);

                    ui.label("标签 (逗号分隔):");
                    ui.text_edit_singleline(&mut self.tags_string);

                    ui.horizontal(|ui| {
                        if ui.button("保存").clicked() {
                            self.entry.tags = self.tags_string
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                            self.entry.updated_at = chrono::Local::now();
                            result = Some(self.entry.clone());
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
