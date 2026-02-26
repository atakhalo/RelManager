/// 计算资产与 Windows 平台的匹配度（分数越高越匹配）
pub fn score_asset_for_windows(name: &str) -> u32 {
    let lower = name.to_lowercase();
    let mut score = 0;

    // 排除明显非 Windows 的关键词（一旦出现直接得0分）
    let exclude_keywords = ["linux", "darwin", "macos", "osx", "rpm", "deb", "appimage", "alpine"];
    for kw in &exclude_keywords {
        if lower.contains(kw) {
            return 0;
        }
    }

    // Windows 特定关键词加分（高权重）
    if lower.contains("windows") { score += 40; }
    if lower.contains("win64") { score += 35; }
    if lower.contains("win32") { score += 30; }
    if lower.contains("win-") || lower.contains("_win") { score += 35; } // 匹配 win-arm64, win-x64

    // 架构关键词加分（仅当至少有一个 Windows 关键词时有效，但这里我们允许单独加分）
    if lower.contains("x86_64") || lower.contains("amd64") || lower.contains("x64") { 
        score += 15;
    }
    if lower.contains("x86") || lower.contains("i386") { 
        score += 10; 
    }
    if lower.contains("arm64") || lower.contains("aarch64") { 
        // Windows on ARM 也存在，但不加分太多，防止干扰
        score += 5;
    }

    // 文件类型加分（权重高，确保可执行文件优先）
    if lower.ends_with(".exe") { score += 30; }
    else if lower.ends_with(".msi") { score += 25; }
    else if lower.ends_with(".zip") { score += 15; }  // zip 仍可能跨平台，但配合 win 关键词后分数会高
    else if lower.ends_with(".7z") { score += 10; }
    else if lower.ends_with(".tar") { score += 5; }   // 可能是源码包，得分低

    // 如果文件名包含 "win" 但尚未得分，给予基础分（如 "win-arm64.zip" 中的 "win"）
    if lower.contains("win") && score == 0 {
        score += 5;
    }

    score
}

/// 判断是否为 Windows 资产（简化版）
pub fn is_windows_asset(name: &str) -> bool {
    score_asset_for_windows(name) > 20
}

/// 根据平台筛选资产，返回按分数排序的列表
pub fn filter_assets_for_windows(assets: &[crate::app::github::Asset]) -> Vec<(usize, u32)> {
    let mut scored: Vec<(usize, u32)> = assets
        .iter()
        .enumerate()
        .map(|(i, a)| (i, score_asset_for_windows(&a.name)))
        .filter(|(_, score)| *score > 0)
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1)); // 降序
    scored
}
