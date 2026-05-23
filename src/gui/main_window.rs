use eframe::egui;
use crate::app::model::{SoftwareEntry, TagGroup};
use crate::app::db;
use crate::gui::edit_dialog::EditDialog;
use crate::gui::add_wizard::AddWizard;
use crate::gui::settings_window::SettingsWindow;
use crate::gui::tag_group_manager::TagGroupManager;
use rusqlite::Connection;
use std::os::windows::process::CommandExt;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use crate::app::updater::Updater;
use chrono::{Local, DateTime, Duration};
use anyhow::anyhow;
use crate::app::model::Settings;

use tokio::task;



pub struct MainWindow {
    entries: Vec<SoftwareEntry>,
    filter_text: String,
    selected_tags: HashSet<String>,      // 多选标签
    all_tags: Vec<String>,
    tag_groups: Vec<TagGroup>,            // 所有标签组
    show_add_wizard: bool,
    add_wizard: Option<AddWizard>,
    edit_dialog: Option<EditDialog>,
    settings_window: Option<SettingsWindow>,
    show_tag_group_manager: bool,
    tag_group_manager: Option<TagGroupManager>,
    conn: Arc<Mutex<Connection>>,
    // 删除确认相关
    show_delete_confirm: bool,
    pending_delete_id: Option<i64>,
    pending_delete_name: String,

	updater: Option<Updater>,
    check_all_in_progress: bool,
    check_single_in_progress: HashSet<i64>,
    last_check_time: Option<DateTime<Local>>,
    auto_check_interval_hours: u64,
    show_update_toast: bool,
    update_messages: Vec<String>,
	first_frame: bool,
}

impl MainWindow {
    pub fn new(conn: Connection) -> Self {
        let conn = Arc::new(Mutex::new(conn));
        let entries = Self::load_entries(&conn);
        let all_tags = Self::extract_all_tags(&entries);
        let tag_groups = Self::load_tag_groups(&conn);

		// 从数据库加载设置
		let (token, interval, last_check) = if let Ok(conn_guard) = conn.lock() {
			let settings = Settings::load_from_db(&conn_guard).unwrap_or_default();
			(settings.github_token, settings.auto_check_interval_hours, settings.last_check_time)
		} else {
			(None, 24, None)
		};

        Self {
            entries,
            filter_text: String::new(),
            selected_tags: HashSet::new(),
            all_tags,
            tag_groups,
            show_add_wizard: false,
            add_wizard: None,
            edit_dialog: None,
            settings_window: None,
            show_tag_group_manager: false,
            tag_group_manager: None,
            conn,
            show_delete_confirm: false,
            pending_delete_id: None,
            pending_delete_name: String::new(),
			updater: Some(Updater::new(token)),
			check_all_in_progress: false,
			check_single_in_progress: HashSet::new(),
			last_check_time: last_check,
			auto_check_interval_hours: interval,
			show_update_toast: false,
			update_messages: Vec::new(),
			first_frame: true,
        }
    }

    fn load_entries(conn: &Arc<Mutex<Connection>>) -> Vec<SoftwareEntry> {
        if let Ok(conn) = conn.lock() {
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

    fn load_tag_groups(conn: &Arc<Mutex<Connection>>) -> Vec<TagGroup> {
        if let Ok(conn) = conn.lock() {
            db::get_all_tag_groups(&conn).unwrap_or_default()
        } else {
            vec![]
        }
    }

    fn filtered_entries(&self) -> Vec<&SoftwareEntry> {
        self.entries
            .iter()
            .filter(|e| {
                let text_match = self.filter_text.is_empty()
                    || e.name.to_lowercase().contains(&self.filter_text.to_lowercase())
                    || e.alias.to_lowercase().contains(&self.filter_text.to_lowercase())
                    || e.notes.to_lowercase().contains(&self.filter_text.to_lowercase())
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&self.filter_text.to_lowercase()));
                if self.selected_tags.is_empty() {
                    text_match
                } else {
                    // 条目必须包含至少一个选中的标签
                    text_match && e.tags.iter().any(|t| self.selected_tags.contains(t))
                }
            })
            .collect()
    }

	/// 手动检查所有软件的更新
	fn check_all_updates(&mut self, ctx: &egui::Context) {
		if self.check_all_in_progress {
			return;
		}
		self.check_all_in_progress = true;

		let updater = self.updater.clone().expect("Updater not initialized");
		let conn = self.conn.clone();
		let ctx = ctx.clone();

		tokio::spawn(async move {
			let conn_for_first = conn.clone();

			// 定义一个异步块返回 Result，便于统一处理错误
			let result: Result<(String, Vec<String>), anyhow::Error> = async {
				// 1. 在阻塞线程中获取所有软件条目
				let entries = task::spawn_blocking(move || {
					let conn_guard = conn_for_first.lock().unwrap();
					db::get_all_software(&conn_guard).unwrap_or_default()
				})
				.await
				.map_err(|e| anyhow::anyhow!("获取软件列表失败: {}", e))?;

				// 2. 逐个检查更新（异步）
				let mut updated_entries = Vec::new();
				for entry in entries {
					if let Ok(Some(latest)) = updater.check_for_updates(&entry).await {
						updated_entries.push((entry.id.unwrap(), entry.name, latest));
					}
				}

				// 3. 在阻塞线程中更新数据库
				let conn_for_second = conn.clone();
				let updated_entries_clone = updated_entries.clone();
				task::spawn_blocking(move || {
					let conn_guard = conn_for_second.lock().unwrap();
					for (id, _, latest) in &updated_entries_clone {
						if let Ok(Some(mut entry)) = db::get_software_by_id(&conn_guard, *id) {
							entry.latest_version = Some(latest.clone());
							entry.updated_at = Local::now();
							let _ = db::update_software(&conn_guard, &entry);
						}
					}
					// 更新最后检查时间
					let now = Local::now();
					let _ = db::save_setting(&conn_guard, "last_check_time", &now.to_rfc3339());
					Ok::<_, anyhow::Error>(())
				})
				.await
				.map_err(|e| anyhow::anyhow!("数据库更新任务失败: {}", e))??; // 先处理 spawn_blocking 错误，再处理内部 Result

				// 4. 准备结果消息
				let (msg, updated_names) = if updated_entries.is_empty() {
					("所有软件已是最新".to_string(), vec![])
				} else {
					let names: Vec<String> = updated_entries
						.iter()
						.map(|(_, name, ver)| format!("{} -> {}", name, ver))
						.collect();
					(format!("发现 {} 个更新", updated_entries.len()), names)
				};

				Ok((msg, updated_names))
			}
			.await;

			// 将结果存储到 UI 上下文
			match result {
				Ok((msg, updated_names)) => {
					ctx.memory_mut(|mem| {
						mem.data.insert_temp("update_result".into(), (msg, updated_names));
					});
				}
				Err(e) => {
					let err_msg = format!("检查更新失败: {}", e);
					ctx.memory_mut(|mem| {
						mem.data.insert_temp::<(String, Vec<String>)>("update_result".into(), (err_msg, vec![]));
					});
				}
			}
			ctx.request_repaint();
		});
	}

    /// 检查单个软件更新
    fn check_single_update(&mut self, entry_id: i64, ctx: &egui::Context) {
        if self.check_single_in_progress.contains(&entry_id) {
            return;
        }
        self.check_single_in_progress.insert(entry_id);

        // 查找该条目
        let entry_opt = self.entries.iter().find(|e| e.id == Some(entry_id)).cloned();
        if let Some(entry) = entry_opt {
            let updater = self.updater.clone().expect("Updater not initialized");
            let conn = self.conn.clone();
            let ctx = ctx.clone();

            tokio::spawn(async move {
                let result = updater.check_for_updates(&entry).await;
                let msg = match result {
                    Ok(Some(latest)) => {
                        // 更新数据库
                        if let Ok(conn_guard) = conn.lock() {
                            let mut updated_entry = entry.clone();
                            updated_entry.latest_version = Some(latest.clone());
                            updated_entry.updated_at = chrono::Local::now();
                            let _ = db::update_software(&conn_guard, &updated_entry);
                        }
                        format!("{} 有新版本: {}", entry.name, latest)
                    }
                    Ok(None) => format!("{} 已是最新", entry.name),
                    Err(e) => format!("{} 检查失败: {}", entry.name, e),
                };
				ctx.memory_mut(|mem| {
					mem.data.insert_temp("single_update_result".into(), msg);
				});
                ctx.request_repaint();
            });
        } else {
            self.check_single_in_progress.remove(&entry_id);
        }
    }
}

impl eframe::App for MainWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
		if self.first_frame {
			self.first_frame = false;
			// 检查是否需要自动更新
			if self.auto_check_interval_hours > 0 {
				let now = Local::now();
				let should_check = match self.last_check_time {
					Some(last) => (now - last).num_hours() >= self.auto_check_interval_hours as i64,
					None => true,
				};
				if should_check {
					self.check_all_updates(ctx);
				}
			}
		}

        // 处理添加向导
        if self.show_add_wizard {
            let mut wizard = self.add_wizard.take().unwrap_or_else(AddWizard::new);
            if let Some(entry) = wizard.ui(ctx) {
                if let Ok(conn) = self.conn.lock() {
                    let _ = db::insert_software(&conn, &entry);
                }
                self.entries = Self::load_entries(&self.conn);
                self.all_tags = Self::extract_all_tags(&self.entries);
                self.show_add_wizard = false;
                ctx.request_repaint();
            } else {
                self.add_wizard = Some(wizard);
            }
        }

        // 处理编辑对话框
        if let Some(dialog) = &mut self.edit_dialog {
            if let Some(updated) = dialog.ui(ctx) {
                if let Ok(conn) = self.conn.lock() {
                    let _ = db::update_software(&conn, &updated);
                }
                self.entries = Self::load_entries(&self.conn);
                self.all_tags = Self::extract_all_tags(&self.entries);
                self.edit_dialog = None;
                ctx.request_repaint();
            }
        }

        // 处理设置窗口
        if let Some(window) = &mut self.settings_window {
            if let Some(settings) = window.ui(ctx) {
                if let Ok(conn) = self.conn.lock() {
                    let _ = db::save_setting(&conn, "github_token", settings.github_token.as_deref().unwrap_or(""));
                    let _ = db::save_setting(&conn, "auto_check_interval", &settings.auto_check_interval_hours.to_string());
                    let _ = db::save_setting(&conn, "download_dir", settings.download_dir.as_deref().unwrap_or(""));
                }
                self.settings_window = None;
            }
        }

        // 处理标签组管理器
        if self.show_tag_group_manager {
            let mut manager = self.tag_group_manager.take().unwrap_or_else(|| TagGroupManager::new(self.conn.clone()));
            if let Some(groups) = manager.ui(ctx) {
                self.tag_groups = groups;
                self.show_tag_group_manager = false;
                ctx.request_repaint();
            } else {
                self.tag_group_manager = Some(manager);
            }
        }

        // 删除确认弹窗
        if self.show_delete_confirm {
            egui::Window::new("确认删除")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("确定要删除 \"{}\" 吗？此操作不可撤销。", self.pending_delete_name));
                    ui.horizontal(|ui| {
                        if ui.button("确认").clicked() {
                            if let Some(id) = self.pending_delete_id {
                                if let Ok(conn) = self.conn.lock() {
                                    let _ = db::delete_software(&conn, id);
                                }
                                self.entries = Self::load_entries(&self.conn);
                                self.all_tags = Self::extract_all_tags(&self.entries);
                                self.show_delete_confirm = false;
                                self.pending_delete_id = None;
                                self.pending_delete_name.clear();
                                ctx.request_repaint();
                            }
                        }
                        if ui.button("取消").clicked() {
                            self.show_delete_confirm = false;
                            self.pending_delete_id = None;
                            self.pending_delete_name.clear();
                        }
                    });
                });
        }

        // 顶部菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("➕ 添加软件").clicked() {
                    self.show_add_wizard = true;
                    self.add_wizard = Some(AddWizard::new());
                }
                if ui.button("⚙️ 设置").clicked() {
                    let settings = if let Ok(conn) = self.conn.lock() {
                        crate::app::model::Settings::load_from_db(&conn).unwrap_or_default()
                    } else {
                        Default::default()
                    };
                    self.settings_window = Some(SettingsWindow::new(settings));
                }
                if ui.button("🏷️ 标签组").clicked() {
                    self.show_tag_group_manager = true;
                }
                if ui.button("🔄 批量检查更新").clicked() {
                    self.check_all_updates(ctx);
                }
            });
        });

		// 左侧标签面板
		egui::SidePanel::left("tag_panel").show(ctx, |ui| {
			ui.heading("标签分类");
			ui.add_space(5.0);
			ui.horizontal(|ui| {
				if ui.button("清除所有").clicked() {
					self.selected_tags.clear();
				}
			});
			ui.separator();

			// 标签组区域
			if !self.tag_groups.is_empty() {
				ui.label("标签组:");
				for group in &self.tag_groups {
					ui.horizontal(|ui| {
						// 将组标签转换为 HashSet 进行集合比较（忽略顺序）
						let group_tags: std::collections::HashSet<String> = group.tags.iter().cloned().collect();
						let selected_tags = &self.selected_tags;
						let is_selected_group = selected_tags == &group_tags;

						// 显示组名，如果完全匹配则加勾选标记
						if is_selected_group {
							ui.label(format!("{} ✔", group.name));
						} else {
							ui.label(&group.name);
						}

						if ui.button("应用").clicked() {
							// 清除当前选中，然后应用该组所有标签
							self.selected_tags.clear();
							for tag in &group.tags {
								self.selected_tags.insert(tag.clone());
							}
						}
					});
				}
				ui.separator();
			}

			// 多选标签（保持不变）
			ui.label("标签:");
			egui::ScrollArea::vertical().show(ui, |ui| {
				for tag in &self.all_tags {
					let mut selected = self.selected_tags.contains(tag);
					if ui.checkbox(&mut selected, tag).changed() {
						if selected {
							self.selected_tags.insert(tag.clone());
						} else {
							self.selected_tags.remove(tag);
						}
					}
				}
			});
		});

        // 中央主面板
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

                        // 判断是否配置了 Git 仓库地址
                        let has_repo = !entry.repo_url.is_empty();

                        // 第一行：名称（带别名逻辑）+ 版本信息 + 操作按钮
                        ui.horizontal(|ui| {
                            if !entry.alias.is_empty() {
                                ui.heading(&entry.alias);
                                ui.colored_label(egui::Color32::GRAY, format!("({})", entry.name));
                            } else {
                                ui.heading(&entry.name);
                            }

                            // 只有配置了 Git 地址才显示版本信息
                            if has_repo {
                                ui.label(format!("当前: {}", entry.current_version));
                                if let Some(latest) = &entry.latest_version {
                                    ui.label(format!("最新: {}", latest));
                                    if latest != &entry.current_version {
                                        ui.colored_label(egui::Color32::from_rgb(255, 180, 80), "有新版本!");
                                    }
                                }
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("🗑️ 删除").clicked() {
                                    if let Some(id) = entry.id {
                                        self.pending_delete_id = Some(id);
                                        self.pending_delete_name = if !entry.alias.is_empty() {
                                            format!("{} ({})", entry.alias, entry.name)
                                        } else {
                                            entry.name.clone()
                                        };
                                        self.show_delete_confirm = true;
                                    }
                                }
                                if ui.button("✏️ 编辑").clicked() {
                                    self.edit_dialog = Some(EditDialog::new(entry.clone()));
                                }
                                // 路径按钮：如果 executable_path 存在且是文件，则显示
                                if let Some(exe_path) = &entry.executable_path {
                                    let p = std::path::Path::new(exe_path);
                                    if p.is_file() {
                                        if ui.button("📂 路径").clicked() {
                                            if let Some(parent) = p.parent() {
                                                let _ = open::that(parent);
                                            }
                                        }
                                    }
                                }
                                // 只有配置了 Git 地址才显示仓库和检查更新按钮
                                if has_repo {
                                    if ui.button("🌐 仓库").clicked() {
                                        let _ = open::that(&entry.repo_url);
                                    }
                                    if ui.button("🔄 检查更新").clicked() {
                                        if let Some(id) = entry.id {
                                            self.check_single_update(id, ctx);
                                        }
                                    }
                                }
                                // 打开按钮始终显示（在所有按钮的最左边）
                                if ui.button("▶ 打开").clicked() {
                                    if let Some(path) = &entry.executable_path {
										if path.ends_with(".bat") {
											if let Some(bat_path_str) = &entry.executable_path {
												let bat_path = std::path::Path::new(bat_path_str);
												let full_command = format!(r#"start "{}" "{}""#, entry.name, bat_path_str);
												let dir = bat_path.parent().unwrap_or_else(|| std::path::Path::new("."));
												    let status = std::process::Command::new("cmd")
														.arg("/c")  // 注意路径可能含空格，start会正确处理
														.raw_arg(&full_command)
														.current_dir(dir)
														.status();
													if let Err(e) = status {
														eprintln!("启动失败: {}", e);
													}
											}
										} else {
											let _ = open::that(&path);
										}
                                    }
                                }
                            });
                        });

                        // 第二行：安装路径（可打开）+ 标签右对齐
                        ui.horizontal(|ui| {
                            if let Some(path) = &entry.install_path {
                                ui.label("📁 ");
                                if ui.link(path).clicked() {
                                    let _ = open::that(path);
                                }
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if !entry.tags.is_empty() {
                                    ui.label(format!("🏷️ {}", entry.tags.join(", ")));
                                }
                            });
                        });

                        // 关联文件夹行
                        if !entry.linked_folders.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label("📂");
                                for folder in &entry.linked_folders {
                                    let display = if folder.alias.trim().is_empty() {
                                        &folder.path
                                    } else {
                                        &folder.alias
                                    };
                                    if ui.link(display).clicked() {
                                        let _ = open::that(&folder.path);
                                    }
                                    ui.label("  ");
                                }
                            });
                        }

                        // 第三行：备注
                        if !entry.notes.is_empty() {
                            ui.label(format!("📝 {}", entry.notes));
                        }
                    });
                }
            });
        });

		// 处理批量更新结果
		if let Some((msg, updated)) = ctx.memory(|mem| mem.data.get_temp::<(String, Vec<String>)>("update_result".into())) {
			self.update_messages = updated;
			self.show_update_toast = true;
			self.check_all_in_progress = false;
			self.entries = Self::load_entries(&self.conn);
			self.all_tags = Self::extract_all_tags(&self.entries);
			ctx.memory_mut(|mem| mem.data.remove::<(String, Vec<String>)>("update_result".into()));
		}

		// 更新最后检查时间
		if let Some(time_str) = ctx.memory(|mem| mem.data.get_temp::<String>("last_check_time".into())) {
			if let Ok(dt) = DateTime::parse_from_rfc3339(&time_str) {
				self.last_check_time = Some(dt.with_timezone(&Local));
			}
			ctx.memory_mut(|mem| mem.data.remove::<String>("last_check_time".into()));
		}

		// 处理单个更新结果
		if let Some(msg) = ctx.memory(|mem| mem.data.get_temp::<String>("single_update_result".into())) {
			self.update_messages.push(msg);
			self.show_update_toast = true;
			self.check_single_in_progress.clear();
			self.entries = Self::load_entries(&self.conn);
			ctx.memory_mut(|mem| mem.data.remove::<String>("single_update_result".into()));
		}

		if self.show_update_toast && !self.update_messages.is_empty() {
			egui::Window::new("检查更新结果")
				.collapsible(false)
				.resizable(false)
				.anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
				.show(ctx, |ui| {
					for msg in &self.update_messages {
						ui.label(msg);
					}
					ui.horizontal(|ui| {
						if ui.button("确定").clicked() {
							self.show_update_toast = false;
							self.update_messages.clear();
						}
					});
				});
		}

		// 正在检查弹窗
		if self.check_all_in_progress || !self.check_single_in_progress.is_empty() {
			egui::Window::new("正在检查")
				.collapsible(false)
				.resizable(false)
				.anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
				.show(ctx, |ui| {
					ui.label("正在检查更新，请稍候...");
					ui.spinner();
				});
		}
    }
}
