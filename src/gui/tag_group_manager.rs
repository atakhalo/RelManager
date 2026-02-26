use eframe::egui;
use crate::app::model::TagGroup;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct TagGroupManager {
    groups: Vec<TagGroup>,
    conn: Arc<Mutex<Connection>>,
    open: bool,
    // 编辑状态
    editing_group: Option<TagGroup>,
    is_creating: bool,  // 是否处于新建模式
    edit_name: String,
    edit_tags: String,
    show_error: bool,
    error_message: String,
    pending_delete_id: Option<i64>,
}

impl TagGroupManager {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        let groups = Self::load_groups(&conn);
        Self {
            groups,
            conn,
            open: true,
            editing_group: None,
            is_creating: false,
            edit_name: String::new(),
            edit_tags: String::new(),
            show_error: false,
            error_message: String::new(),
            pending_delete_id: None,
        }
    }

    fn load_groups(conn: &Arc<Mutex<Connection>>) -> Vec<TagGroup> {
        if let Ok(conn) = conn.lock() {
            crate::app::db::get_all_tag_groups(&conn).unwrap_or_default()
        } else {
            vec![]
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) -> Option<Vec<TagGroup>> {
        let mut result = None;
        if self.open {
            egui::Window::new("标签组管理")
                .default_width(400.0)
                .default_height(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    // 错误弹窗
                    if self.show_error {
                        egui::Window::new("错误")
                            .collapsible(false)
                            .resizable(false)
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .show(ctx, |ui| {
                                ui.label(&self.error_message);
                                ui.horizontal(|ui| {
                                    if ui.button("确定").clicked() {
                                        self.show_error = false;
                                    }
                                });
                            });
                    }

                    // 延迟删除
                    if let Some(id) = self.pending_delete_id.take() {
                        if let Ok(conn) = self.conn.lock() {
                            let _ = crate::app::db::delete_tag_group(&conn, id);
                        }
                        self.groups = Self::load_groups(&self.conn);
                        ctx.request_repaint();
                    }

                    // 标题和新建按钮
                    ui.horizontal(|ui| {
                        ui.heading("标签组列表");
                        if ui.button("➕ 新建组").clicked() {
                            self.editing_group = None;
                            self.is_creating = true;      // 进入新建模式
                            self.edit_name.clear();
                            self.edit_tags.clear();
                        }
                    });
                    ui.separator();

                    // 列表
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let groups = self.groups.clone();
                        for group in groups {
                            ui.horizontal(|ui| {
                                ui.label(format!("{} ({} 标签)", group.name, group.tags.len()));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("✏️").clicked() {
                                        self.editing_group = Some(group.clone());
                                        self.is_creating = false;
                                        self.edit_name = group.name.clone();
                                        self.edit_tags = group.tags.join(", ");
                                    }
                                    if ui.button("🗑️").clicked() {
                                        self.pending_delete_id = group.id;
                                    }
                                });
                            });
                        }
                    });

                    ui.separator();

                    // 编辑/新建区域（显示条件：编辑或新建模式）
                    if self.editing_group.is_some() || self.is_creating {
                        ui.heading(if self.editing_group.is_some() { "编辑组" } else { "新建组" });
                        ui.horizontal(|ui| {
                            ui.label("组名:");
                            ui.text_edit_singleline(&mut self.edit_name);
                        });
                        ui.horizontal(|ui| {
                            ui.label("标签 (逗号分隔):");
                            ui.text_edit_singleline(&mut self.edit_tags);
                        });
                        ui.horizontal(|ui| {
                            if ui.button("保存").clicked() {
                                if self.edit_name.trim().is_empty() {
                                    self.error_message = "组名不能为空".to_string();
                                    self.show_error = true;
                                } else {
                                    let tags: Vec<String> = self.edit_tags
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect();
                                    let group = TagGroup {
                                        id: self.editing_group.as_ref().and_then(|g| g.id),
                                        name: self.edit_name.trim().to_string(),
                                        tags,
                                    };
                                    if let Ok(conn) = self.conn.lock() {
                                        let _ = crate::app::db::save_tag_group(&conn, &group);
                                    }
                                    self.groups = Self::load_groups(&self.conn);
                                    self.editing_group = None;
                                    self.is_creating = false;
                                    self.edit_name.clear();
                                    self.edit_tags.clear();
                                    ctx.request_repaint();
                                }
                            }
                            if ui.button("取消").clicked() {
                                self.editing_group = None;
                                self.is_creating = false;
                                self.edit_name.clear();
                                self.edit_tags.clear();
                            }
                        });
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("关闭").clicked() {
                                self.open = false;
                            }
                        });
                    });
                });
        }
        if !self.open {
            result = Some(self.groups.clone());
        }
        result
    }
}
