//! 验证框架的属性测试
//!
//! **Feature: bug-fixes, Property 22: Path Traversal Protection**
//! **Feature: bug-fixes, Property 23: Workspace ID Safety**
//! **Validates: Requirements 6.1, 6.2**

#[cfg(test)]
mod property_tests {
    use crate::utils::validation::{
        prevent_path_traversal, sanitize_file_name_strict, validate_workspace_id,
    };
    use proptest::prelude::*;

    // ============================================================================
    // Property 22: Path Traversal Protection
    // ============================================================================

    /// **Feature: bug-fixes, Property 22: Path Traversal Protection**
    /// **Validates: Requirements 6.1**
    ///
    /// *For any* path parameter input, the system should validate against path traversal attacks
    ///
    /// 这个属性确保所有包含路径遍历模式的输入都被正确拒绝
    #[test]
    fn property_22_path_traversal_protection() {
        proptest!(ProptestConfig::with_cases(100), |(
            // 生成包含路径遍历模式的路径
            prefix in "[a-zA-Z0-9_-]{0,10}",
            traversal_pattern in prop_oneof![
                Just(".."),
                Just("/../"),
                Just("\\..\\"),
                Just("%2e%2e"),
                Just("%252e%252e"),
                Just("..%2f"),
                Just("..%5c"),
            ],
            suffix in "[a-zA-Z0-9_/-]{0,20}"
        )| {
            let dangerous_path = format!("{}{}{}", prefix, traversal_pattern, suffix);

            // 验证：包含路径遍历模式的路径应该被拒绝
            let result = prevent_path_traversal(&dangerous_path);
            prop_assert!(
                result.is_err(),
                "Path with traversal pattern should be rejected: {}",
                dangerous_path
            );
        });
    }

    /// **Feature: bug-fixes, Property 22: Path Traversal Protection (Safe Paths)**
    /// **Validates: Requirements 6.1**
    ///
    /// *For any* safe path (without traversal patterns), the system should accept it
    #[test]
    fn property_22_safe_paths_accepted() {
        proptest!(ProptestConfig::with_cases(100), |(
            // 生成安全的路径组件
            components in prop::collection::vec("[a-zA-Z0-9_-]{1,20}", 1..5)
        )| {
            let safe_path = components.join("/");

            // 验证：安全路径应该被接受
            let result = prevent_path_traversal(&safe_path);
            prop_assert!(
                result.is_ok(),
                "Safe path should be accepted: {}",
                safe_path
            );

            if let Ok(normalized) = result {
                // 规范化后的路径不应包含危险模式
                prop_assert!(!normalized.contains(".."));
            }
        });
    }

    /// **Feature: bug-fixes, Property 22: Null Byte Injection Protection**
    /// **Validates: Requirements 6.1**
    ///
    /// *For any* path containing null bytes, the system should reject it
    #[test]
    fn property_22_null_byte_protection() {
        proptest!(ProptestConfig::with_cases(100), |(
            prefix in "[a-zA-Z0-9_/-]{0,20}",
            suffix in "[a-zA-Z0-9_/-]{0,20}"
        )| {
            let path_with_null = format!("{}\0{}", prefix, suffix);

            // 验证：包含 null 字节的路径应该被拒绝
            let result = prevent_path_traversal(&path_with_null);
            prop_assert!(
                result.is_err(),
                "Path with null byte should be rejected"
            );
        });
    }

    // ============================================================================
    // Property 23: Workspace ID Safety
    // ============================================================================

    /// **Feature: bug-fixes, Property 23: Workspace ID Safety**
    /// **Validates: Requirements 6.2**
    ///
    /// *For any* workspace ID submission, only safe characters should be accepted
    ///
    /// 这个属性确保工作区 ID 只包含字母数字、连字符和下划线
    #[test]
    fn property_23_workspace_id_safety() {
        proptest!(ProptestConfig::with_cases(100), |(
            // 生成只包含安全字符的 ID
            id in "[a-zA-Z0-9_-]{1,50}"
        )| {
            // 验证：只包含安全字符的 ID 应该被接受
            let result = validate_workspace_id(&id);
            prop_assert!(
                result.is_ok(),
                "Valid workspace ID should be accepted: {}",
                id
            );
        });
    }

    /// **Feature: bug-fixes, Property 23: Workspace ID Rejection**
    /// **Validates: Requirements 6.2**
    ///
    /// *For any* workspace ID containing unsafe characters, it should be rejected
    #[test]
    fn property_23_unsafe_workspace_id_rejected() {
        proptest!(ProptestConfig::with_cases(100), |(
            // 生成包含不安全字符的 ID
            prefix in "[a-zA-Z0-9_-]{0,10}",
            unsafe_char in "[^a-zA-Z0-9_-]",
            suffix in "[a-zA-Z0-9_-]{0,10}"
        )| {
            let unsafe_id = format!("{}{}{}", prefix, unsafe_char, suffix);

            // 跳过空 ID（已经被长度检查覆盖）
            if unsafe_id.trim().is_empty() {
                return Ok(());
            }

            // 验证：包含不安全字符的 ID 应该被拒绝
            let result = validate_workspace_id(&unsafe_id);
            prop_assert!(
                result.is_err(),
                "Workspace ID with unsafe characters should be rejected: {}",
                unsafe_id
            );
        });
    }

    /// **Feature: bug-fixes, Property 23: Workspace ID Length Limits**
    /// **Validates: Requirements 6.2**
    ///
    /// *For any* workspace ID exceeding length limits, it should be rejected
    #[test]
    fn property_23_workspace_id_length_limits() {
        proptest!(ProptestConfig::with_cases(100), |(
            // 生成超长的 ID
            id in "[a-zA-Z0-9_-]{51,100}"
        )| {
            // 验证：超长 ID 应该被拒绝
            let result = validate_workspace_id(&id);
            prop_assert!(
                result.is_err(),
                "Workspace ID exceeding length limit should be rejected"
            );
        });
    }

    // ============================================================================
    // Additional Properties: Filename Sanitization
    // ============================================================================

    /// **Feature: bug-fixes, Property: Filename Sanitization Safety**
    /// **Validates: Requirements 6.1, 6.4**
    ///
    /// *For any* filename, sanitization should produce a safe result
    #[test]
    fn property_filename_sanitization_safety() {
        proptest!(ProptestConfig::with_cases(100), |(
            // 生成各种文件名
            name in "[a-zA-Z0-9_.-]{1,50}"
        )| {
            // 验证：清理后的文件名应该是安全的
            let result = sanitize_file_name_strict(&name);

            if let Ok(sanitized) = result {
                // 清理后的文件名不应包含危险字符
                prop_assert!(!sanitized.contains('/'));
                prop_assert!(!sanitized.contains('\\'));
                prop_assert!(!sanitized.contains('\0'));
                prop_assert!(!sanitized.is_empty());
                prop_assert!(sanitized.len() <= 255);
            }
        });
    }

    /// **Feature: bug-fixes, Property: Reserved Filename Rejection**
    /// **Validates: Requirements 6.1**
    ///
    /// *For any* Windows reserved filename, it should be rejected
    #[test]
    fn property_reserved_filename_rejection() {
        proptest!(ProptestConfig::with_cases(100), |(
            reserved in prop_oneof![
                Just("CON"),
                Just("PRN"),
                Just("AUX"),
                Just("NUL"),
                Just("COM1"),
                Just("LPT1"),
            ],
            extension in prop::option::of("[a-z]{3}")
        )| {
            let filename = if let Some(ext) = extension {
                format!("{}.{}", reserved, ext)
            } else {
                reserved.to_string()
            };

            // 验证：保留文件名应该被拒绝
            let result = sanitize_file_name_strict(&filename);
            prop_assert!(
                result.is_err(),
                "Reserved filename should be rejected: {}",
                filename
            );
        });
    }
}
