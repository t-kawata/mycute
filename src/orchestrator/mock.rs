//! MockOrchestrator: ラリーカウント・セッション自動生成・3往復で完了するモック実装。

use async_trait::async_trait;
use uuid::Uuid;

use super::{Orchestrator, OrchestratorError, OrchestratorInput, OrchestratorOutput};

/// モックオーケストレーター
///
/// # 動作
/// - 空入力 → `EmptyInput` エラー
/// - 1回目の `process()` 呼び出しで UUID v4 を自動生成して session_id として保持する
/// - 呼び出しのたびに内部ラリーカウンターをインクリメントする
/// - 入力をそのまま `response_text` としてエコーバックする
/// - 3回のラリー（ラリーカウントが3に達する）で `task_completed: true` を返す
/// - 完了後はラリーカウントを0にリセットし、次の入力を待つ
pub struct MockOrchestrator {
    /// 初回呼び出し時に発番されるセッションID
    session_id: Option<String>,
    /// 現在のラリーカウント（0始まり、3で完了）
    rally_count: u32,
}

impl MockOrchestrator {
    /// 新しい MockOrchestrator を生成する。
    pub fn new() -> Self {
        Self {
            session_id: None,
            rally_count: 0,
        }
    }
}

impl Default for MockOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Orchestrator for MockOrchestrator {
    async fn process(
        &mut self,
        input: &OrchestratorInput,
    ) -> Result<OrchestratorOutput, OrchestratorError> {
        // 空入力・空白のみの入力を検証する
        if input.raw_text.trim().is_empty() {
            return Err(OrchestratorError::EmptyInput);
        }

        // 初回呼び出し時に session_id を自動生成する
        if self.session_id.is_none() {
            self.session_id = Some(Uuid::new_v4().to_string());
        }

        // ラリーカウントをインクリメントする
        self.rally_count += 1;

        let task_completed = self.rally_count >= 3;

        // 完了後はラリーカウントをリセットする
        if task_completed {
            self.rally_count = 0;
        }

        Ok(OrchestratorOutput {
            response_text: input.raw_text.clone(),
            task_completed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ヘルパー: テスト用の OrchestratorInput を生成する
    fn make_input(raw_text: &str) -> OrchestratorInput {
        OrchestratorInput {
            raw_text: raw_text.to_string(),
            session_id: String::new(),
        }
    }

    #[tokio::test]
    async fn test_echo_back() {
        let mut orch = MockOrchestrator::new();
        let output = orch.process(&make_input("hello world")).await.unwrap();
        assert_eq!(output.response_text, "hello world");
    }

    #[tokio::test]
    async fn test_session_id_auto_generated() {
        let mut orch = MockOrchestrator::new();
        let input = OrchestratorInput {
            raw_text: "test".into(),
            session_id: String::new(),
        };
        let output = orch.process(&input).await.unwrap();
        // 入力をそのままエコーバック
        assert_eq!(output.response_text, "test");
        // 内部で session_id が生成されていることを確認する
        assert!(orch.session_id.is_some());
        let sid1 = orch.session_id.clone().unwrap();
        // UUID v4 形式であることを確認する
        assert_eq!(sid1.len(), 36);
        assert_eq!(sid1.chars().filter(|&c| c == '-').count(), 4);

        // 2回目の呼び出しでも同じ session_id が維持される
        let _ = orch.process(&input).await.unwrap();
        let sid2 = orch.session_id.unwrap();
        assert_eq!(sid1, sid2);
    }

    #[tokio::test]
    async fn test_rally_increment() {
        let mut orch = MockOrchestrator::new();
        assert_eq!(orch.rally_count, 0);

        let _ = orch.process(&make_input("first")).await.unwrap();
        assert_eq!(orch.rally_count, 1);

        let _ = orch.process(&make_input("second")).await.unwrap();
        assert_eq!(orch.rally_count, 2);
    }

    #[tokio::test]
    async fn test_completion_at_three_rallies() {
        let mut orch = MockOrchestrator::new();

        let out1 = orch.process(&make_input("one")).await.unwrap();
        assert!(!out1.task_completed);

        let out2 = orch.process(&make_input("two")).await.unwrap();
        assert!(!out2.task_completed);

        let out3 = orch.process(&make_input("three")).await.unwrap();
        assert!(out3.task_completed);
        assert_eq!(out3.response_text, "three");
    }

    #[tokio::test]
    async fn test_reset_after_completion() {
        let mut orch = MockOrchestrator::new();

        // 3回呼び出して完了させる
        let _ = orch.process(&make_input("a")).await.unwrap();
        let _ = orch.process(&make_input("b")).await.unwrap();
        let completed = orch.process(&make_input("c")).await.unwrap();
        assert!(completed.task_completed);

        // リセット後、再度1ラリー目としてカウントされる
        let out = orch.process(&make_input("d")).await.unwrap();
        assert!(!out.task_completed);
        assert_eq!(orch.rally_count, 1);
    }

    #[tokio::test]
    async fn test_empty_input_error() {
        let mut orch = MockOrchestrator::new();
        let result = orch.process(&make_input("")).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), OrchestratorError::EmptyInput);
    }

    #[tokio::test]
    async fn test_whitespace_only_input_error() {
        let mut orch = MockOrchestrator::new();
        let result = orch.process(&make_input("   ")).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), OrchestratorError::EmptyInput);
    }
}
