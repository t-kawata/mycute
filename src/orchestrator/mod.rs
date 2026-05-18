//! オーケストレーター: 音声認識結果のテキストを受け取り、応答を返すパイプライン。
//!
//! 本モジュールは `Orchestrator` トレイトを定義し、そのモック実装を提供する。
//! 本チケットではモック実装までを行い、将来の本実装（RealOrchestrator）と
//! シームレスに差し替え可能な抽象化を提供する。

pub mod mock;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// オーケストレーターエラー
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorError {
    /// 音声認識テキストが空
    #[error("Input text is empty")]
    EmptyInput,
    /// パイプライン処理失敗
    #[error("Pipeline processing failed: {0}")]
    PipelineFailed(String),
    /// 内部エラー
    #[error("Internal error: {0}")]
    Internal(String),
}

/// オーケストレーターへの入力
#[derive(Debug, Clone)]
pub struct OrchestratorInput {
    /// 生の音声認識テキスト
    pub raw_text: String,
    /// オーケストレーターが管理するセッションID（初回入力時に自動発番）
    pub session_id: String,
}

/// オーケストレーターからの出力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorOutput {
    /// ユーザーに表示する応答テキスト（マークダウン形式）
    pub response_text: String,
    /// タスク完了フラグ（true の場合、UI は完了通知を表示する）
    pub task_completed: bool,
}

/// オーケストレーターのインターフェース。
///
/// 音声認識結果のテキストをパイプラインで処理し、応答を返す。
/// session_id の生成・管理はオーケストレーター側が行う。
#[async_trait]
pub trait Orchestrator: Send + Sync {
    /// ユーザー発話をパイプラインで処理し、応答を返す。
    async fn process(&mut self, input: &OrchestratorInput) -> Result<OrchestratorOutput, OrchestratorError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_error_display() {
        let err = OrchestratorError::EmptyInput;
        assert_eq!(format!("{}", err), "Input text is empty");

        let err = OrchestratorError::PipelineFailed("timeout".into());
        assert_eq!(format!("{}", err), "Pipeline processing failed: timeout");

        let err = OrchestratorError::Internal("db error".into());
        assert_eq!(format!("{}", err), "Internal error: db error");
    }

    #[test]
    fn test_orchestrator_error_clone_eq() {
        let err = OrchestratorError::EmptyInput;
        assert_eq!(err.clone(), OrchestratorError::EmptyInput);
    }

    #[test]
    fn test_orchestrator_input_construction() {
        let input = OrchestratorInput {
            raw_text: "hello".into(),
            session_id: "sess-001".into(),
        };
        assert_eq!(input.raw_text, "hello");
        assert_eq!(input.session_id, "sess-001");
    }

    #[test]
    fn test_orchestrator_output_construction() {
        let output = OrchestratorOutput {
            response_text: "# Hello".into(),
            task_completed: false,
        };
        assert_eq!(output.response_text, "# Hello");
        assert!(!output.task_completed);

        let completed = OrchestratorOutput {
            response_text: "done".into(),
            task_completed: true,
        };
        assert!(completed.task_completed);
    }
}
