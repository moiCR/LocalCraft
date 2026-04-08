<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  Folder,
  FileText,
  FileCode2,
  FileJson,
  Image,
  FileArchive,
  File,
  ChevronRight,
  Pencil,
  Trash2,
} from '@lucide/vue';

interface FileInfo {
  name: string;
  is_dir: boolean;
  size: number;
}

interface Props {
  file: FileInfo;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  open: [payload: { file: FileInfo; anchor: HTMLElement }];
  rename: [payload: { file: FileInfo; anchor: HTMLElement }];
  delete: [payload: { file: FileInfo; anchor: HTMLElement }];
}>();

const renameBtn = ref<HTMLButtonElement | null>(null);
const deleteBtn = ref<HTMLButtonElement | null>(null);

const ext = computed(() =>
  props.file.is_dir ? '' : props.file.name.split('.').pop()?.toLowerCase() ?? ''
);

const FileIcon = computed(() => {
  if (props.file.is_dir) return Folder;
  switch (ext.value) {
    case 'json':      return FileJson;
    case 'js':
    case 'ts':
    case 'vue':
    case 'py':
    case 'java':
    case 'kt':
    case 'rs':
    case 'sh':
    case 'yml':
    case 'yaml':
    case 'toml':
    case 'xml':       return FileCode2;
    case 'txt':
    case 'md':
    case 'log':
    case 'cfg':
    case 'conf':
    case 'properties': return FileText;
    case 'png':
    case 'jpg':
    case 'jpeg':
    case 'gif':
    case 'svg':
    case 'webp':      return Image;
    case 'zip':
    case 'tar':
    case 'gz':
    case 'jar':
    case '7z':        return FileArchive;
    default:           return File;
  }
});

const iconColor = computed(() => {
  if (props.file.is_dir) return '#60a5fa'; // blue-400
  switch (ext.value) {
    case 'json':       return '#facc15'; // yellow-400
    case 'js':         return '#f59e0b';
    case 'ts':         return '#38bdf8';
    case 'vue':        return '#4ade80';
    case 'py':         return '#a78bfa';
    case 'java':
    case 'kt':         return '#f97316';
    case 'rs':         return '#fb923c';
    case 'yml':
    case 'yaml':
    case 'toml':       return '#e879f9';
    case 'txt':
    case 'md':
    case 'log':        return '#94a3b8';
    case 'cfg':
    case 'conf':
    case 'properties': return '#6ee7b7';
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'svg':
    case 'webp':       return '#f472b6';
    case 'jar':
    case 'zip':
    case 'tar':
    case 'gz':         return '#fbbf24';
    default:            return '#94a3b8';
  }
});

function formatSize(bytes: number): string {
  if (bytes === 0) return '—';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<template>
  <div
    class="file-item group flex items-center gap-3 px-3 py-2 rounded-lg cursor-pointer transition-all duration-150"
    :class="file.is_dir ? 'hover:bg-blue-500/10' : 'hover:bg-white/5'"
    @click="(e: MouseEvent) => emit('open', { file, anchor: e.currentTarget as HTMLElement })"
    layout-id="file-editor-modal"
  >
    <!-- Icon -->
    <component
      :is="FileIcon"
      :size="18"
      :color="iconColor"
      class="shrink-0 transition-transform duration-150 group-hover:scale-110"
    />

    <!-- Name -->
    <span
      class="flex-1 text-sm font-medium truncate"
      :class="file.is_dir ? 'text-blue-300' : 'text-white/80'"
    >
      {{ file.name }}
    </span>

    <!-- Size (files only) -->
    <span v-if="!file.is_dir" class="text-xs text-white/30 shrink-0 tabular-nums">
      {{ formatSize(file.size) }}
    </span>

    <!-- Actions (visible on hover) -->
    <div
      class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity duration-150 shrink-0"
      @click.stop
    >
      <button
        ref="renameBtn"
        class="p-1.5 rounded-md text-white/40 hover:text-white/90 hover:bg-white/10 transition-all duration-150"
        title="Rename"
        @click="emit('rename', { file, anchor: renameBtn! })"
        layout-id="file-rename-modal"
      >
        <Pencil :size="13" />
      </button>
      <button
        ref="deleteBtn"
        class="p-1.5 rounded-md text-white/40 hover:text-red-400 hover:bg-red-500/10 transition-all duration-150"
        title="Delete"
        @click="emit('delete', { file, anchor: deleteBtn! })"
        layout-id="file-delete-modal"
      >
        <Trash2 :size="13" />
      </button>
    </div>

    <!-- Dir chevron -->
    <ChevronRight
      v-if="file.is_dir"
      :size="14"
      class="shrink-0 text-white/20 group-hover:text-blue-400 transition-colors duration-150"
    />
  </div>
</template>
