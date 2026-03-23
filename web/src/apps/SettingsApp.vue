<template>
  <div class="__harunohi-tabpanel-container __harunohi-tabpanel-container-settings">
    <q-list>
      <!-------------- 一行 bgn ---------------->
      <q-item class="q-px-none">
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
      <q-item class="q-px-none q-mt-sm">
        <q-item-section avatar>
          <q-avatar color="primary" text-color="white">
            <q-icon name="translate" />
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
      <q-item class="q-px-none q-mt-sm">
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
      <!-------------- 一行 bgn ---------------->
      <q-item-section>
        <q-expansion-item v-model="isLlmExpanded" class="q-px-none" header-class="q-px-none">
          <template v-slot:header>
            <q-item-section avatar>
              <q-avatar color="primary" text-color="white">
                <BrainAI1Icon style="width: 24px; height: 24px;" />
              </q-avatar>
            </q-item-section>
            <q-item-section>
              <q-item-label>{{ t("app.settings.llmSettings") }}</q-item-label>
              <q-item-label caption>{{
                t("app.settings.llmSettingsDescription")
                }}</q-item-label>
            </q-item-section>
          </template>
          <!-- LLM エンドポイント一覧 -->
          <div>
            <div v-for="(llm, idx) in localLlms" :key="idx" class="__harunohi-llm-entry q-mt-sm">
              <q-input dense label-color="white" bg-color="accent" input-style="color: white;" filled v-model="llm.name" :label="t('app.settings.llmName')" class="q-mb-xs" @update:model-value="onLlmChanged" />
              <q-input dense outlined color="dark" v-model="llm.base_url" :label="t('app.settings.llmBaseUrl')" class="q-mb-xs" @update:model-value="onLlmChanged" />
              <q-input dense outlined color="dark" v-model="llm.api_key" :label="t('app.settings.llmApiKey')" type="password" class="q-mb-xs" @update:model-value="onLlmChanged" />
              <q-input dense outlined color="dark" v-model="llm.model" :label="t('app.settings.llmModel')" class="q-mb-xs" @update:model-value="onLlmChanged">
                <template v-slot:after>
                  <q-btn flat round dense icon="delete_forever" color="negative" @click="removeLlm(idx)" />
                </template>
              </q-input>
              <q-separator v-if="idx < localLlms.length - 1" style="margin-top: 8px; margin-bottom: 4px" />
            </div>
            <!-- 追加ボタン -->
            <div class="flex justify-end q-mt-md">
              <q-btn class="full-width" rounded dense icon="add" color="purple" :label="t('app.settings.llmAdd')" @click="addLlm" />
            </div>
          </div>
        </q-expansion-item>
      </q-item-section>
      <!-------------- 一行 end ---------------->
      <q-separator class="q-my-md"/>
      <!-------------- 一行 bgn ---------------->
      <q-item-section style="margin-left: 0;">
        <q-expansion-item v-model="isDangerExpanded" class="q-px-none" header-class="q-px-none">
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
      </q-item-section>
      <!-------------- 一行 end ---------------->
    </q-list>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useQuasar } from "quasar";
import { useRouter } from "vue-router";
import { URL } from "src/router/routes";
import { invoke } from "@tauri-apps/api/core";
import { useMainStore } from "src/stores/main-store";
import { LANG, useLangSetter, t } from "src/utils/some";
import {
  ENGINE_OPENAI,
  ENGINE_OS,
  MYCUTE_VERSION,
  DEFAULT_LLM_NAME,
  DEFAULT_LLM_BASE_URL,
  DEFAULT_LLM_API_KEY,
  DEFAULT_LLM_MODEL,
} from "src/consts/generated_constants";
import { LOGO_IMG_WHITE_SRC } from "src/configs/settings";
import { setMycuteLlms, type LlmEndpoint } from "src/utils/rest";
import Bot2ErrorIcon from "src/components/icons/Bot2ErrorIcon.vue";
import BrainAI1Icon from "src/components/icons/BrainAI1Icon.vue";
import MicAI1Icon from "src/components/icons/MicAI1Icon.vue";

const mainStore = useMainStore();
const langSetter = useLangSetter();
const $q = useQuasar();
const router = useRouter();

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
// LLM 設定（500ms デバウンスによるオートセーブ）
// ============================================================

/** 折りたたみ状態（初期値は閉じている） */
const isLlmExpanded = ref(false);

/** 危険設定の折りたたみ状態（初期値は閉じている） */
const isDangerExpanded = ref(false);

/** ローカル編集用のコピー。バックエンドとの同期前に一時的な値を保持する。 */
const localLlms = ref<LlmEndpoint[]>([]);

/** バックエンドの llms が変更されたらローカルコピーを更新する。 */
watch(
  () => mainStore.llms,
  (newLlms) => {
    // ユーザーが編集中に上書きされないよう、deep equal チェックを省略して常に同期する
    localLlms.value = newLlms.map((l) => ({ ...l }));
  },
  { immediate: true, deep: true },
);

/** デバウンスタイマーの参照（クリアに使用）。 */
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

/** 入力変更時に呼ばれるデバウンスハンドラ（500ms 後に保存）。 */
function onLlmChanged() {
  if (debounceTimer) clearTimeout(debounceTimer);
  debounceTimer = setTimeout(async () => {
    const ok = await setMycuteLlms(localLlms.value);
    if (!ok) console.error("Failed to save LLM settings to backend.");
    else mainStore.setLlms(localLlms.value.map((l) => ({ ...l })));
  }, 500);
}

/** LLM エンドポイントを1件追加する。 */
function addLlm() {
  localLlms.value.push({
    name: DEFAULT_LLM_NAME,
    base_url: DEFAULT_LLM_BASE_URL,
    api_key: DEFAULT_LLM_API_KEY,
    model: DEFAULT_LLM_MODEL,
  });
  onLlmChanged();
}

/** LLM エンドポイントを1件削除する。 */
function removeLlm(idx: number) {
  localLlms.value.splice(idx, 1);
  onLlmChanged();
}
</script>
