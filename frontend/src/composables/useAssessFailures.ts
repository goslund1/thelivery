import { ref } from 'vue'

export interface FailedAssess {
  liveryId: number
  liveryName: string
  cardName: string
}

const FAIL_KEY = 'imm-assess-failures'

function load(): FailedAssess[] {
  try { return JSON.parse(localStorage.getItem(FAIL_KEY) ?? '[]') } catch { return [] }
}

// Module-level singleton — shared across all consumers
const failedAssess = ref<FailedAssess[]>(load())

function persist() { localStorage.setItem(FAIL_KEY, JSON.stringify(failedAssess.value)) }

export function useAssessFailures() {
  function add(f: FailedAssess) {
    failedAssess.value = [...failedAssess.value.filter(x => x.liveryId !== f.liveryId), f]
    persist()
  }

  function remove(liveryId: number) {
    failedAssess.value = failedAssess.value.filter(x => x.liveryId !== liveryId)
    persist()
  }

  return { failedAssess, add, remove }
}
