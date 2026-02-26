use semver::Version;

/// 去除版本字符串常见前缀（如 'v'）
pub fn normalize_version(ver: &str) -> &str {
    ver.trim_start_matches('v')
}

/// 比较两个版本，若 a 比 b 新则返回 true
pub fn is_newer(a: &str, b: &str) -> bool {
    compare_versions(a, b) == std::cmp::Ordering::Greater
}

/// 版本比较（优先语义化，否则字符串）
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_norm = normalize_version(a);
    let b_norm = normalize_version(b);
    
    if let (Ok(va), Ok(vb)) = (Version::parse(a_norm), Version::parse(b_norm)) {
        va.cmp(&vb)
    } else {
        a_norm.cmp(b_norm)
    }
}
