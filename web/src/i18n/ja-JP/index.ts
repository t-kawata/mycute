// ja-JP
export default {
  app: {
    fab: {
      overlay: {
        on: 'オーバーレイ解除',
        off: 'オーバーレイ表示'
      },
      alwaysOnTop: {
        on: '最前面固定を解除',
        off: '最前面に固定'
      },
      logout: 'ログアウト',
      restart: '再起動',
      shutdown: '終了'
    },
    settings: {
      englishMode: 'English Mode',
      englishModeDescription: 'When this switch is turned ON, the entire app will be in English. When it is turned OFF, it will be in Japanese.',
      sttEngine: '音声認識エンジン',
      sttEngineDescription: '使用する音声認識エンジンを選択します。',
      sttEngineOs: 'OS標準',
      sttEngineOpenAI: 'OpenAI',
      llmSettings: '言語モデル設定',
      llmSettingsDescription: 'AIとの対話に使用する言語モデル（LLM）の設定を行います。複数の設定を行うと、ラウンドロビンで使用されます。',
      llmName: '表示名',
      llmBaseUrl: 'ベースURL',
      llmApiKey: 'APIキー',
      llmModel: 'モデル名',
      llmAdd: 'LLMエンドポイントを追加',
      danger: '危険な操作',
      dangerDescription: 'アプリケーションの初期化など、取り返しのつかない操作が含まれます。',
      resetApplication: 'アプリをリセット',
      resetConfirm: '本当にアプリケーションをリセットしますか？この操作は取り消せず、全てのデータは永久に失われます。',
      resetFailed: 'リセットに失敗しました。'
    },
    common: {
      cancel: 'キャンセル',
      reset: 'リセット'
    }
  },
  page: {
    login: {
      signin: 'Sign In',
      signup: 'Sign Up',
      createAccount: 'アカウントを作成する',
      login: 'ログインする',
      registerVdrKey: 'ベンダー鍵を登録する',
      error: {
        failedToSignIn: '認証情報が正しくありません',
        failedToSignUp: '入力内容が正しくありません',
        failedToValidateVdrKey: 'VDR-KEYが正しくありません',
        requiredVdrKey: 'VDR-KEYを入力してください',
        autoSetupFailed: '自動セットアップに失敗しました',
        comingSoon: '現在準備中です'
      }
    },
    index: {
      calendar: {
        ok: 'あり',
        ng: 'なし'
      },
      search: {
        ok: 'あり',
        ng: 'なし',
        salary: 'お仕事収入'
      },
      friends: {
        give: 'あげる',
        none: 'やめる',
        point: 'Gift Point'
      }
    }
  }
};
