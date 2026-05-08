use eframe::egui;
use crate::app::model::SoftwareEntry;
use rfd::FileDialog;

pub struct EditDialog {
    entry: SoftwareEntry,
    open: bool,
    tags_string: String,
	show_error_dialog: bool,
	error_message: String,
}

impl EditDialog {
    pub fn new(entry: SoftwareEntry) -> Self {
        let tags_string = entry.tags.join(", ");
        Self {
            entry,
            open: true,
            tags_string,
			show_error_dialog: false,
			error_message: String::new(),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<SoftwareEntry> {
        let mut result = None;
        if self.open {
            egui::Window::new("编辑软件信息")
                .default_width(450.0)
                .show(ctx, |ui| {
                    // 名称（原名）
                    ui.label("名称（必填）:");
                    ui.text_edit_singleline(&mut self.entry.name);

                    // 别名
                    ui.label("别名:");
                    ui.text_edit_singleline(&mut self.entry.alias);

                    // 仓库信息
					ui.label("GitHub 仓库 URL:");
					ui.text_edit_singleline(&mut self.entry.repo_url);

                    // 版本信息
                    ui.horizontal(|ui| {
                        ui.label("当前版本:");
                        ui.text_edit_singleline(&mut self.entry.current_version);
                    });
                    ui.horizontal(|ui| {
                        ui.label("最新版本:");
                        let mut latest = self.entry.latest_version.clone().unwrap_or_default();
                        if ui.text_edit_singleline(&mut latest).changed() {
                            self.entry.latest_version = if latest.is_empty() { None } else { Some(latest) };
                        }
                    });

                    // 软件包
                    ui.horizontal(|ui| {
                        ui.label("软件包:");
                        ui.text_edit_singleline(&mut self.entry.asset_name);
                    });

                    // 安装路径
                    ui.label("根路径:");
                    ui.horizontal(|ui| {
                        let mut path = self.entry.install_path.clone().unwrap_or_default();
                        if ui.text_edit_singleline(&mut path).changed() {
                            self.entry.install_path = if path.is_empty() { None } else { Some(path) };
                        }
                        if ui.button("浏览...").clicked() {
                            if let Some(selected) = FileDialog::new().pick_folder() {
                                let path = selected.display().to_string();
                                self.entry.install_path = Some(path.clone());
                                // 自动检测可执行文件
                                if let Some(exe) = crate::utils::path::guess_main_exe(&path) {
                                    self.entry.executable_path = Some(exe.display().to_string());
                                }
                            }
                        }
                    });

                    // 可执行文件
                    ui.horizontal(|ui| {
						ui.label("目标文件:");
                        if ui.button("文件").clicked() {
                            if let Some(path) = FileDialog::new()
                                // .add_filter("exe", &["exe"])
                                .set_directory(self.entry.install_path.as_deref().unwrap_or(""))
                                .pick_file()
                            {
                                self.entry.executable_path = Some(path.display().to_string());
                            }
                        }
						if ui.button("文件夹").clicked() {
                            if let Some(path) = FileDialog::new()
                                .set_directory(self.entry.install_path.as_deref().unwrap_or(""))
                                .pick_folder()
                            {
                                self.entry.executable_path = Some(path.display().to_string());
                            }
                        }
                    });
					let mut exe = self.entry.executable_path.clone().unwrap_or_default();
					if ui.text_edit_singleline(&mut exe).changed() {
						self.entry.executable_path = if exe.is_empty() { None } else { Some(exe) };
					};

                    // 备注
                    ui.label("备注:");
                    ui.text_edit_multiline(&mut self.entry.notes);

                    // 标签
                    ui.label("标签 (逗号分隔):");
                    ui.text_edit_singleline(&mut self.tags_string);

                    ui.horizontal(|ui| {
                        if ui.button("保存").clicked() {
							if self.entry.name.trim().is_empty() {
								self.error_message = "软件名称不能为空".to_string();
								self.show_error_dialog = true;
								return; // 直接返回，不继续构建条目
							}

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
			// 错误弹窗（独立窗口）
			if self.show_error_dialog {
				egui::Window::new("错误")
					.collapsible(false)
					.resizable(false)
					.anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
					.show(ctx, |ui| {
						ui.label(&self.error_message);
						ui.horizontal(|ui| {
							if ui.button("确定").clicked() {
								self.show_error_dialog = false;
							}
						});
					});
			}
        }
        result
    }
}
