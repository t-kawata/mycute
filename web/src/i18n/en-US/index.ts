// en-US
export default {
  app: {
    fab: {
      overlay: {
        on: 'Close Overlay',
        off: 'Overlay View'
      },
      alwaysOnTop: {
        on: 'Unpin from Top',
        off: 'Always On Top'
      },
      logout: 'Logout',
      restart: 'Restart',
      shutdown: 'Shutdown'
    },
    settings: {
      englishMode: '英語モード',
      englishModeDescription: 'このスイッチをONにするとアプリ全体が英語になります。OFFにすると日本語になります。',
      sttEngine: 'STT Engine',
      sttEngineDescription: 'Select the speech recognition engine to use.',
      sttEngineOs: 'OS Native',
      sttEngineOpenAI: 'OpenAI',
      llmSettings: 'LLM Settings',
      llmSettingsDescription: 'Configure the Large Language Models (LLM) used for AI interaction. Multiple settings will be used in round robin.',
      llmName: 'Name',
      llmBaseUrl: 'Base URL',
      llmApiKey: 'API Key',
      llmModel: 'Model',
      llmAdd: 'Add LLM Endpoint',
      danger: 'Danger Zone',
      dangerDescription: 'Operations here are irreversible, including application reset.',
      resetApplication: 'Reset Application',
      resetConfirm: 'Are you sure you want to reset the application? This action cannot be undone, and all data will be permanently lost.',
      resetFailed: 'Failed to reset application.',
      ownerActivation: 'Owner Activation',
      ownerPassphrase: 'Owner Passphrase',
      activate: 'Activate',
      ownerModeActivated: 'Owner Mode Activated.',
      ownerModeDeactivated: 'Owner Mode Deactivated.',
      invalidPassphrase: 'Invalid Passphrase.',
      ownerModeActive: 'Owner Mode Active',
      rootAuthority: 'You have Owner Authority.',
      myPubKey: 'Your Public Key',
      copyPubKey: 'Public key copied to clipboard',
      genCaToken: 'Generate CA Cert',
      genCaTokenDescription: 'Generate a CA Cert from the target public key.',
      verifyCaToken: 'Verify CA Cert',
      verifyCaTokenDescription: 'Verify a CA Cert and check its validity.',
      genCaTokenDialogTitle: 'Generate CA Cert',
      targetPubKey: 'Target Public Key (Hex)',
      targetPubKeyHint: 'Please enter the target node\'s public key (Hex).',
      expireHours: 'Expire Hours',
      expireHoursHint: 'Expiration time (hours, default 336h = 14 days).',
      issueAndCopy: 'Issue and Copy',
      enterPubKey: 'Please enter the public key.',
      enterValidHours: 'Please enter valid hours.',
      genCaTokenSuccess: 'CA Cert copied to clipboard.',
      genCaTokenFail: 'Failed to generate CA Cert.',
      verifyCaTokenDialogTitle: 'Verify CA Cert',
      enterCaToken: 'Please enter the CA Cert.',
      verify: 'Verify',
      caTokenInputLabel: 'CA Cert String',
      verificationResult: 'Verification Result',
      caPubKey: 'CA Public Key',
      expireAt: 'Expire At',
      tokenValid: 'This CA Cert is VALID.',
      tokenInvalid: 'This CA Cert is INVALID.',
      verifyCaTokenSuccess: 'Verification successful.',
      verifyCaTokenFail: 'Verification failed.',
      copyPubKeySuccess: 'CA Public Key copied to clipboard.'
    },
    common: {
      cancel: 'Cancel',
      reset: 'Reset'
    }
  },
  page: {
    login: {
      signin: 'Sign In',
      signup: 'Sign Up',
      reset: 'Reset Up',
      createAccount: 'Create Account',
      login: 'Login',
      registerVdrKey: 'Register Vendor Key',
      error: {
        failedToSignIn: 'Invalid authentication information',
        failedToSignUp: 'Invalid input content',
        failedToValidateVdrKey: 'Invalid VDR-KEY',
        requiredVdrKey: 'Please enter VDR-KEY',
        autoSetupFailed: 'Auto Setup Failed',
        comingSoon: 'Under preparation'
      }
    },
    index: {
      calendar: {
        ok: 'OK',
        ng: 'NG'
      },
      search: {
        ok: 'OK',
        ng: 'NG',
        salary: 'Job Salary'
      },
      friends: {
        give: 'GIVE',
        none: 'NONE',
        point: 'Gift Point'
      }
    }
  }
};
