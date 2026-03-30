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
      resetFailed: 'リセットに失敗しました。',
      ownerActivation: 'Owner Activation',
      ownerPassphrase: 'Owner Passphrase',
      activate: 'アクティベート',
      ownerModeActivated: 'オーナーモードを有効化しました。',
      ownerModeDeactivated: 'オーナーモードを解除しました。',
      invalidPassphrase: 'パスフレーズが正しくありません。',
      ownerModeActive: 'オーナーモード有効',
      rootAuthority: 'オーナー権限が付与されています。',
      myPubKey: 'あなたの公開鍵',
      copyPubKey: '公開鍵をクリップボードにコピーしました',
      genCaToken: 'CA任命証の発行',
      genCaTokenDescription: '対象の公開鍵からCA任命証を発行します',
      verifyCaToken: 'CA任命証の検証',
      verifyCaTokenDescription: 'CA任命証を送信して妥当性を検証します',
      genCaTokenDialogTitle: 'CA任命証の生成',
      targetPubKey: '対象の公開鍵',
      targetPubKeyHint: '対象の公開鍵（hex）を入力してください。',
      expireHours: '有効期限',
      expireHoursHint: '有効期限（hours, 初期値 336時間 = 14日）',
      issueAndCopy: '発行してコピー',
      enterPubKey: '公開鍵を入力してください',
      enterValidHours: '有効な期間を入力してください',
      genCaTokenSuccess: 'CA任命証をクリップボードにコピーしました',
      genCaTokenFail: 'CA任命証の生成に失敗しました',
      verifyCaTokenDialogTitle: 'CA任命証の検証',
      enterCaToken: 'CA任命証を入力してください',
      verify: '検証実行',
      caTokenInputLabel: 'CA任命証（トークン文字列）',
      verificationResult: '検証結果',
      caPubKey: 'CA公開鍵',
      expireAt: '有効期限',
      tokenValid: 'このCA任命証は【有効】です。',
      tokenInvalid: 'このCA任命証は【無効】です。',
      verifyCaTokenSuccess: '検証に成功しました。',
      verifyCaTokenFail: '検証に失敗しました。',
      copyPubKeySuccess: 'CA公開鍵をクリップボードにコピーしました'
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
      reset: 'Reset Up',
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
