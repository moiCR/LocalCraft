<script setup lang="ts">
import { ref, watch } from 'vue';
import { Trash2 } from '@lucide/vue';
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
  layoutId: 'file-delete-modal',
});

const emit = defineEmits<{
  confirm: [];
  close: [];
}>();

const isOpen = ref(false);

watch(
  () => props.file,
  (file) => {
    isOpen.value = !!file;
  }
);

function confirm() {
  emit('confirm');
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
        <div class="flex h-9 w-9 items-center justify-center rounded-xl border border-red-400/20 bg-red-500/10">
          <Trash2 :size="14" class="text-red-400" />
        </div>
        <div>
          <p class="text-sm font-black leading-none text-white">Delete</p>
          <p class="text-white/40 text-xs mt-1 truncate max-w-[180px]">"{{ file?.name }}"</p>
        </div>
      </div>

      <!-- Warning -->
      <p class="text-white/50 text-sm leading-relaxed">
        This action cannot be undone.
        <template v-if="file?.is_dir">
          The folder and all its contents will be permanently removed.
        </template>
      </p>

      <!-- Actions -->
      <div class="flex justify-end gap-2">
        <button
          class="rounded-xl border border-white/10 bg-white/5 px-3 py-1.5 text-sm font-bold text-white/40 transition-colors duration-150 hover:bg-white/10 hover:text-white/70"
          @click="close"
        >
          Cancel
        </button>
        <button
          class="rounded-xl border-2 border-red-400/60 bg-red-600 px-4 py-1.5 text-sm font-black text-white shadow-[0_3px_0_#060806] transition-all duration-150 hover:bg-red-500 active:translate-y-0.5 active:shadow-[0_1px_0_#060806]"
          @click="confirm"
        >
          Delete
        </button>
      </div>
    </div>
  </Modal>
</template>
