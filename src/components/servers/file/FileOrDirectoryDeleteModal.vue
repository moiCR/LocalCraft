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
        <div class="flex items-center justify-center w-8 h-8 rounded-lg bg-red-500/10">
          <Trash2 :size="14" class="text-red-400" />
        </div>
        <div>
          <p class="text-white text-sm font-semibold leading-none">Delete</p>
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
          class="text-sm text-white/40 hover:text-white/70 px-3 py-1.5 rounded-lg transition-colors duration-150"
          @click="close"
        >
          Cancel
        </button>
        <button
          class="text-sm font-semibold text-white bg-red-600 hover:bg-red-500
                 px-4 py-1.5 rounded-lg transition-colors duration-150"
          @click="confirm"
        >
          Delete
        </button>
      </div>
    </div>
  </Modal>
</template>
