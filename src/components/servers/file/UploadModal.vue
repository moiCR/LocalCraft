<script setup lang="ts">
import { ref } from 'vue';
import { X, Upload, Loader2, AlertCircle } from '@lucide/vue';
import Modal from '../../ui/Modal.vue';

interface Props {
  isOpen: boolean;
  uploading?: boolean;
  anchorEl?: HTMLElement | null;
  layoutId?: string;
}

const props = defineProps<Props>();
const emit = defineEmits<{
  (e: 'close'): void;
  (e: 'upload', files: FileList): void;
}>();

const isDragging = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

const handleDragEnter = (e: DragEvent) => {
  e.preventDefault();
  e.stopPropagation();
  isDragging.value = true;
};

const handleDragOver = (e: DragEvent) => {
  e.preventDefault();
  e.stopPropagation();
  isDragging.value = true;
};

const handleDragLeave = (e: DragEvent) => {
  e.preventDefault();
  e.stopPropagation();
  // Only stop dragging if we are leaving the main container, not its children
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
  if (
    e.clientX <= rect.left ||
    e.clientX >= rect.right ||
    e.clientY <= rect.top ||
    e.clientY >= rect.bottom
  ) {
    isDragging.value = false;
  }
};

const handleDrop = (e: DragEvent) => {
  e.preventDefault();
  e.stopPropagation();
  isDragging.value = false;
  
  if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
    emit('upload', e.dataTransfer.files);
  }
};

const handleFileSelect = (e: Event) => {
  const input = e.target as HTMLInputElement;
  if (input.files && input.files.length > 0) {
    emit('upload', input.files);
  }
};

const triggerFileInput = () => {
  fileInput.value?.click();
};
</script>

<template>
  <Modal :is-open="isOpen" :anchor-el="anchorEl" :layout-id="layoutId" @close="emit('close')">
    <div class="relative flex flex-col p-6 w-full max-w-[380px] gap-6">
      <!-- Header -->
      <header class="relative flex flex-col items-center justify-center pt-2">
        <button 
          @click="emit('close')"
          class="absolute -top-1 -left-1 p-2 rounded-full hover:bg-black/5 dark:hover:bg-white/5 transition-colors group"
        >
          <X :size="18" class="text-black/60 dark:text-white/60 group-hover:text-black dark:group-hover:text-white" />
        </button>
        
        <h2 class="text-xl font-bold tracking-tight text-black dark:text-white">
          Upload Files
        </h2>
      </header>

      <!-- Content -->
      <div class="flex flex-col gap-6">
        <!-- Drag & Drop Zone -->
        <div
          class="relative group cursor-pointer"
          @dragenter="handleDragEnter"
          @dragover="handleDragOver"
          @dragleave="handleDragLeave"
          @drop="handleDrop"
          @click="triggerFileInput"
        >
          <input
            ref="fileInput"
            type="file"
            multiple
            class="hidden"
            @change="handleFileSelect"
          />
          
          <div 
            :class="[
              'flex flex-col items-center justify-center gap-4 py-10 px-6 rounded-[32px] border-2 border-dashed transition-all duration-300',
              isDragging 
                ? 'border-brand bg-brand/5 scale-[0.98]' 
                : 'border-black/10 dark:border-white/10 hover:border-brand/40 bg-black/2 dark:bg-white/2 hover:bg-black/4 dark:hover:bg-white/4'
            ]"
          >
            <div 
              :class="[
                'w-12 h-12 rounded-full flex items-center justify-center transition-transform duration-500',
                isDragging ? 'bg-brand text-black scale-110' : 'bg-black/5 dark:bg-white/5 text-black/40 dark:text-white/40'
              ]"
            >
              <Upload :size="24" />
            </div>

            <div class="text-center">
              <p class="text-base font-semibold text-black dark:text-white">
                Drag and drop files here
              </p>
              <p class="text-xs text-black/40 dark:text-white/40 mt-1">
                or click to select manually
              </p>
            </div>
          </div>

          <!-- Overlay for uploading state -->
          <Transition
            enter-active-class="transition duration-300 ease-out"
            enter-from-class="opacity-0"
            enter-to-class="opacity-100"
            leave-active-class="transition duration-200 ease-in"
            leave-from-class="opacity-100"
            leave-to-class="opacity-0"
          >
            <div v-if="uploading" class="absolute inset-0 bg-white/80 dark:bg-[#1c1c1e]/80 backdrop-blur-sm rounded-[32px] flex flex-col items-center justify-center gap-4 z-10">
              <Loader2 :size="32" class="animate-spin text-brand" />
              <p class="text-base font-bold text-black dark:text-white animate-pulse">Subiendo...</p>
            </div>
          </Transition>
        </div>

        <!-- Support Text -->
        <div class="flex items-center gap-3 p-4 bg-black/5 dark:bg-white/5 rounded-2xl">
          <AlertCircle :size="18" class="text-brand shrink-0" />
          <p class="text-xs text-black/60 dark:text-white/60 leading-relaxed">
            You can upload multiple files at once. The progress will be updated in real time.
          </p>
        </div>
      </div>
    </div>
  </Modal>
</template>

<style scoped>
/* Minor animation tweak for the upload icon */
.group:hover .upload-icon {
  transform: translateY(-4px);
}
</style>
