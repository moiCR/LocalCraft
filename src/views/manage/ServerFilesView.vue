<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { useRoute } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { ChevronLeftCircle, FolderPlus, RefreshCw, Home, Loader2, Folder, FilePlus, Upload } from '@lucide/vue';
import Button from '../../components/ui/Button.vue';
import FileItem from '../../components/servers/file/FileItem.vue';
import FileOrDirectoryRenameModal from '../../components/servers/file/FileOrDirectoryRenameModal.vue';
import FileOrDirectoryDeleteModal from '../../components/servers/file/FileOrDirectoryDeleteModal.vue';
import FileEditorModal from '../../components/servers/file/FileEditorModal.vue';

interface FileInfo {
  name: string;
  is_dir: boolean;
  size: number;
}

const route = useRoute();
const serverId = computed(() => route.params.id as string);

const pathStack = ref<string[]>([]);
const currentPath = computed(() => pathStack.value.join('/'));
const breadcrumbs = computed(() => {
  const crumbs = [{ label: '/', index: -1 }];
  pathStack.value.forEach((segment, i) => {
    crumbs.push({ label: segment, index: i });
  });
  return crumbs;
});


const files = ref<FileInfo[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function openFolder(){
    await invoke('open_folder', {
        serverId: serverId.value,
        path: currentPath.value || null,
    });
}

async function loadDir() {
  loading.value = true;
  error.value = null;
  try {
    files.value = await invoke<FileInfo[]>('read_dir', {
      serverId: serverId.value,
      subPath: currentPath.value || null,
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

const editingFileName = ref<string | null>(null);
const editingFilePath = ref<string | null>(null);

function openItem(file: FileInfo) {
  if (file.is_dir) {
    pathStack.value.push(file.name);
  } else {
    editingFileName.value = file.name;
    editingFilePath.value = currentPath.value
      ? `${currentPath.value}/${file.name}`
      : file.name;
  }
}

function goBack() {
  if (pathStack.value.length > 0) pathStack.value.pop();
}

function goToIndex(index: number) {
  pathStack.value = pathStack.value.slice(0, index + 1);
}

function goHome() {
  pathStack.value = [];
}

const renameTarget = ref<FileInfo | null>(null);
const renameAnchorEl = ref<HTMLElement | null>(null);
const deleteTarget = ref<FileInfo | null>(null);
const deleteAnchorEl = ref<HTMLElement | null>(null);

function startRename(payload: { file: FileInfo; anchor: HTMLElement }) {
  renameAnchorEl.value = payload.anchor;
  renameTarget.value = payload.file;
}

async function confirmRename(newName: string) {
  if (!renameTarget.value) return;
  const oldPath = currentPath.value
    ? `${currentPath.value}/${renameTarget.value.name}`
    : renameTarget.value.name;
  const newPath = currentPath.value
    ? `${currentPath.value}/${newName}`
    : newName;
  try {
    await invoke('rename_file', { serverId: serverId.value, path: oldPath, newPath });
    renameTarget.value = null;
    renameAnchorEl.value = null;
    await loadDir();
  } catch (e) {
    error.value = String(e);
  }
}

function closeRename() {
  renameTarget.value = null;
  renameAnchorEl.value = null;
}

function startDelete(payload: { file: FileInfo; anchor: HTMLElement }) {
  deleteAnchorEl.value = payload.anchor;
  deleteTarget.value = payload.file;
}

async function confirmDelete() {
  if (!deleteTarget.value) return;
  const path = currentPath.value
    ? `${currentPath.value}/${deleteTarget.value.name}`
    : deleteTarget.value.name;
  try {
    if (deleteTarget.value.is_dir) {
      await invoke('delete_dir', { serverId: serverId.value, path });
    } else {
      await invoke('delete_file', { serverId: serverId.value, path });
    }
    deleteTarget.value = null;
    deleteAnchorEl.value = null;
    await loadDir();
  } catch (e) {
    error.value = String(e);
  }
}

function closeDelete() {
  deleteTarget.value = null;
  deleteAnchorEl.value = null;
}

const showNewFolder = ref(false);
const newFolderName = ref('');
const showNewFile = ref(false);
const newFileName = ref('');
const uploading = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

async function confirmNewFolder() {
  if (!newFolderName.value.trim()) return;
  const path = currentPath.value
    ? `${currentPath.value}/${newFolderName.value.trim()}`
    : newFolderName.value.trim();
  try {
    await invoke('create_dir', { serverId: serverId.value, path });
    showNewFolder.value = false;
    newFolderName.value = '';
    await loadDir();
  } catch (e) {
    error.value = String(e);
  }
}

async function confirmNewFile() {
  if (!newFileName.value.trim()) return;
  const path = currentPath.value
    ? `${currentPath.value}/${newFileName.value.trim()}`
    : newFileName.value.trim();
  try {
    // Create an empty file using the binary save command
    await invoke('save_file_binary', { 
      serverId: serverId.value, 
      path, 
      data: [] 
    });
    showNewFile.value = false;
    newFileName.value = '';
    await loadDir();
  } catch (e) {
    error.value = String(e);
  }
}

function triggerUpload() {
  fileInput.value?.click();
}

async function handleFileUpload(event: Event) {
  const input = event.target as HTMLInputElement;
  if (!input.files || input.files.length === 0) return;

  uploading.value = true;
  error.value = null;

  try {
    for (const file of Array.from(input.files)) {
      const arrayBuffer = await file.arrayBuffer();
      const data = new Uint8Array(arrayBuffer);
      
      const path = currentPath.value
        ? `${currentPath.value}/${file.name}`
        : file.name;

      await invoke('save_file_binary', {
        serverId: serverId.value,
        path,
        data: Array.from(data) // Tauri expects a normal array for Vec<u8>
      });
    }
    await loadDir();
  } catch (e) {
    error.value = `Upload failed: ${String(e)}`;
  } finally {
    uploading.value = false;
    input.value = ''; // Reset input
  }
}

watch(currentPath, loadDir);
onMounted(loadDir);
</script>

<template>
  <div class="flex flex-col w-full h-full overflow-hidden">

    <!-- Toolbar -->
    <section class="flex flex-row px-4 py-3 border-b border-white/10 gap-2 items-center shrink-0">
      <Button
        :tooltip="'Back'"
        :tooltip-position="'bottom'"
        :class="'bg-transparent hover:bg-white/10 px-2! py-1.5!'"
        :disabled="pathStack.length === 0"
        @click="goBack"
      >
        <ChevronLeftCircle :size="16" />
      </Button>

      <Button
        :tooltip="'Root'"
        :tooltip-position="'bottom'"
        :class="'bg-transparent hover:bg-white/10 px-2! py-1.5!'"
        :disabled="pathStack.length === 0"
        @click="goHome"
      >
        <Home :size="16" />
      </Button>

      <!-- Breadcrumbs -->
      <div class="flex items-center gap-1 flex-1 min-w-0 text-white/60 font-mono text-sm">
        <template v-for="(crumb, i) in breadcrumbs" :key="i">
          <span
            class="cursor-pointer hover:text-white transition-colors duration-150 truncate"
            :class="i === breadcrumbs.length - 1 ? 'text-white' : ''"
            @click="crumb.index === -1 ? goHome() : goToIndex(crumb.index)"
          >{{ crumb.label }}</span>
          <span v-if="i < breadcrumbs.length - 1" class="text-white/20">/</span>
        </template>
      </div>

      <Button
        :tooltip="'Open Folder'"
        :tooltip-position="'bottom'"
        :class="'bg-transparent hover:bg-white/10 px-2! py-1.5!'"
        @click="openFolder"
      >
        <Folder :size="16" />
      </Button>

      <Button
        :tooltip="'New file'"
        :tooltip-position="'bottom'"
        :class="'bg-transparent hover:bg-white/10 px-2! py-1.5!'"
        @click="showNewFile = !showNewFile; showNewFolder = false"
      >
        <FilePlus :size="16" />
      </Button>

      <Button
        :tooltip="'New folder'"
        :tooltip-position="'bottom'"
        :class="'bg-transparent hover:bg-white/10 px-2! py-1.5!'"
        @click="showNewFolder = !showNewFolder; showNewFile = false"
      >
        <FolderPlus :size="16" />
      </Button>

      <Button
        :tooltip="'Upload files'"
        :tooltip-position="'bottom'"
        :class="'bg-transparent hover:bg-white/10 px-2! py-1.5!'"
        :disabled="uploading"
        @click="triggerUpload"
      >
        <Upload v-if="!uploading" :size="16" />
        <Loader2 v-else :size="16" class="animate-spin" />
      </Button>

      <Button
        :tooltip="'Refresh'"
        :tooltip-position="'bottom'"
        :class="'bg-transparent hover:bg-white/10 px-2! py-1.5!'"
        @click="loadDir"
      >
        <RefreshCw :size="16" :class="loading ? 'animate-spin' : ''" />
      </Button>
    </section>

    <Transition
      enter-active-class="transition-all duration-200 ease-out"
      enter-from-class="opacity-0 -translate-y-2"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition-all duration-150 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 -translate-y-2"
    >
      <div
        v-if="showNewFolder"
        class="flex items-center gap-2 px-4 py-2 border-b border-white/10 bg-white/5 shrink-0"
      >
        <input
          v-model="newFolderName"
          autofocus
          placeholder="Folder name..."
          class="flex-1 bg-transparent text-sm text-white placeholder-white/30 outline-none"
          @keyup.enter="confirmNewFolder"
          @keyup.escape="showNewFolder = false; newFolderName = ''"
        />
        <button
          class="text-xs text-green-400 hover:text-green-300 font-semibold px-2 py-1 rounded transition-colors"
          @click="confirmNewFolder"
        >Create</button>
        <button
          class="text-xs text-white/40 hover:text-white/70 px-2 py-1 rounded transition-colors"
          @click="showNewFolder = false; newFolderName = ''"
        >Cancel</button>
      </div>
    </Transition>

    <!-- New File Input -->
    <Transition
      enter-active-class="transition-all duration-200 ease-out"
      enter-from-class="opacity-0 -translate-y-2"
      enter-to-class="opacity-100 translate-y-0"
      leave-active-class="transition-all duration-150 ease-in"
      leave-from-class="opacity-100 translate-y-0"
      leave-to-class="opacity-0 -translate-y-2"
    >
      <div
        v-if="showNewFile"
        class="flex items-center gap-2 px-4 py-2 border-b border-white/10 bg-white/5 shrink-0"
      >
        <input
          v-model="newFileName"
          autofocus
          placeholder="File name (e.g. server.properties)..."
          class="flex-1 bg-transparent text-sm text-white placeholder-white/30 outline-none"
          @keyup.enter="confirmNewFile"
          @keyup.escape="showNewFile = false; newFileName = ''"
        />
        <button
          class="text-xs text-green-400 hover:text-green-300 font-semibold px-2 py-1 rounded transition-colors"
          @click="confirmNewFile"
        >Create</button>
        <button
          class="text-xs text-white/40 hover:text-white/70 px-2 py-1 rounded transition-colors"
          @click="showNewFile = false; newFileName = ''"
        >Cancel</button>
      </div>
    </Transition>

    <input
      ref="fileInput"
      type="file"
      multiple
      class="hidden"
      @change="handleFileUpload"
    />

    <!-- File list -->
    <section class="flex-1 overflow-hidden p-4">
      <div class="flex flex-col gap-1 w-full h-full border border-white/5 rounded-xl p-2 overflow-y-auto scroll-smooth">

        <!-- Loading -->
        <div v-if="loading" class="flex items-center justify-center py-16 text-white/30 gap-2">
          <Loader2 :size="18" class="animate-spin" />
          <span class="text-sm">Loading...</span>
        </div>

        <!-- Error -->
        <div v-else-if="error" class="flex items-center justify-center py-16">
          <p class="text-red-400/80 text-sm text-center max-w-xs">{{ error }}</p>
        </div>

        <!-- Empty -->
        <div v-else-if="files.length === 0" class="flex flex-col items-center justify-center py-16 text-white/25 gap-2">
          <span class="text-3xl">📂</span>
          <span class="text-sm">This folder is empty</span>
        </div>

        <!-- Items -->
        <template v-else>
          <FileItem
            v-for="file in files"
            :key="file.name"
            :file="file"
            @open="openItem"
            @rename="startRename"
            @delete="startDelete"
          />

          <!-- Rename modal -->
          <FileOrDirectoryRenameModal
            :file="renameTarget"
            :anchor-element="renameAnchorEl"
            layout-id="file-rename-modal"
            @confirm="confirmRename"
            @close="closeRename"
          />
        </template>
      </div>
    </section>


    <!-- Delete modal -->
    <FileOrDirectoryDeleteModal
      :file="deleteTarget"
      :anchor-element="deleteAnchorEl"
      layout-id="file-delete-modal"
      @confirm="confirmDelete"
      @close="closeDelete"
    />

    <!-- Editor modal -->
    <FileEditorModal
      :file-name="editingFileName"
      :file-path="editingFilePath"
      :server-id="serverId"
      @close="editingFileName = null; editingFilePath = null"
    />

  </div>
</template>
