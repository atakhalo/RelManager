use anyhow::{Result, Context};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 递归扫描目录下的所有 .exe 文件
pub fn find_exe_files(dir: &str) -> Vec<PathBuf> {
    let mut exes = Vec::new();
    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("exe") {
            exes.push(path.to_path_buf());
        }
    }
    exes
}

/// 获取可执行文件的版本信息（Windows）
pub fn get_file_version(exe_path: &str) -> Option<String> {
    // TODO: 实现版本信息读取，例如使用 winapi 或第三方库
    // 目前返回 None 作为占位
    None
}

/// 尝试从路径中推测可执行文件（通常取最大或最常见的 exe）
pub fn guess_main_exe(dir: &str) -> Option<PathBuf> {
    let exes = find_exe_files(dir);
    // 简单的启发式：取体积最大的（或名称包含特定关键词）
    exes.into_iter().max_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
}
