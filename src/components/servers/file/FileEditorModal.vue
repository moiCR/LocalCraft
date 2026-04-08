<script setup lang="ts">
import { ref, watch, nextTick, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { FileCode2, Save, X, Loader2 } from '@lucide/vue';
import Modal from '../../ui/Modal.vue';

interface Props {
  filePath: string | null;
  fileName: string | null;
  serverId: string;
  anchorEl?: HTMLElement | null;
  layoutId?: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  close: [];
}>();

const isOpen = ref(false);
const content = ref('');
const originalContent = ref('');
const loading = ref(false);
const saving = ref(false);
const error = ref<string | null>(null);
const textareaRef = ref<HTMLTextAreaElement | null>(null);

const hasChanges = computed(() => content.value !== originalContent.value);

const lineCount = computed(() => {
  const lines = content.value.split('\n').length;
  return Math.max(lines, 1);
});

const ext = computed(() =>
  props.fileName?.split('.').pop()?.toLowerCase() ?? ''
);

const languageLabel = computed(() => {
  switch (ext.value) {
    case 'json':       return 'JSON';
    case 'js':         return 'JavaScript';
    case 'ts':         return 'TypeScript';
    case 'vue':        return 'Vue';
    case 'py':         return 'Python';
    case 'java':       return 'Java';
    case 'kt':         return 'Kotlin';
    case 'rs':         return 'Rust';
    case 'sh':         return 'Shell';
    case 'yml':
    case 'yaml':       return 'YAML';
    case 'toml':       return 'TOML';
    case 'xml':        return 'XML';
    case 'html':       return 'HTML';
    case 'css':        return 'CSS';
    case 'md':         return 'Markdown';
    case 'txt':        return 'Text';
    case 'log':        return 'Log';
    case 'cfg':
    case 'conf':
    case 'properties': return 'Config';
    default:           return ext.value.toUpperCase() || 'Text';
  }
});

watch(
  () => props.filePath,
  async (path) => {
    if (path && props.fileName) {
      isOpen.value = true;
      await loadFile(path);
    } else {
      isOpen.value = false;
    }
  }
);

async function loadFile(path: string) {
  loading.value = true;
  error.value = null;
  try {
    const result = await invoke<string>('read_file', {
      serverId: props.serverId,
      path,
    });
    content.value = result;
    originalContent.value = result;
    await nextTick();
    textareaRef.value?.focus();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function save() {
  if (!props.filePath || !hasChanges.value) return;
  saving.value = true;
  try {
    await invoke('write_file', {
      serverId: props.serverId,
      path: props.filePath,
      content: content.value,
    });
    originalContent.value = content.value;
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

function close() {
  isOpen.value = false;
  emit('close');
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault();
    save();
  }
}

function handleTab(e: KeyboardEvent) {
  if (e.key === 'Tab') {
    e.preventDefault();
    const textarea = textareaRef.value;
    if (!textarea) return;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    content.value =
      content.value.substring(0, start) + '  ' + content.value.substring(end);
    nextTick(() => {
      textarea.selectionStart = textarea.selectionEnd = start + 2;
    });
  }
}

function syncScroll(e: Event) {
  const textarea = e.target as HTMLTextAreaElement;
  const lineNumbers = textarea.parentElement?.querySelector('.line-numbers') as HTMLElement | null;
  if (lineNumbers) {
    lineNumbers.scrollTop = textarea.scrollTop;
  }
}
</script>

<template>
  <Modal
    :is-open="isOpen"
    :anchor-el="anchorEl"
    :layout-id="layoutId"
    @close="close"
  >
    <div
      class="flex flex-col w-[min(720px,90vw)] h-[min(520px,80vh)]"
      @keydown="handleKeydown"
    >
      <!-- Header -->
      <div class="flex items-center gap-3 px-4 py-3 border-b border-white/10 shrink-0">
        <FileCode2 :size="16" class="text-white/50 shrink-0" />
        <span class="text-white text-sm font-semibold truncate flex-1">{{ fileName }}</span>

        <!-- Language badge -->
        <span class="text-[10px] uppercase tracking-wider text-white/30 bg-white/5 px-2 py-0.5 rounded-md font-medium shrink-0">
          {{ languageLabel }}
        </span>

        <!-- Unsaved dot -->
        <div
          v-if="hasChanges"
          class="w-2 h-2 rounded-full bg-amber-400 shrink-0 animate-pulse"
          title="Unsaved changes"
        />

        <!-- Save -->
        <button
          class="flex items-center gap-1.5 text-xs font-semibold px-3 py-1.5 rounded-lg transition-all duration-150 shrink-0"
          :class="hasChanges
            ? 'text-white bg-blue-600 hover:bg-blue-500'
            : 'text-white/30 bg-white/5 cursor-default'"
          :disabled="!hasChanges || saving"
          @click="save"
        >
          <Loader2 v-if="saving" :size="12" class="animate-spin" />
          <Save v-else :size="12" />
          Save
        </button>

        <!-- Close -->
        <button
          class="p-1.5 rounded-lg text-white/40 hover:text-white/80 hover:bg-white/10 transition-all duration-150 shrink-0"
          @click="close"
        >
          <X :size="14" />
        </button>
      </div>

      <!-- Editor body -->
      <div class="flex-1 min-h-0 relative">
        <!-- Loading -->
        <div v-if="loading" class="absolute inset-0 flex items-center justify-center text-white/30 gap-2">
          <Loader2 :size="18" class="animate-spin" />
          <span class="text-sm">Loading file...</span>
        </div>

        <!-- Error -->
        <div v-else-if="error" class="absolute inset-0 flex items-center justify-center p-6">
          <p class="text-red-400/80 text-sm text-center max-w-xs">{{ error }}</p>
        </div>

        <!-- Editor -->
        <div v-else class="flex h-full">
          <!-- Line numbers -->
          <div
            class="line-numbers shrink-0 w-12 bg-white/2 border-r border-white/5 overflow-hidden
                   select-none text-right pr-2 pt-3 pb-3 font-mono text-xs text-white/15 leading-[1.65]"
          >
            <div v-for="n in lineCount" :key="n">{{ n }}</div>
          </div>

          <!-- Textarea -->
          <textarea
            ref="textareaRef"
            v-model="content"
            spellcheck="false"
            class="flex-1 bg-transparent text-white/80 text-sm font-mono leading-[1.65] p-3 resize-none
                   outline-none overflow-y-auto placeholder-white/20 caret-blue-400"
            placeholder="Empty file..."
            @scroll="syncScroll"
            @keydown="handleTab"
          />
        </div>
      </div>

      <!-- Status bar -->
      <div class="flex items-center justify-between px-4 py-1.5 border-t border-white/5 text-[10px] text-white/25 shrink-0 font-mono">
        <span>{{ lineCount }} lines</span>
        <span>{{ content.length }} chars</span>
        <span v-if="hasChanges" class="text-amber-400/60">Modified</span>
        <span v-else class="text-green-400/40">Saved</span>
      </div>
    </div>
  </Modal>
</template>
