import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'

export interface OrchestratorMessage {
  role: 'user' | 'assistant'
  text: string
}

export const useOrchestratorStore = defineStore('orchestrator', () => {
  const isVisible = ref(false)
  const isRecording = ref(false)
  const isProcessing = ref(false)
  const messages = ref<OrchestratorMessage[]>([])

  const startSession = async () => {
    try {
      await invoke('create_orchestrator_session')
      isVisible.value = true
      isRecording.value = true
    } catch (e) {
      console.error('Failed to create orchestrator session:', e)
    }
  }

  const sendText = async (text: string) => {
    if (!text.trim()) return
    messages.value.push({ role: 'user', text })
    isProcessing.value = true
    isRecording.value = false
    try {
      await invoke('orchestrator_process', {
        text,
        sessionId: '',
      })
    } catch (e) {
      console.error('orchestrator_process failed:', e)
      isProcessing.value = false
    }
  }

  const addAssistantMessage = (text: string) => {
    messages.value.push({ role: 'assistant', text })
    isProcessing.value = false
  }

  const closeOverlay = async () => {
    isVisible.value = false
    isRecording.value = false
    isProcessing.value = false
    messages.value = []
    try {
      await invoke('destroy_orchestrator_session')
    } catch (e) {
      console.error('Failed to destroy orchestrator session:', e)
    }
  }

  return {
    isVisible, isRecording, isProcessing, messages,
    startSession, sendText, addAssistantMessage, closeOverlay,
  }
})
