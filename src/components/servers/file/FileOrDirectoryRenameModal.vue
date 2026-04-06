<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { Pencil } from '@lucide/vue';
import Modal from '../../ui/Modal.vue';

interface FileInfo {
  name: string;
  is_dir: boolean;
  size: number;
}

interface Props {
  file: FileInfo | null;
  anchorElement?: HTMLElement | null;
  layoutId?: string;
}

const props = withDefaults(defineProps<Props>(), {
  anchorElement: null,
  layoutId: 'file-rename-modal',
});

const emit = defineEmits<{
  confirm: [newName: string];
  close: [];
}>();

const isOpen = ref(false);
const inputRef = ref<HTMLInputElement | null>(null);
const value = ref('');

watch(
  () => props.file,
  (file) => {
    if (file) {
      value.value = file.name;
      isOpen.value = true;
      nextTick(() => {
        inputRef.value?.focus();
        inputRef.value?.select();
      });
    } else {
      isOpen.value = false;
    }
  }
);

function confirm() {
  const trimmed = value.value.trim();
  if (!trimmed || trimmed === props.file?.name) {
    close();
    return;
  }
  emit('confirm', trimmed);
}

function close() {
  isOpen.value = false;
  emit('close');
}
</script>

<template>
  <Modal
    :is-open="isOpen"
    :anchor-el="anchorElement"
    :layout-id="layoutId"
    @close="close"
  >
    <div class="flex flex-col gap-5 p-5 w-72">
      <!-- Header -->
      <div class="flex items-center gap-2.5">
        <div class="flex items-center justify-center w-8 h-8 rounded-lg bg-white/5">
          <Pencil :size="14" class="text-white/60" />
        </div>
        <div>
          <p class="text-white text-sm font-semibold leading-none">Rename</p>
          <p class="text-white/40 text-xs mt-1 truncate max-w-[180px]">{{ file?.name }}</p>
        </div>
      </div>

      <!-- Input -->
      <input
        ref="inputRef"
        v-model="value"
        class="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-white text-sm outline-none
               focus:border-blue-500/50 focus:bg-white/8 transition-all duration-200 placeholder-white/25"
        placeholder="New name..."
        @keyup.enter="confirm"
        @keyup.escape="close"
      />

      <!-- Actions -->
      <div class="flex justify-end gap-2">
        <button
          class="text-sm text-white/40 hover:text-white/70 px-3 py-1.5 rounded-lg transition-colors duration-150"
          @click="close"
        >
          Cancel
        </button>
        <button
          class="text-sm font-semibold text-white bg-blue-600 hover:bg-blue-500
                 px-4 py-1.5 rounded-lg transition-colors duration-150 disabled:opacity-40 disabled:pointer-events-none"
          :disabled="!value.trim() || value.trim() === file?.name"
          @click="confirm"
        >
          Rename
        </button>
      </div>
    </div>
  </Modal>
</template>
