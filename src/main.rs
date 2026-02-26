// 声明模块
mod app;
mod gui;
mod utils;

use eframe::egui;
use crate::gui::main_window::MainWindow;
use crate::app::db;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();
    
	    // 创建 Tokio 运行时（多线程）
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let _guard = runtime.enter(); // 进入运行时上下文，使 tokio::spawn 可工作

    // 初始化数据库
    let conn = db::init_db().expect("Failed to initialize database");
    
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 700.0])
            .with_min_inner_size([600.0, 400.0])
            .with_title("GitHub Release Manager"),
        ..Default::default()
    };
    
    eframe::run_native(
        "GitHub 发布管理",
        options,
        Box::new(|cc| {
            // 设置中文字体（通常默认已支持，但显式配置可确保兼容性）
            setup_fonts(&cc.egui_ctx);
            Box::new(MainWindow::new(conn))
        }),
    )
}

fn setup_fonts(ctx: &egui::Context) {
    // 获取默认字体定义
    let mut fonts = egui::FontDefinitions::default();
    
    // 添加中文字体支持
    fonts.font_data.insert(
        "simsun".to_owned(),
        egui::FontData::from_static(include_bytes!("C:/Windows/Fonts/simsun.ttc")),
    );
    
    fonts.font_data.insert(
        "msyh".to_owned(),
        egui::FontData::from_static(include_bytes!("C:/Windows/Fonts/msyh.ttc")),
    );
    
    // 设置字体族
    if let Some(families) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        families.insert(0, "msyh".to_owned());
        families.insert(1, "simsun".to_owned());
    }
    
    if let Some(families) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
        families.insert(0, "msyh".to_owned());
        families.push("simsun".to_owned());
    }
    
    ctx.set_fonts(fonts);
}
