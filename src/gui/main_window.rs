use eframe::egui;
use crate::app::model::SoftwareEntry;
use crate::app::db;
use crate::gui::add_wizard::AddWizard;
use crate::gui::edit_dialog::EditDialog;
use crate::gui::settings_window::SettingsWindow;
use rusqlite::Connection;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct MainWindow {
    entries: Vec<SoftwareEntry>,
    filter_text: String,
    selected_tag: Option<String>,
    all_tags: Vec<String>,
    show_add_wizard: bool,
    add_wizard: Option<AddWizard>,
    edit_dialog: Option<EditDialog>,
    settings_window: Option<SettingsWindow>,
    conn: Arc<Mutex<Connection>>, // 数据库连接，跨线程共享
}

impl MainWindow {
    pub fn new(conn: Connection) -> Self {
        let conn = Arc::new(Mutex::new(conn));
        // 初始加载数据
        let entries = Self::load_entries(&conn);
        let all_tags = Self::extract_all_tags(&entries);
        Self {
            entries,
            filter_text: String::new(),
            selected_tag: None,
            all_tags,
            show_add_wizard: false,
            add_wizard: None,
            edit_dialog: None,
            settings_window: None,
            conn,
        }
    }

    fn load_entries(conn: &Arc<Mutex<Connection>>) -> Vec<SoftwareEntry> {
        if let Ok(conn) = conn.try_lock() {
            db::get_all_software(&conn).unwrap_or_default()
        } else {
            vec![]
        }
    }

    fn extract_all_tags(entries: &[SoftwareEntry]) -> Vec<String> {
        let mut tags = std::collections::HashSet::new();
        for e in entries {
            tags.extend(e.tags.iter().cloned());
        }
        let mut tags: Vec<String> = tags.into_iter().collect();
        tags.sort();
        tags
    }

    fn filtered_entries(&self) -> Vec<&SoftwareEntry> {
        self.entries
            .iter()
            .filter(|e| {
                let text_match = self.filter_text.is_empty()
                    || e.name.to_lowercase().contains(&self.filter_text.to_lowercase())
                    || e.notes.to_lowercase().contains(&self.filter_text.to_lowercase())
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&self.filter_text.to_lowercase()));
                let tag_match = self.selected_tag.is_none()
                    || e.tags.contains(self.selected_tag.as_ref().unwrap());
                text_match && tag_match
            })
            .collect()
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 处理添加向导
        if self.show_add_wizard {
            let mut wizard = self.add_wizard.take().unwrap_or_else(AddWizard::new);
            if let Some(entry) = wizard.ui(ctx) {
                // 保存到数据库
                if let Ok(conn) = self.conn.try_lock() {
                    let _ = db::insert_software(&conn, &entry);
                }
                // 刷新列表
                self.entries = Self::load_entries(&self.conn);
                self.all_tags = Self::extract_all_tags(&self.entries);
                self.show_add_wizard = false;
            } else {
                self.add_wizard = Some(wizard);
            }
        }

        // 处理编辑对话框
        if let Some(dialog) = &mut self.edit_dialog {
            if let Some(updated) = dialog.ui(ctx) {
                if let Ok(conn) = self.conn.try_lock() {
                    let _ = db::update_software(&conn, &updated);
                }
                // 刷新列表
                self.entries = Self::load_entries(&self.conn);
                self.all_tags = Self::extract_all_tags(&self.entries);
                self.edit_dialog = None;
            }
        }

        // 处理设置窗口
        if let Some(window) = &mut self.settings_window {
            if let Some(settings) = window.ui(ctx) {
                // 保存设置到数据库
                if let Ok(conn) = self.conn.try_lock() {
                    let _ = db::save_setting(&conn, "github_token", settings.github_token.as_deref().unwrap_or(""));
                    let _ = db::save_setting(&conn, "auto_check_interval", &settings.auto_check_interval_hours.to_string());
                    let _ = db::save_setting(&conn, "download_dir", settings.download_dir.as_deref().unwrap_or(""));
                }
                self.settings_window = None;
            }
        }

        // 主界面
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("➕ 添加软件").clicked() {
                    self.show_add_wizard = true;
                    self.add_wizard = Some(AddWizard::new());
                }
                if ui.button("⚙️ 设置").clicked() {
                    // 从数据库加载当前设置
                    let settings = if let Ok(conn) = self.conn.try_lock() {
                        crate::app::model::Settings::load_from_db(&conn).unwrap_or_default()
                    } else {
                        Default::default()
                    };
                    self.settings_window = Some(SettingsWindow::new(settings));
                }
                if ui.button("🔄 检查更新").clicked() {
                    // 触发更新检查（异步）
                }
            });
        });

        egui::SidePanel::left("tag_panel").show(ctx, |ui| {
            ui.heading("标签分类");
            ui.add_space(5.0);
            if ui.button("全部").clicked() {
                self.selected_tag = None;
            }
            for tag in &self.all_tags {
                let mut selected = self.selected_tag.as_ref() == Some(tag);
                if ui.checkbox(&mut selected, tag).changed() && selected {
                    self.selected_tag = Some(tag.clone());
                } else if !selected && self.selected_tag.as_ref() == Some(tag) {
                    self.selected_tag = None;
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("🔍 搜索:");
                ui.text_edit_singleline(&mut self.filter_text);
                if !self.filter_text.is_empty() && ui.button("×").clicked() {
                    self.filter_text.clear();
                }
            });

            egui::ScrollArea::vertical().show(ui, |ui| {
			let entries: Vec<SoftwareEntry> = self.filtered_entries().into_iter().cloned().collect();
			for entry in entries {
				ui.group(|ui| {
					ui.set_width(ui.available_width());
					ui.horizontal(|ui| {
						ui.heading(&entry.name);
						ui.label(format!("当前: {}", entry.current_version));
						if let Some(latest) = &entry.latest_version {
							ui.label(format!("最新: {}", latest));
							if latest != &entry.current_version {
								ui.colored_label(egui::Color32::YELLOW, "有新版本!");
							}
						}
						ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
							if ui.button("✏️ 编辑").clicked() {
								self.edit_dialog = Some(EditDialog::new(entry.clone()));
							}
							if ui.button("🌐 仓库").clicked() {
								let url = format!("https://github.com/{}/{}", entry.repo_owner, entry.repo_name);
								let _ = open::that(url);
							}
							if ui.button("▶ 打开").clicked() {
								if let Some(path) = &entry.executable_path {
									let _ = open::that(path);
								}
							}
							if ui.button("🔄 更新").clicked() {
								// TODO: 触发单个软件的更新
							}
						});
					});
					ui.horizontal(|ui| {
						if let Some(path) = &entry.install_path {
							ui.label(format!("📁 {}", path));
						}
						if !entry.notes.is_empty() {
							ui.label(format!("📝 {}", entry.notes));
						}
						if !entry.tags.is_empty() {
							ui.label(format!("🏷️ {}", entry.tags.join(", ")));
						}
					});
				});
			}
            });
        });
    }
}

impl MainWindow {
    fn render_entry(&mut self, ui: &mut egui::Ui, entry: &SoftwareEntry) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.heading(&entry.name);
                ui.label(format!("当前: {}", entry.current_version));
                if let Some(latest) = &entry.latest_version {
                    ui.label(format!("最新: {}", latest));
                    if latest != &entry.current_version {
                        ui.colored_label(egui::Color32::YELLOW, "有新版本!");
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✏️ 编辑").clicked() {
                        self.edit_dialog = Some(EditDialog::new(entry.clone()));
                    }
                    if ui.button("🌐 仓库").clicked() {
                        let url = format!("https://github.com/{}/{}", entry.repo_owner, entry.repo_name);
                        let _ = open::that(url);
                    }
                    if ui.button("▶ 打开").clicked() {
                        if let Some(path) = &entry.executable_path {
                            let _ = open::that(path);
                        }
                    }
                    if ui.button("🔄 更新").clicked() {
                        // 触发单个软件的更新
                    }
                });
            });
            ui.horizontal(|ui| {
                if let Some(path) = &entry.install_path {
                    ui.label(format!("📁 {}", path));
                }
                if !entry.notes.is_empty() {
                    ui.label(format!("📝 {}", entry.notes));
                }
                if !entry.tags.is_empty() {
                    ui.label(format!("🏷️ {}", entry.tags.join(", ")));
                }
            });
        });
    }
}
