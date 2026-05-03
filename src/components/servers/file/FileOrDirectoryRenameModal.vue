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
        <div class="flex h-9 w-9 items-center justify-center rounded-xl border border-brand/20 bg-brand/10">
          <Pencil :size="14" class="text-brand" />
        </div>
        <div>
          <p class="text-sm font-black leading-none text-white">Rename</p>
          <p class="text-white/40 text-xs mt-1 truncate max-w-[180px]">{{ file?.name }}</p>
        </div>
      </div>

      <!-- Input -->
      <input
        ref="inputRef"
        v-model="value"
        class="w-full rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm text-white outline-none transition-all duration-200 placeholder-white/25 focus:border-brand/50 focus:bg-white/10 focus:ring-4 focus:ring-brand/10"
        placeholder="New name..."
        @keyup.enter="confirm"
        @keyup.escape="close"
      />

      <!-- Actions -->
      <div class="flex justify-end gap-2">
        <button
          class="rounded-xl border border-white/10 bg-white/5 px-3 py-1.5 text-sm font-bold text-white/40 transition-colors duration-150 hover:bg-white/10 hover:text-white/70"
          @click="close"
        >
          Cancel
        </button>
        <button
          class="rounded-xl border-2 border-brand/70 bg-brand px-4 py-1.5 text-sm font-black text-black shadow-[0_3px_0_#060806] transition-all duration-150 hover:bg-brand/90 active:translate-y-0.5 active:shadow-[0_1px_0_#060806] disabled:pointer-events-none disabled:opacity-40"
          :disabled="!value.trim() || value.trim() === file?.name"
          @click="confirm"
        >
          Rename
        </button>
      </div>
    </div>
  </Modal>
</template>
