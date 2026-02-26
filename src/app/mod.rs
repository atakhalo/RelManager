// 导出子模块
pub mod model;
pub mod db;
pub mod github;
pub mod updater;
pub mod platform;

// 重新导出常用类型以便外部使用
pub use model::SoftwareEntry;
pub use db::{init_db, insert_software, get_all_software, get_software_by_id, update_software, delete_software};
