<script setup lang="ts">
import { ref, watch } from 'vue';
import { X, Folder, FileText } from '@lucide/vue';
import Modal from '../../ui/Modal.vue';
import Button from '../../ui/Button.vue';

interface Props {
  isOpen: boolean;
  type: 'file' | 'folder';
  anchorEl?: HTMLElement | null;
  layoutId?: string;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'confirm', name: string): void;
}>();

const name = ref('');
const inputRef = ref<HTMLInputElement | null>(null);

watch(() => props.isOpen, (open) => {
  if (open) {
    name.value = '';
    setTimeout(() => inputRef.value?.focus(), 100);
  }
});

const handleConfirm = () => {
  if (!name.value.trim()) return;
  emit('confirm', name.value.trim());
};
</script>

<template>
  <Modal :is-open="isOpen" :anchor-el="anchorEl" :layout-id="layoutId" @close="emit('close')">
    <div class="relative flex flex-col p-6 w-full max-w-[340px] gap-6">
      <!-- Header -->
      <header class="relative flex flex-col items-center justify-center pt-2">
        <button 
          @click="emit('close')"
          class="absolute -top-1 -left-1 p-2 rounded-full hover:bg-white/5 transition-colors group"
        >
          <X :size="18" class="text-white/60 group-hover:text-white" />
        </button>
        
        <h2 class="text-xl font-black tracking-tight text-white">
          {{ type === 'file' ? 'New File' : 'New Folder' }}
        </h2>
      </header>

      <!-- Content -->
      <div class="flex flex-col gap-6">
        <div class="flex flex-col gap-2">
          <label class="text-xs font-semibold text-white/40 uppercase tracking-widest ml-1">
            {{ type === 'file' ? 'File Name' : 'Folder Name' }}
          </label>
          <div class="relative group">
            <div class="absolute left-4 top-1/2 -translate-y-1/2 text-white/40 group-focus-within:text-brand transition-colors">
              <FileText v-if="type === 'file'" :size="18" />
              <Folder v-else :size="18" />
            </div>
            <input
              ref="inputRef"
              v-model="name"
              :placeholder="type === 'file' ? 'Ej. server.properties' : 'Ej. mods, config...'"
              class="w-full bg-white/5 border border-white/10 rounded-xl pl-12 pr-4 py-3.5 text-sm text-white placeholder-white/20 outline-none focus:border-brand/50 focus:ring-4 focus:ring-brand/10 transition-all font-mono"
              @keyup.enter="handleConfirm"
            />
          </div>
        </div>

        <Button 
          class="w-full rounded-xl border-2 border-brand/70 bg-brand py-3.5 text-sm font-black text-black shadow-[0_4px_0_#060806] transition-all hover:bg-brand/90 active:translate-y-0.5 active:shadow-[0_1px_0_#060806]"
          @click="handleConfirm"
        >
          {{ type === 'file' ? 'Create File' : 'Create Folder' }}
        </Button>
      </div>
    </div>
  </Modal>
</template>
