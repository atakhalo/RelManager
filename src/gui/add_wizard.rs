use eframe::egui;
use crate::app::github::GitHubClient;
use crate::app::platform::{filter_assets_for_windows, score_asset_for_windows};
use crate::app::model::SoftwareEntry;
use crate::utils::path::{find_exe_files, guess_main_exe};
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
    // 填写信息
    name: String,
    install_path: String,
    executable_path: String,
    notes: String,
    tags: String,
    // 异步状态
    loading: bool,
    error: Option<String>,
    fetch_result: Arc<Mutex<Option<Result<Vec<crate::app::github::Release>, String>>>>,
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
            name: String::new(),
            install_path: String::new(),
            executable_path: String::new(),
            notes: String::new(),
            tags: String::new(),
            loading: false,
            error: None,
            fetch_result: Arc::new(Mutex::new(None)),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<SoftwareEntry> {
        let mut result = None;
        egui::Window::new("添加软件")
			.resizable(true)          // 允许调整大小
			.movable(true)             // 允许移动
			.min_width(400.0)          // 最小宽度
			.min_height(300.0)         // 最小高度
			.max_width(f32::INFINITY)  // 无最大宽度
			.max_height(f32::INFINITY) // 无最大高度
			.default_width(500.0)      // 默认宽度
			.default_height(400.0)     // 默认高度
            .show(ctx, |ui| {
                ui.heading(match self.step {
                    0 => "步骤1: 输入GitHub链接",
                    1 => "步骤2: 选择版本与安装包",
                    2 => "步骤3: 填写本地信息",
                    _ => "",
                });

                // 检查异步任务是否完成
                if let Ok(mut guard) = self.fetch_result.try_lock() {
                    if let Some(fetch_result) = guard.take() {
                        self.loading = false;
                        match fetch_result {
                            Ok(releases) => {
                                if releases.is_empty() {
                                    self.error = Some("该仓库没有 releases".to_string());
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
        result
    }

	fn step0_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
		ui.label("GitHub 仓库链接 (例如: https://github.com/owner/repo):");
		ui.text_edit_singleline(&mut self.repo_url);

		ui.horizontal(|ui| {
			let loading = self.loading;
			let btn = ui.add_enabled(!loading, egui::Button::new("获取 Releases"));
			if btn.clicked() && !self.repo_url.is_empty() {
				self.loading = true;
				self.error = None;
				let url = self.repo_url.clone();
				let fetch_result = self.fetch_result.clone();
				let ctx = ctx.clone();

				tokio::spawn(async move {
					let client = GitHubClient::new(None); // TODO: 从设置读取 token
					let result = if let Some((owner, repo)) = GitHubClient::parse_repo_url(&url) {
						client.fetch_releases(&owner, &repo).await
							.map_err(|e| e.to_string())
					} else {
						Err("无效的 GitHub 链接".to_string())
					};
					
					*fetch_result.lock().unwrap() = Some(result);
					ctx.request_repaint(); // 通知 UI 更新
				});
			}

			if loading {
				ui.spinner();
			}
		});
	}

    fn step1_ui(&mut self, ui: &mut egui::Ui) {
        if self.releases.is_empty() {
            ui.label("没有找到 releases，请返回上一步检查链接。");
            if ui.button("上一步").clicked() {
                self.step = 0;
            }
            return;
        }

		// 确保选中的版本有效
		if self.selected_release_index >= self.releases.len() {
			self.selected_release_index = 0;
		}
		let release = &self.releases[self.selected_release_index];

        // 选择版本
        ui.label("选择版本:");
        egui::ComboBox::from_label("版本")
            .selected_text(&self.releases[self.selected_release_index].tag_name)
            .show_ui(ui, |ui| {
                for (i, rel) in self.releases.iter().enumerate() {
					if ui.selectable_value(&mut self.selected_release_index, i, &rel.tag_name).clicked() {
						// 版本改变时重置资产索引
						self.selected_asset_index = 0;
						// 更新下载链接（将在后面统一处理）
					}
                }
            });

        // 选择资产
        ui.label("选择安装包 (⭐ 为推荐匹配 Windows 的资产):");
let scored_indices = crate::app::platform::filter_assets_for_windows(&release.assets);

    if scored_indices.is_empty() {
        ui.colored_label(egui::Color32::YELLOW, "未找到匹配 Windows 的资产，请手动检查。");
    }

    // 确保资产索引有效，并更新当前下载链接
    if release.assets.is_empty() {
        self.selected_asset_index = 0;
        self.current_download_url.clear();
    } else {
        if self.selected_asset_index >= release.assets.len() {
            self.selected_asset_index = 0;
        }
        self.current_download_url = release.assets[self.selected_asset_index].browser_download_url.clone();
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
            }
        }
    });

    // 显示当前选中资产的下载链接
    if !scored_indices.is_empty() && self.selected_asset_index < release.assets.len() {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label("下载链接:");
            // 使用只读文本框显示链接
            ui.add(egui::TextEdit::singleline(&mut self.current_download_url)
                .desired_width(300.0)
                .interactive(false));
            if ui.button("📋 复制").clicked() {
                ui.ctx().copy_text(self.current_download_url.clone());
            }
			if ui.button("🌐 打开").clicked() {
            let url = self.current_download_url.clone();
            let _ = std::thread::spawn(move || {
                open::that(url).ok();
            });
        }
        });
        ui.label("提示：下载安装完成后，点击下一步填写本地信息。");
    }

        ui.horizontal(|ui| {
            if ui.button("上一步").clicked() {
                self.step = 0;
            }
            if ui.button("下一步").clicked() {
                // 预填充名称
                if self.name.is_empty() {
                    self.name = self.repo.clone();
                }
                self.step = 2;
            }
        });
    }

    fn step2_ui(&mut self, ui: &mut egui::Ui, result: &mut Option<SoftwareEntry>) {
        ui.label("软件名称:");
        ui.text_edit_singleline(&mut self.name);

        ui.label("安装路径:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.install_path);
            if ui.button("浏览...").clicked() {
                if let Some(path) = FileDialog::new().pick_folder() {
                    self.install_path = path.display().to_string();
                    // 自动猜测可执行文件
                    if let Some(exe) = crate::utils::path::guess_main_exe(&self.install_path) {
                        self.executable_path = exe.display().to_string();
                    }
                }
            }
        });

        ui.label("可执行文件:");
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.executable_path);
            if ui.button("浏览...").clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter("exe", &["exe"])
                    .set_directory(&self.install_path)
                    .pick_file()
                {
                    self.executable_path = path.display().to_string();
                }
            }
            if ui.button("自动检测").clicked() && !self.install_path.is_empty() {
                if let Some(exe) = crate::utils::path::guess_main_exe(&self.install_path) {
                    self.executable_path = exe.display().to_string();
                }
            }
        });

        ui.label("备注:");
        ui.text_edit_multiline(&mut self.notes);

        ui.label("标签 (逗号分隔):");
        ui.text_edit_singleline(&mut self.tags);

        ui.horizontal(|ui| {
            if ui.button("上一步").clicked() {
                self.step = 1;
            }
            if ui.button("完成").clicked() {
                // 构建条目
                let tags: Vec<String> = self.tags
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                
                let asset = &self.releases[self.selected_release_index]
                    .assets[self.selected_asset_index];
                
                let entry = SoftwareEntry {
                    id: None,
                    name: self.name.clone(),
                    repo_owner: self.owner.clone(),
                    repo_name: self.repo.clone(),
                    current_version: self.releases[self.selected_release_index].tag_name.clone(),
                    latest_version: None,
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
