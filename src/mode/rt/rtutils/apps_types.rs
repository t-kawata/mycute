use crate::constants::{APP_LAYER_LOCAL, APP_LAYER_PREINSTALL, APP_LAYER_REMOTE};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

/// アプリケーションの配置レイヤーを定義する Enum。
/// DB 上は文字列として保存されるが、コード内では型安全に扱う。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AppLayer {
    /// システムにあらかじめ組み込まれているアプリ (削除不可)
    Preinstall,
    /// ユーザーがローカルファイルからインストールしたアプリ
    Local,
    /// リモートサーバーから取得しインストールしたアプリ
    Remote,
}

impl AppLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppLayer::Preinstall => APP_LAYER_PREINSTALL,
            AppLayer::Local => APP_LAYER_LOCAL,
            AppLayer::Remote => APP_LAYER_REMOTE,
        }
    }
}

impl fmt::Display for AppLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<AppLayer> for String {
    fn from(layer: AppLayer) -> Self {
        layer.as_str().to_string()
    }
}

impl std::str::FromStr for AppLayer {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == APP_LAYER_PREINSTALL {
            Ok(AppLayer::Preinstall)
        } else if s == APP_LAYER_LOCAL {
            Ok(AppLayer::Local)
        } else if s == APP_LAYER_REMOTE {
            Ok(AppLayer::Remote)
        } else {
            Err(())
        }
    }
}
