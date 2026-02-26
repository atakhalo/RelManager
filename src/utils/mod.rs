//! 工具模块

pub mod path;
pub mod version;

// 重新导出常用函数
pub use path::{find_exe_files, get_file_version, guess_main_exe};
pub use version::{compare_versions, is_newer, normalize_version};
