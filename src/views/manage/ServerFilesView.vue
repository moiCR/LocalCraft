<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useRoute } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import {
    ChevronLeftCircle,
    FolderPlus,
    RefreshCw,
    Home,
    Loader2,
    Folder,
    FilePlus,
    Upload,
} from "@lucide/vue";
import Button from "../../components/ui/Button.vue";
import FileItem from "../../components/servers/file/FileItem.vue";
import FileOrDirectoryRenameModal from "../../components/servers/file/FileOrDirectoryRenameModal.vue";
import FileOrDirectoryDeleteModal from "../../components/servers/file/FileOrDirectoryDeleteModal.vue";
import FileEditorModal from "../../components/servers/file/FileEditorModal.vue";
import CreateItemModal from "../../components/servers/file/CreateItemModal.vue";
import UploadModal from "../../components/servers/file/UploadModal.vue";

interface FileInfo {
    name: string;
    is_dir: boolean;
    size: number;
}

const route = useRoute();
const serverId = computed(() => route.params.id as string);

const pathStack = ref<string[]>([]);
const currentPath = computed(() => pathStack.value.join("/"));
const breadcrumbs = computed(() => {
    const crumbs = [{ label: "/", index: -1 }];
    pathStack.value.forEach((segment, i) => {
        crumbs.push({ label: segment, index: i });
    });
    return crumbs;
});

const files = ref<FileInfo[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function openFolder() {
    await invoke("open_folder", {
        serverId: serverId.value,
        path: currentPath.value || null,
    });
}

async function loadDir() {
    loading.value = true;
    error.value = null;
    try {
        const allFiles = await invoke<FileInfo[]>("read_dir", {
            serverId: serverId.value,
            subPath: currentPath.value || null,
        });
        files.value = allFiles.filter(
            (f) => f.name !== "server.json" && f.name !== "java_path.txt",
        );
    } catch (e) {
        error.value = String(e);
    } finally {
        loading.value = false;
    }
}

const editingFileName = ref<string | null>(null);
const editingFilePath = ref<string | null>(null);
const editorAnchorEl = ref<HTMLElement | null>(null);

function openItem(payload: { file: FileInfo; anchor?: HTMLElement }) {
    const { file, anchor } = payload;
    if (file.is_dir) {
        pathStack.value.push(file.name);
    } else {
        editorAnchorEl.value = anchor || null;
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
        await invoke("rename_file", {
            serverId: serverId.value,
            path: oldPath,
            newPath,
        });
        renameTarget.value = null;
        renameAnchorEl.value = null;
        await loadDir();
    } catch (e) {
        error.value = String(e);
    }
}

function closeRename() {
    renameTarget.value = null;
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
            await invoke("delete_dir", { serverId: serverId.value, path });
        } else {
            await invoke("delete_file", { serverId: serverId.value, path });
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
}

const showNewFolder = ref(false);
const showNewFile = ref(false);
const showUploadModal = ref(false);
const newFolderAnchorEl = ref<HTMLElement | null>(null);
const newFileAnchorEl = ref<HTMLElement | null>(null);
const uploadAnchorEl = ref<HTMLElement | null>(null);
const uploading = ref(false);

async function confirmNewFolder(name: string) {
    const path = currentPath.value ? `${currentPath.value}/${name}` : name;
    try {
        await invoke("create_dir", { serverId: serverId.value, path });
        showNewFolder.value = false;
        await loadDir();
    } catch (e) {
        error.value = String(e);
    }
}

async function confirmNewFile(name: string) {
    const path = currentPath.value ? `${currentPath.value}/${name}` : name;
    try {
        await invoke("save_file_binary", {
            serverId: serverId.value,
            path,
            data: [],
        });
        showNewFile.value = false;
        await loadDir();
    } catch (e) {
        error.value = String(e);
    }
}

async function handleUploadFiles(fileList: FileList) {
    uploading.value = true;
    error.value = null;

    try {
        const filesArray = Array.from(fileList);
        for (const file of filesArray) {
            const arrayBuffer = await file.arrayBuffer();
            const data = new Uint8Array(arrayBuffer);

            const path = currentPath.value
                ? `${currentPath.value}/${file.name}`
                : file.name;

            await invoke("save_file_binary", {
                serverId: serverId.value,
                path,
                data: Array.from(data),
            });
        }
        await loadDir();
        showUploadModal.value = false;
    } catch (e) {
        error.value = `Upload failed: ${String(e)}`;
    } finally {
        uploading.value = false;
    }
}

watch(currentPath, loadDir);
onMounted(loadDir);
</script>

<template>
    <div class="flex flex-col w-full h-full overflow-hidden">
        <!-- Toolbar -->
        <section
            class="flex flex-row px-4 py-3 border-b border-white/10 gap-2 items-center shrink-0"
        >
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
            <div
                class="flex items-center gap-1 flex-1 min-w-0 text-white/60 font-mono text-sm"
            >
                <template v-for="(crumb, i) in breadcrumbs" :key="i">
                    <span
                        class="cursor-pointer hover:text-white transition-colors duration-150 truncate"
                        :class="
                            i === breadcrumbs.length - 1 ? 'text-white' : ''
                        "
                        @click="
                            crumb.index === -1
                                ? goHome()
                                : goToIndex(crumb.index)
                        "
                        >{{ crumb.label }}</span
                    >
                    <span
                        v-if="i < breadcrumbs.length - 1"
                        class="text-white/20"
                        >/</span
                    >
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
                @click="
                    (e: MouseEvent) => {
                        newFileAnchorEl = e.currentTarget as HTMLElement;
                        showNewFile = true;
                    }
                "
                layout-id="new-file-modal"
            >
                <FilePlus :size="16" />
            </Button>

            <Button
                :tooltip="'New folder'"
                :tooltip-position="'bottom'"
                :class="'bg-transparent hover:bg-white/10 px-2! py-1.5!'"
                @click="
                    (e: MouseEvent) => {
                        newFolderAnchorEl = e.currentTarget as HTMLElement;
                        showNewFolder = true;
                    }
                "
                layout-id="new-folder-modal"
            >
                <FolderPlus :size="16" />
            </Button>

            <Button
                :tooltip="'Upload files'"
                :tooltip-position="'bottom'"
                :class="'bg-transparent hover:bg-white/10 px-2! py-1.5!'"
                @click="
                    (e: MouseEvent) => {
                        uploadAnchorEl = e.currentTarget as HTMLElement;
                        showUploadModal = true;
                    }
                "
                layout-id="upload-modal"
            >
                <Upload :size="16" />
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

        <!-- File list -->
        <section class="flex-1 overflow-hidden p-4">
            <div
                class="flex flex-col gap-1 w-full h-full border border-white/5 rounded-xl p-2 overflow-y-auto scroll-smooth"
            >
                <!-- Loading -->
                <div
                    v-if="loading"
                    class="flex items-center justify-center py-16 text-white/30 gap-2"
                >
                    <Loader2 :size="18" class="animate-spin" />
                    <span class="text-sm">Loading...</span>
                </div>

                <!-- Error -->
                <div
                    v-else-if="error"
                    class="flex items-center justify-center py-16"
                >
                    <p class="text-red-400/80 text-sm text-center max-w-xs">
                        {{ error }}
                    </p>
                </div>

                <!-- Empty -->
                <div
                    v-else-if="files.length === 0"
                    class="flex flex-col items-center justify-center py-16 text-white/25 gap-2"
                >
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
            :anchor-el="editorAnchorEl"
            layout-id="file-editor-modal"
            @close="
                editingFileName = null;
                editingFilePath = null;
            "
        />

        <!-- Create Item Modal -->
        <CreateItemModal
            :is-open="showNewFile"
            :anchor-el="newFileAnchorEl"
            layout-id="new-file-modal"
            type="file"
            @close="showNewFile = false"
            @confirm="confirmNewFile"
        />

        <CreateItemModal
            :is-open="showNewFolder"
            :anchor-el="newFolderAnchorEl"
            layout-id="new-folder-modal"
            type="folder"
            @close="showNewFolder = false"
            @confirm="confirmNewFolder"
        />

        <!-- Upload Modal -->
        <UploadModal
            :is-open="showUploadModal"
            :anchor-el="uploadAnchorEl"
            layout-id="upload-modal"
            :uploading="uploading"
            @close="showUploadModal = false"
            @upload="handleUploadFiles"
        />
    </div>
</template>
