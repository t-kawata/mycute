// Darvium Applicability Gate (ハードゲート)
//
// RFC §11.1 で規定された AG-01〜AG-07 のうち、AG-06 と AG-07 を実装する。
// 本モジュールは State を持たない純粋比較関数として提供される。

use crate::error::DarviumError;

/// 埋め込みチャネルバージョン (RFC §11)。
///
/// 埋め込みベクトルの生成に使用されたモデルとテンプレートのバージョンを保持する。
/// AG-06 / AG-07 のハードゲート判定でクエリと候補の突合に使用される。
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingChannelVersion {
    /// 埋め込みモデルのバージョン識別子 (例: "v2.0-final")
    pub model_version: String,
    /// テンプレートのバージョン識別子 (structural channel のみ使用、semantic channel では None)
    pub template_version: Option<String>,
}

impl EmbeddingChannelVersion {
    /// デフォルト値で初期化した EmbeddingChannelVersion を生成する。
    pub fn new(model_version: String, template_version: Option<String>) -> Self {
        Self {
            model_version,
            template_version,
        }
    }

    /// デフォルトバージョンで初期化する (semantic channel 用)。
    pub fn default_task() -> Self {
        Self {
            model_version: crate::constants::AG_HARD_GATE_DEFAULT_MODEL_VERSION.to_string(),
            template_version: None,
        }
    }

    /// デフォルトバージョンで初期化する (structural channel 用)。
    pub fn default_design() -> Self {
        Self {
            model_version: crate::constants::AG_HARD_GATE_DEFAULT_MODEL_VERSION.to_string(),
            template_version: Some(
                crate::constants::AG_HARD_GATE_DEFAULT_TEMPLATE_VERSION.to_string(),
            ),
        }
    }
}

impl Default for EmbeddingChannelVersion {
    fn default() -> Self {
        Self::default_task()
    }
}

/// 埋め込みバージョン群 (RFC §11)。
///
/// semantic channel と structural proxy channel の両方のバージョンを保持する。
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddingVersions {
    /// semantic channel (task_embedding) のバージョン
    pub task: EmbeddingChannelVersion,
    /// structural proxy channel (workflow_design_embedding) のバージョン
    pub design: EmbeddingChannelVersion,
}

impl EmbeddingVersions {
    /// デフォルト値で初期化した EmbeddingVersions を生成する。
    pub fn new(task: EmbeddingChannelVersion, design: EmbeddingChannelVersion) -> Self {
        Self { task, design }
    }
}

impl Default for EmbeddingVersions {
    fn default() -> Self {
        Self {
            task: EmbeddingChannelVersion::default_task(),
            design: EmbeddingChannelVersion::default_design(),
        }
    }
}

/// AG-06: semantic channel (task_embedding) のハードゲート。
///
/// クエリと候補の model_version が完全一致する場合のみ通過を許可する。
/// 不一致の場合は `ApplicabilityRejected` エラーを返す。
/// ピクセル単位の完全一致比較を行い、大文字小文字・部分一致も不一致と判定する。
pub fn check_ag06(
    query_version: &EmbeddingChannelVersion,
    candidate_version: &EmbeddingChannelVersion,
) -> Result<(), DarviumError> {
    if query_version.model_version != candidate_version.model_version {
        return Err(DarviumError::ApplicabilityRejected {
            gate: "AG-06".to_string(),
            reason: format!(
                "Semantic channel model version mismatch: query='{}', candidate='{}'",
                query_version.model_version, candidate_version.model_version
            ),
        });
    }
    Ok(())
}

/// AG-07: structural proxy channel (workflow_design_embedding) のハードゲート。
///
/// クエリと候補の model_version および template_version が完全一致する場合のみ通過を許可する。
/// template_version は両方の値が完全一致する必要がある (None と Some("...") も不一致)。
/// 不一致の場合は `ApplicabilityRejected` エラーを返す。
pub fn check_ag07(
    query_version: &EmbeddingChannelVersion,
    candidate_version: &EmbeddingChannelVersion,
) -> Result<(), DarviumError> {
    if query_version.model_version != candidate_version.model_version {
        return Err(DarviumError::ApplicabilityRejected {
            gate: "AG-07".to_string(),
            reason: format!(
                "Structural channel model version mismatch: query='{}', candidate='{}'",
                query_version.model_version, candidate_version.model_version
            ),
        });
    }
    if query_version.template_version != candidate_version.template_version {
        return Err(DarviumError::ApplicabilityRejected {
            gate: "AG-07".to_string(),
            reason: format!(
                "Structural channel template version mismatch: query='{:?}', candidate='{:?}'",
                query_version.template_version, candidate_version.template_version
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // === T1〜T12: check_ag06 / check_ag07 の単体テスト ===

    #[test]
    fn test_ag06_exact_match() {
        let query = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        let candidate = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        assert!(check_ag06(&query, &candidate).is_ok());
    }

    #[test]
    fn test_ag07_exact_match() {
        let query = EmbeddingChannelVersion::new(
            "v2.0-final".into(),
            Some("v2.0-final".into()),
        );
        let candidate = EmbeddingChannelVersion::new(
            "v2.0-final".into(),
            Some("v2.0-final".into()),
        );
        assert!(check_ag07(&query, &candidate).is_ok());
    }

    #[test]
    fn test_ag07_template_none_match() {
        let query = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        let candidate = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        // AG-07 の template_version が両方 None の場合は一致とみなす
        assert!(check_ag07(&query, &candidate).is_ok());
    }

    #[test]
    fn test_ag06_empty_string_match() {
        let query = EmbeddingChannelVersion::new(String::new(), None);
        let candidate = EmbeddingChannelVersion::new(String::new(), None);
        assert!(check_ag06(&query, &candidate).is_ok());
    }

    #[test]
    fn test_ag06_model_version_mismatch() {
        let query = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        let candidate = EmbeddingChannelVersion::new("v1.8-legacy".into(), None);
        let result = check_ag06(&query, &candidate);
        assert!(result.is_err());
        if let Err(DarviumError::ApplicabilityRejected { gate, .. }) = result {
            assert_eq!(gate, "AG-06");
        } else {
            panic!("Expected ApplicabilityRejected");
        }
    }

    #[test]
    fn test_ag07_model_version_mismatch() {
        let query = EmbeddingChannelVersion::new(
            "v2.0-final".into(),
            Some("v2.0-final".into()),
        );
        let candidate = EmbeddingChannelVersion::new(
            "v1.8-legacy".into(),
            Some("v2.0-final".into()),
        );
        let result = check_ag07(&query, &candidate);
        assert!(result.is_err());
        if let Err(DarviumError::ApplicabilityRejected { gate, .. }) = result {
            assert_eq!(gate, "AG-07");
        } else {
            panic!("Expected ApplicabilityRejected");
        }
    }

    #[test]
    fn test_ag07_template_version_some_mismatch() {
        let query = EmbeddingChannelVersion::new(
            "v2.0-final".into(),
            Some("v2.0".into()),
        );
        let candidate = EmbeddingChannelVersion::new(
            "v2.0-final".into(),
            Some("v1.0".into()),
        );
        let result = check_ag07(&query, &candidate);
        assert!(result.is_err());
        if let Err(DarviumError::ApplicabilityRejected { gate, .. }) = result {
            assert_eq!(gate, "AG-07");
        } else {
            panic!("Expected ApplicabilityRejected");
        }
    }

    #[test]
    fn test_ag07_template_version_some_vs_none() {
        let query = EmbeddingChannelVersion::new("v2.0-final".into(), Some("v2.0".into()));
        let candidate = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        let result = check_ag07(&query, &candidate);
        assert!(result.is_err());
        if let Err(DarviumError::ApplicabilityRejected { gate, .. }) = result {
            assert_eq!(gate, "AG-07");
        } else {
            panic!("Expected ApplicabilityRejected");
        }
    }

    #[test]
    fn test_ag07_template_version_none_vs_some() {
        let query = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        let candidate = EmbeddingChannelVersion::new("v2.0-final".into(), Some("v2.0".into()));
        let result = check_ag07(&query, &candidate);
        assert!(result.is_err());
        if let Err(DarviumError::ApplicabilityRejected { gate, .. }) = result {
            assert_eq!(gate, "AG-07");
        } else {
            panic!("Expected ApplicabilityRejected");
        }
    }

    #[test]
    fn test_ag06_case_sensitive_mismatch() {
        let query = EmbeddingChannelVersion::new("V2.0-FINAL".into(), None);
        let candidate = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        assert!(check_ag06(&query, &candidate).is_err());
    }

    #[test]
    fn test_ag06_partial_string_mismatch() {
        let query = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        let candidate = EmbeddingChannelVersion::new("v2.0".into(), None);
        assert!(check_ag06(&query, &candidate).is_err());
    }

    #[test]
    fn test_ag06_empty_vs_nonempty() {
        let query = EmbeddingChannelVersion::new(String::new(), None);
        let candidate = EmbeddingChannelVersion::new("v2.0-final".into(), None);
        assert!(check_ag06(&query, &candidate).is_err());
    }
}
