use eframe::egui;
use crate::app::github::GitHubClient;
use crate::app::platform::{filter_assets_for_windows, score_asset_for_windows};
use crate::app::model::SoftwareEntry;
use crate::utils::path::guess_main_exe;
use chrono::Local;
use rfd::FileDialog;
use std::sync::{Arc, Mutex};

pub struct AddWizard {
    step: usize,
    repo_url: String,
    owner: String,
    repo: String,
    releases: Vec<crate::app::github::Release>,
    selected_release_index: usize,
    selected_asset_index: usize,
    current_download_url: String,
    // 可编辑字段（第三步）
    software_name: String,     // 软件名称（原名）
    alias: String,             // 显示别名
    current_version: String,   // 当前版本
    latest_version: String,    // 最新版本（可手动输入或自动检测后填充）
    asset_name: String,        // 软件包名称
    install_path: String,
    executable_path: String,
    notes: String,
    tags: String,
    // 异步状态
    loading: bool,
    error: Option<String>,
    fetch_result: Arc<Mutex<Option<Result<Vec<crate::app::github::Release>, String>>>>,
    // 关闭标志
    closed: bool,
	show_error_dialog: bool,
    error_message: String,
}

impl AddWizard {
    pub fn new() -> Self {
        Self {
            step: 0,
            repo_url: String::new(),
            owner: String::new(),
            repo: String::new(),
            releases: Vec::new(),
            selected_release_index: 0,
            selected_asset_index: 0,
            current_download_url: String::new(),
            software_name: String::new(),
            alias: String::new(),
            current_version: String::new(),
            latest_version: String::new(),
            asset_name: String::new(),
            install_path: String::new(),
            executable_path: String::new(),
            notes: String::new(),
            tags: String::new(),
            loading: false,
            error: None,
            fetch_result: Arc::new(Mutex::new(None)),
            closed: false,
			show_error_dialog: false,
			error_message: String::new(),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<SoftwareEntry> {
        if self.closed {
            return None;
        }

        let mut result = None;
        egui::Window::new("添加软件")
            .resizable(true)
            .movable(true)
            .min_width(450.0)
            .min_height(350.0)
            .max_width(f32::INFINITY)
            .max_height(f32::INFINITY)
            .default_width(500.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                // 顶部标题（当前步骤）
                ui.horizontal(|ui| {
                    ui.heading(match self.step {
                        0 => "步骤1: 输入 GitHub 链接",
                        1 => "步骤2: 选择版本与安装包",
                        2 => "步骤3: 填写本地信息",
                        _ => "",
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("❌ 关闭").clicked() {
                            self.closed = true;
                        }
                    });
                });
                ui.separator();

                // 检查异步任务结果
                if let Ok(mut guard) = self.fetch_result.try_lock() {
                    if let Some(fetch_result) = guard.take() {
                        self.loading = false;
                        match fetch_result {
                            Ok(releases) => {
                                if releases.is_empty() {
                                    self.error = Some("该仓库没有发布版本".to_string());
                                } else {
                                    self.releases = releases;
                                    self.step = 1;
                                    self.error = None;
                                }
                            }
                            Err(e) => {
                                self.error = Some(format!("获取失败: {}", e));
                            }
                        }
                    }
                }

                if let Some(err) = &self.error {
                    ui.colored_label(egui::Color32::RED, err);
                }

                match self.step {
                    0 => self.step0_ui(ui, ctx),
                    1 => self.step1_ui(ui),
                    2 => self.step2_ui(ui, &mut result),
                    _ => {}
                }
            });

		// 错误弹窗（独立于向导窗口）
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


        result
    }

    // 步骤0：输入链接
    fn step0_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label("GitHub 仓库链接 (例如: https://github.com/owner/repo):");
        ui.text_edit_singleline(&mut self.repo_url);

        ui.horizontal(|ui| {
            let loading = self.loading;
            let btn = ui.add_enabled(!loading, egui::Button::new("获取 Releases"));
            if btn.clicked() && !self.repo_url.is_empty() {
                // 去除首尾空格和换行
                let trimmed = self.repo_url.trim().to_string();
                self.repo_url = trimmed;

                if let Some((owner, repo)) = GitHubClient::parse_repo_url(&self.repo_url) {
                    self.owner = owner.clone();
                    self.repo = repo.clone();
                    self.loading = true;
                    self.error = None;

                    let fetch_result = self.fetch_result.clone();
                    let ctx = ctx.clone();

                    tokio::spawn(async move {
                        let client = GitHubClient::new(None); // TODO: 从设置读取 token
                        let result = client.fetch_releases(&owner, &repo).await
                            .map_err(|e| e.to_string());

                        *fetch_result.lock().unwrap() = Some(result);
                        ctx.request_repaint();
                    });
                } else {
                    self.error = Some("无效的 GitHub 链接".to_string());
                }
            }

            if loading {
                ui.spinner();
            }

        });
		ui.separator();

		// 直接编辑按钮
		if ui.button("非github软件，直接编辑信息").clicked() {
			// 清空 releases 以跳过步骤1
			self.releases.clear();
			self.step = 2;
		}
    }

    // 步骤1：选择版本和资产
    fn step1_ui(&mut self, ui: &mut egui::Ui) {
        if self.releases.is_empty() {
            ui.label("没有找到 releases，请返回上一步检查链接。");
            if ui.button("上一步").clicked() {
                self.step = 0;
            }
            return;
        }
		
		// 最新版本为 releases 列表的第一个
		self.latest_version = self.releases[0].tag_name.clone();

        // 确保选中的版本有效
        if self.selected_release_index >= self.releases.len() {
            self.selected_release_index = 0;
        }
        let release = &self.releases[self.selected_release_index];

        ui.label("选择版本:");
        egui::ComboBox::from_label("版本")
            .selected_text(&release.tag_name)
            .show_ui(ui, |ui| {
                for (i, rel) in self.releases.iter().enumerate() {
                    if ui.selectable_value(&mut self.selected_release_index, i, &rel.tag_name).changed() {
                        // 版本改变时重置资产索引
                        self.selected_asset_index = 0;
                    }
                }
            });

        ui.label("选择安装包 (⭐ 为推荐匹配 Windows 的资产):");

        let scored_indices = filter_assets_for_windows(&release.assets);

        if scored_indices.is_empty() {
            ui.colored_label(egui::Color32::YELLOW, "未找到匹配 Windows 的资产，请手动检查。");
        }

        // 确定默认选中的资产索引：优先推荐列表的第一个
        if !release.assets.is_empty() {
            if !scored_indices.is_empty() {
                let is_current_recommended = scored_indices.iter().any(|(idx, _)| *idx == self.selected_asset_index);
                if !is_current_recommended {
                    self.selected_asset_index = scored_indices[0].0;
                }
            } else {
                if self.selected_asset_index >= release.assets.len() {
                    self.selected_asset_index = 0;
                }
            }
            self.current_download_url = release.assets[self.selected_asset_index].browser_download_url.clone();
            // 更新当前版本和资产名（用于步骤2预填充）
            self.current_version = release.tag_name.clone();
            self.asset_name = release.assets[self.selected_asset_index].name.clone();
        } else {
            self.selected_asset_index = 0;
            self.current_download_url.clear();
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for (original_index, _) in &scored_indices {
                let asset = &release.assets[*original_index];
                let lower_name = asset.name.to_lowercase();
                let is_x64 = lower_name.contains("x86_64") || lower_name.contains("amd64") || lower_name.contains("x64");
                let text = if is_x64 {
                    format!("{} ⭐", asset.name)
                } else {
                    asset.name.clone()
                };
                if ui.radio(self.selected_asset_index == *original_index, text).clicked() {
                    self.selected_asset_index = *original_index;
                    self.current_download_url = asset.browser_download_url.clone();
                    self.asset_name = asset.name.clone(); // 同步更新
                }
            }
        });

        // 显示下载链接和操作
        if !scored_indices.is_empty() && self.selected_asset_index < release.assets.len() {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label("下载链接:");
                ui.vertical(|ui| {
                    let mut url_display = self.current_download_url.clone();
                    ui.add_sized([ui.available_width() - 100.0, 60.0],
                        egui::TextEdit::multiline(&mut url_display)
                            .desired_rows(3)
                            .interactive(false)
                    );
                    ui.horizontal(|ui| {
                        if ui.button("📋 复制").clicked() {
                            ui.ctx().copy_text(self.current_download_url.clone());
                        }
                        if ui.button("🌐 打开").clicked() {
                            let url = self.current_download_url.clone();
                            std::thread::spawn(move || {
                                let _ = open::that(url);
                            });
                        }
                    });
                });
            });
            ui.label("提示：下载安装完成后，点击下一步填写本地信息。");
        }

        ui.horizontal(|ui| {
            if ui.button("上一步").clicked() {
                self.step = 0;
            }
            if ui.button("下一步").clicked() {
                // 预填充软件名称（如果未设置）
                if self.software_name.is_empty() {
                    self.software_name = self.repo.clone();
                }
                self.step = 2;
            }
        });
    }

    // 步骤2：填写本地信息（所有字段可编辑）
    fn step2_ui(&mut self, ui: &mut egui::Ui, result: &mut Option<SoftwareEntry>) {
        ui.label("软件名称（必填）:");
        ui.text_edit_singleline(&mut self.software_name);

        ui.label("别名:");
        ui.text_edit_singleline(&mut self.alias);

		ui.label("GitHub 仓库:");
		ui.text_edit_singleline(&mut self.repo_url);

        ui.horizontal(|ui| {
            ui.label("当前版本:");
            ui.text_edit_singleline(&mut self.current_version);
        });

        ui.horizontal(|ui| {
            ui.label("最新版本:");
            ui.text_edit_singleline(&mut self.latest_version);
        });

        ui.horizontal(|ui| {
            ui.label("软件包:");
            ui.text_edit_singleline(&mut self.asset_name);
        });

        ui.label("根路径:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.install_path);
            if ui.button("浏览...").clicked() {
                if let Some(path) = FileDialog::new().pick_folder() {
                    self.install_path = path.display().to_string();
                    if let Some(exe) = guess_main_exe(&self.install_path) {
                        self.executable_path = exe.display().to_string();
                    }
                }
            }
        });


        ui.horizontal(|ui| {
			ui.label("目标文件/文件夹: ");
            if ui.button("文件").clicked() {
                if let Some(path) = FileDialog::new()
                    // .add_filter("exe", &["exe"])
                    .set_directory(&self.install_path)
                    .pick_file()
                {
                    self.executable_path = path.display().to_string();
                }
            }
			if ui.button("文件夹").clicked() {
                if let Some(path) = FileDialog::new()
                    .set_directory(&self.install_path)
                    .pick_folder()
                {
                    self.executable_path = path.display().to_string();
                }
            }
            if ui.button("自动检测exe").clicked() && !self.install_path.is_empty() {
                if let Some(exe) = guess_main_exe(&self.install_path) {
                    self.executable_path = exe.display().to_string();
                }
            }
        });
		ui.text_edit_singleline(&mut self.executable_path);


        ui.label("备注:");
        ui.text_edit_multiline(&mut self.notes);

        ui.label("标签 (逗号分隔):");
        ui.text_edit_singleline(&mut self.tags);

        ui.horizontal(|ui| {
            if ui.button("上一步").clicked() {
                self.step = 1;
            }
            if ui.button("完成").clicked() {
				if self.software_name.trim().is_empty() {
					self.error_message = "软件名称不能为空".to_string();
					self.show_error_dialog = true;
					return; // 直接返回，不继续构建条目
				}

                // 解析标签
                let tags: Vec<String> = self.tags
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                let entry = SoftwareEntry {
                    id: None,
                    name: self.software_name.clone(),
                    alias:  self.alias.clone(),
					repo_url: self.repo_url.trim().to_string(),
                    current_version: self.current_version.clone(),
                    latest_version: if self.latest_version.is_empty() { None } else { Some(self.latest_version.clone()) },
                    asset_name: self.asset_name.clone(),
                    install_path: if self.install_path.is_empty() { None } else { Some(self.install_path.clone()) },
                    executable_path: if self.executable_path.is_empty() { None } else { Some(self.executable_path.clone()) },
                    notes: self.notes.clone(),
                    tags,
                    created_at: Local::now(),
                    updated_at: Local::now(),
                };
                *result = Some(entry);
            }
        });
    }
}
