use eframe::egui;

/// 带清除按钮的搜索框
pub fn search_bar(ui: &mut egui::Ui, text: &mut String) {
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(text).hint_text("搜索..."));
        if !text.is_empty() && ui.button("×").clicked() {
            text.clear();
        }
    });
}

/// 标签选择器（显示所有可用标签，可多选）
pub fn tag_selector(ui: &mut egui::Ui, all_tags: &[String], selected_tags: &mut Vec<String>) {
    for tag in all_tags {
        let mut selected = selected_tags.contains(tag);
        if ui.checkbox(&mut selected, tag).changed() {
            if selected {
                selected_tags.push(tag.clone());
            } else {
                selected_tags.retain(|t| t != tag);
            }
        }
    }
}
