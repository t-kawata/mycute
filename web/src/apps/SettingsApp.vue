<template>
  <div class="__mycute-glass-app-container q-pa-sm">
    <div class="__mycute-glass-panel-inner">
      <WaterRipple />
      <div class="__mycute-tabpanel-container __mycute-tabpanel-container-settings __mycute-tabpanel-container-settings-app" style="flex: 1; padding: 0; height: auto !important; overflow: visible !important;">
        <q-list>
      <!-------------- オーナー表示 bgn ---------------->
      <q-item v-if="mainStore.isOwnerActive" key="owner-display" class="q-px-none" style="border-radius: 8px;">
        <q-item-section avatar>
          <q-avatar color="dark" text-color="white">
            <Crown4Icon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.ownerModeActive') }}</q-item-label>
          <q-item-label caption class="text-dark">{{ t('app.settings.rootAuthority') }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- オーナー表示 end ---------------->

      <!-------------- 一行 bgn ---------------->
      <q-item v-if="mainStore.isOwnerActive" key="gen-ca-token" class="q-px-none q-mt-sm" clickable @click="mainStore.setIsGenCaTokenDialogOpen(true)" style="user-select: none;">
        <q-item-section avatar>
          <q-avatar color="dark" text-color="white">
            <CreditCardAIIcon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.genCaToken') }}</q-item-label>
          <q-item-label caption class="text-dark">{{ t('app.settings.genCaTokenDescription') }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- 一行 end ---------------->

      <q-separator v-if="mainStore.isOwnerActive" key="owner-sep" class="q-my-md"/>

      <!-------------- CA表示 bgn ---------------->
      <q-item v-if="mainStore.isCaActive" key="ca-display" class="q-px-none q-mt-sm" style="border-radius: 8px;">
        <q-item-section avatar>
          <q-avatar color="dark" text-color="white">
            <CreditCardCheckCircleIcon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.caModeActive') }}</q-item-label>
          <q-item-label caption class="text-dark">{{ t('app.settings.caAuthorized') }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- CA表示 end ---------------->

      <!-------------- 一行 bgn ---------------->
      <q-item v-if="mainStore.isCaActive" key="my-ca-token" class="q-px-none q-mt-sm" clickable @click="copyCaToken" style="user-select: none;">
        <q-item-section avatar>
          <q-avatar color="dark" text-color="white">
            <CreditCardUserIcon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.myCaToken') }}</q-item-label>
          <q-item-label caption class="ellipsis text-dark">{{ mainStore.caToken }}</q-item-label>
        </q-item-section>
        <q-item-section v-if="mainStore.isOwnerActive" side>
          <q-btn flat round color="negative" icon="delete_forever" @click.stop="onUnregCaTokenClicked" />
        </q-item-section>
      </q-item>
      <!-------------- 一行 end ---------------->

      <!-------------- ライセンスの発行 bgn ---------------->
      <q-item v-if="mainStore.isCaActive" key="gen-license" class="q-px-none q-mt-sm" clickable @click="mainStore.setIsGenLicenseDialogOpen(true)" style="user-select: none;">
        <q-item-section avatar>
          <q-avatar color="dark" text-color="white">
            <BookmarkPencilIcon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.genLicense') }}</q-item-label>
          <q-item-label caption class="text-dark">{{ t('app.settings.genLicenseDescription') }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- ライセンスの発行 end ---------------->

      <q-separator v-if="mainStore.isCaActive" key="ca-sep" class="q-my-md"/>

      <!-------------- ライセンス管理 bgn ---------------->

      <!-------------- ライセンス検証ボタン bgn ---------------->
      <q-item key="verify-license" class="q-px-none q-mt-sm" clickable @click="mainStore.setIsVerifyLicenseDialogOpen(true)" style="user-select: none;">
        <q-item-section avatar>
          <q-avatar color="app" text-color="white">
            <BookmarkSearchIcon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.verifyLicense') }}</q-item-label>
          <q-item-label caption class="text-dark">{{ t('app.settings.verifyLicenseDescription') }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- ライセンス検証ボタン end ---------------->

      <!-------------- ライセンス登録ボタン bgn ---------------->
      <q-item key="reg-license" class="q-px-none q-mt-sm" clickable @click="mainStore.setIsRegisterLicenseDialogOpen(true)" style="user-select: none;">
        <q-item-section avatar>
          <q-avatar color="app" text-color="white">
            <BookmarkPlusCircleIcon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.regLicense') }}</q-item-label>
          <q-item-label caption class="text-dark">{{ t('app.settings.regLicenseDescription') }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- ライセンス登録ボタン end ---------------->

      <!-------------- 登録済みライセンス一覧（アコーディオン）bgn ---------------->
      <q-expansion-item key="licenses-expansion" v-model="isLicensesExpanded" class="q-px-none" header-class="q-px-none" @show="onLicenseExpand">
          <template v-slot:header>
            <q-item-section avatar>
              <q-avatar color="app" text-color="white">
                <AllBookmark1Icon style="width: 24px; height: 24px;" />
              </q-avatar>
            </q-item-section>
            <q-item-section>
              <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.myLicenses') }}</q-item-label>
              <q-item-label caption class="text-dark">{{ mainStore.licenses.length }} {{ t('app.settings.myLicensesUnit') }}</q-item-label>
            </q-item-section>
          </template>
          <!-- ライセンス一覧コンテンツ -->
          <div class="q-mt-sm q-mb-md">
            <div v-if="mainStore.licenses.length === 0" class="text-center text-caption q-pa-md text-grey-8">
              {{ t('app.settings.noLicenses') }}
            </div>
            <div v-for="lic in mainStore.licenses" :key="lic.id" class="q-mx-sm q-mb-md q-pa-md bg-app text-dark relative-position shadow-6 __mycute-settings-license-card" style="border-radius: 16px; border: 2px solid rgba(0,0,0,0.1);">
              <!-- ツールボタン (Floating) -->
              <div class="absolute-top-right q-pa-sm flex q-gutter-xs" style="z-index: 2;">
                <q-btn flat round dense color="dark" icon="copy_all" @click="copyLicense(lic.raw)" />
                <q-btn flat round dense color="dark" icon="delete_forever" @click="onUnregisterLicense(lic.id)" />
              </div>

              <!-- 詳細リスト: VerifyLicenseDialog と一貫した順序 -->
              <q-list class="relative-position" style="z-index: 1;">
                <q-item class="q-px-none">
                  <q-item-section>
                    <q-item-label caption class="text-dark text-weight-bold">ID</q-item-label>
                    <q-item-label class="break-all text-caption" style="word-break: break-all;">
                      {{ lic.id }}
                    </q-item-label>
                  </q-item-section>
                </q-item>

                <q-item class="q-px-none">
                  <q-item-section>
                    <q-item-label caption class="text-dark text-weight-bold">{{ t('app.settings.expireAt') }}</q-item-label>
                    <q-item-label class="text-caption">
                      {{ formatExpireDate(lic.expire_at) }}
                    </q-item-label>
                  </q-item-section>
                </q-item>

                <q-item class="q-px-none">
                  <q-item-section>
                    <q-item-label caption class="text-dark text-weight-bold">{{ t('app.settings.caPubKey') }}</q-item-label>
                    <q-item-label class="ellipsis text-caption">
                      {{ lic.ca_pubkey }}
                    </q-item-label>
                  </q-item-section>
                </q-item>

                <!-- 権限 (JSON) -->
                <div v-if="lic.permissions" class="q-mt-sm">
                  <div class="text-caption text-dark text-weight-bold">{{ t('app.settings.grantedPermissions') }}</div>
                  <pre class="bg-dark q-pa-sm q-ma-none q-mt-xs text-caption text-white" style="border-radius: 6px; overflow: auto; max-height: 120px; font-family: monospace;">{{ JSON.stringify(lic.permissions, null, 4) }}</pre>
                </div>
              </q-list>
            </div>
          </div>
        </q-expansion-item>
      <!-------------- 登録済みライセンス一覧（アコーディオン）end ---------------->

      <!-------------- ライセンス管理 end ---------------->

      <q-separator key="ca-sep" class="q-my-md"/>

      <!-------------- CA任命証の検証 bgn ---------------->
      <q-item key="verify-ca-token" class="q-px-none q-mt-sm" clickable @click="mainStore.setIsVerifyCaTokenDialogOpen(true)" style="user-select: none;">
        <q-item-section avatar>
          <q-avatar color="app" text-color="white">
            <CreditCardSearchIcon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.verifyCaToken') }}</q-item-label>
          <q-item-label caption class="text-dark">{{ t('app.settings.verifyCaTokenDescription') }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- CA任命証の検証 end ---------------->

      <!-------------- CA任命証の登録 bgn ---------------->
      <q-item key="reg-ca-token" class="q-px-none q-mt-sm" clickable @click="mainStore.setIsRegisterCaTokenDialogOpen(true)" style="user-select: none;">
        <q-item-section avatar>
          <q-avatar color="app" text-color="white">
            <CreditCardPlusCircleIcon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label class="text-dark text-weight-bold">{{ t('app.settings.regCaToken') }}</q-item-label>
          <q-item-label caption class="text-dark">{{ t('app.settings.regCaTokenDescription') }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- CA任命証の登録 end ---------------->

      <q-separator key="last-sep" class="q-my-md"/>
      
      <!-------------- 一行 bgn ---------------->
      <q-item key="version" class="q-px-none q-mt-sm" clickable @click="onVersionClicked" style="user-select: none;">
        <q-item-section avatar>
          <q-avatar color="primary" text-color="white">
            <img :src="LOGO_IMG_WHITE_SRC" style="
                width: 28px !important;
                height: 28px !important;
                position: relative;
                top: -1px;
              " />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label>Version</q-item-label>
          <q-item-label caption>{{ MYCUTE_VERSION }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- 一行 end ---------------->
      <!-------------- 一行 bgn ---------------->
      <q-item key="my-pubkey" class="q-px-none q-mt-sm" clickable @click="copyPubKey" style="user-select: none;">
        <q-item-section avatar>
          <q-avatar color="primary" text-color="white">
            <KeyAI1Icon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label>{{ t('app.settings.myPubKey') }}</q-item-label>
          <q-item-label caption class="ellipsis">{{ mainStore.myPubKey }}</q-item-label>
        </q-item-section>
      </q-item>
      <!-------------- 一行 end ---------------->
      <!-------------- 一行 bgn ---------------->
      <q-item key="english-mode" class="q-px-none q-mt-sm">
        <q-item-section avatar>
          <q-avatar color="primary" text-color="white">
            <FontSquareIcon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label>{{ t("app.settings.englishMode") }}</q-item-label>
          <q-item-label caption>{{
            t("app.settings.englishModeDescription")
            }}</q-item-label>
        </q-item-section>
        <q-item-section side>
          <q-toggle color="primary" v-model="isEn" val="battery" />
        </q-item-section>
      </q-item>
      <!-------------- 一行 end ---------------->
      <!-------------- 一行 bgn ---------------->
      <q-item key="stt-engine" class="q-px-none q-mt-sm">
        <q-item-section avatar>
          <q-avatar color="primary" text-color="white">
            <MicAI1Icon style="width: 24px; height: 24px;" />
          </q-avatar>
        </q-item-section>
        <q-item-section>
          <q-item-label>{{ t("app.settings.sttEngine") }}</q-item-label>
          <q-item-label caption>{{
            t("app.settings.sttEngineDescription")
            }}</q-item-label>
        </q-item-section>
        <q-item-section side>
          <q-select filled v-model="sttEngine" :options="sttEngineOptions" emit-value map-options dense
            style="margin-top: 8px" />
        </q-item-section>
      </q-item>
      <!-------------- 一行 end ---------------->

      <q-separator key="danger-sep" class="q-my-md"/>
      <!-------------- 一行 bgn ---------------->
      <q-expansion-item key="danger-zone" v-model="isDangerExpanded" class="q-px-none" header-class="q-px-none">
          <template v-slot:header>
            <q-item-section avatar>
              <q-avatar color="negative" text-color="white">
                <Bot2ErrorIcon style="width: 24px; height: 24px;" />
              </q-avatar>
            </q-item-section>
            <q-item-section>
              <q-item-label color="negative">{{ t("app.settings.danger") }}</q-item-label>
              <q-item-label caption>{{ t("app.settings.dangerDescription") }}</q-item-label>
            </q-item-section>
          </template>
          <!-- Danger コンテンツ -->
          <div class="q-mt-sm q-mb-md">
            <q-btn class="full-width" color="negative" icon="restore" :label="t('app.settings.resetApplication')" @click="mainStore.setIsResetConfirmOpen(true)" />
          </div>
        </q-expansion-item>
      <!-------------- 一行 end ---------------->
    </q-list>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import WaterRipple from "src/components/effects/WaterRipple.vue";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { useMainStore } from "src/stores/main-store";
import { LANG, useLangSetter, t } from "src/utils/some";
import { showNotify } from "src/utils/notify";
import {
  ENGINE_OPENAI,
  ENGINE_OS,
  MYCUTE_VERSION,
} from "src/consts/generated_constants";
import { LOGO_IMG_WHITE_SRC } from "src/configs/settings";
import { formatExpireDate } from "src/utils/some";
import Bot2ErrorIcon from "src/components/icons/Bot2ErrorIcon.vue";
import MicAI1Icon from "src/components/icons/MicAI1Icon.vue";
import Crown4Icon from "src/components/icons/Crown4Icon.vue";
import KeyAI1Icon from "src/components/icons/KeyAI1Icon.vue";
import FontSquareIcon from "src/components/icons/FontSquareIcon.vue";
import CreditCardAIIcon from "src/components/icons/CreditCardAIIcon.vue";
import CreditCardCheckCircleIcon from "src/components/icons/CreditCardCheckCircleIcon.vue";
import CreditCardUserIcon from "src/components/icons/CreditCardUserIcon.vue";
import AllBookmark1Icon from "src/components/icons/AllBookmark1Icon.vue";
import BookmarkSearchIcon from "src/components/icons/BookmarkSearchIcon.vue";
import CreditCardPlusCircleIcon from "src/components/icons/CreditCardPlusCircleIcon.vue";
import CreditCardSearchIcon from "src/components/icons/CreditCardSearchIcon.vue";
import BookmarkPlusCircleIcon from "src/components/icons/BookmarkPlusCircleIcon.vue";
import BookmarkPencilIcon from "src/components/icons/BookmarkPencilIcon.vue";

const mainStore = useMainStore();
const langSetter = useLangSetter();
const isDangerExpanded = ref(false);

async function copyPubKey() {
  if (!mainStore.myPubKey) return;
  await writeText(mainStore.myPubKey);
  showNotify(t("app.settings.copyPubKey"));
}

async function copyCaToken() {
  if (!mainStore.caToken) return;
  await writeText(mainStore.caToken);
  showNotify(t("app.settings.genCaTokenSuccess"));
}

async function onUnregCaTokenClicked() {
  mainStore.setIsUnregisterCaTokenConfirmOpen(true)
}


let versionClickTimer: ReturnType<typeof setTimeout> | null = null;
const versionClickCount = ref(0);
function onVersionClicked() {
  if (versionClickTimer) clearTimeout(versionClickTimer);

  versionClickCount.value++;
  if (versionClickCount.value >= 10) {
    versionClickCount.value = 0;
    if (mainStore.isOwnerActive) {
      mainStore.deactivateOwner();
    } else {
      promptOwnerPassphrase();
    }
  } else {
    versionClickTimer = setTimeout(() => {
      versionClickCount.value = 0;
    }, 500);
  }
}

function promptOwnerPassphrase() {
  mainStore.setIsOwnerActivateConfirmOpen(true);
}

const isEn = computed({
  get() {
    return mainStore.lang === LANG.EN.LONG;
  },
  set(newValue: boolean) {
    if (newValue) langSetter.setLangEN();
    else langSetter.setLangJA();
  },
});

const sttEngine = computed({
  get() {
    return mainStore.sttEngine;
  },
  set(newValue: string) {
    mainStore.setSttEngine(newValue);
  },
});

const sttEngineOptions = computed(() => [
  { label: t("app.settings.sttEngineOs"), value: ENGINE_OS },
  { label: t("app.settings.sttEngineOpenAI"), value: ENGINE_OPENAI },
]);


// ============================================================
// ライセンス管理
// ============================================================


/** ライセンス一覧アコーディオンの開閉状態 */
const isLicensesExpanded = ref(false);

async function onLicenseExpand() {
  await mainStore.fetchLicenses();
}

/** ライセンス文字列をクリップボードにコピーする。 */
async function copyLicense(raw: string) {
  await writeText(raw);
  showNotify(t('app.settings.copyLicense'));
}

/** ライセンス削除確認ダイアログを開く。 */
function onUnregisterLicense(id: string) {
  mainStore.setLicenseIdToUnregister(id)
  mainStore.setIsUnregisterLicenseConfirmOpen(true)
}
</script>

<style scoped>
.__mycute-settings-license-card::before {
  content: "";
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-image: url('/mycute-white-256.png');
  background-size: 78%;
  background-position: 90% 15%;
  background-repeat: no-repeat;
  opacity: 0.15;
  pointer-events: none;
  z-index: 0;
}
</style>
