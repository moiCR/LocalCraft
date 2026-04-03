<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue';
import { Loader2, Plus, ServerIcon } from '@lucide/vue';
import Modal from '../ui/Modal.vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

interface ProgressPayload {
    process: string;
    percentage: number | null;
}

const props = defineProps<{
    isOpen: boolean;
    anchorEl?: HTMLElement | null;
}>();

const emit = defineEmits<{
    (e: 'close'): void;
}>();

const ramOptions = [
    { label: "512 MB",  value: "512"   },
    { label: "1 GB",    value: "1024"  },
    { label: "2 GB",    value: "2048"  },
    { label: "4 GB",    value: "4096"  },
    { label: "8 GB",    value: "8192"  },
    { label: "16 GB",   value: "16384" },
    { label: "32 GB",   value: "32768" },
];

const softwareOptions = [
    { label: "Vanilla", value: "vanilla" },
    { label: "Paper",   value: "paper"   },
    { label: "Fabric",  value: "fabric"  },
    { label: "Forge",   value: "forge"   },
];


const name          = ref('');
const software      = ref('paper');
const version       = ref('');
const ram           = ref('2048');
const javaVersion   = ref('');
const error         = ref('');
const isLoading     = ref(false);
const isCreating    = ref(false);
const progressMsg   = ref('Initializing...');
const progressPct   = ref(0);

let unlisten: UnlistenFn | null = null;

onUnmounted(() => {
    if (unlisten) unlisten();
});

const versionOptions     = ref<{ label: string; value: string }[]>([]);
const javaVersionOptions = ref<{ label: string; value: string }[]>([]);

const canCreate = () => name.value && version.value && software.value && ram.value && javaVersion.value;

const handleClose = () => {
    name.value        = '';
    software.value    = 'paper';
    version.value     = '';
    ram.value         = '2048';
    javaVersion.value = '';
    error.value       = '';
    isCreating.value  = false;
    progressPct.value = 0;
    emit('close');
};

const fetchJavaVersions = async () => {
    try {
        const res  = await fetch('https://api.adoptium.net/v3/info/available_releases');
        const data = await res.json();
        const releases: number[] = data.available_releases || [];
        javaVersionOptions.value = [...releases].reverse().map(v => ({ label: `Java ${v}`, value: v.toString() }));
        javaVersion.value = data.most_recent_lts?.toString() ?? (releases.length ? releases[releases.length - 1].toString() : '21');
    } catch {
        javaVersionOptions.value = [{ label: 'Java 21', value: '21' }];
        javaVersion.value = '21';
    }
};

const fetchVersions = async () => {
    isLoading.value      = true;
    version.value        = '';
    versionOptions.value = [];

    try {
        let options: { label: string; value: string }[] = [];

        if (software.value === 'paper') {
            const data = await (await fetch('https://api.papermc.io/v2/projects/paper')).json();
            options = [...data.versions].filter((v: string) => !v.includes('-')).reverse().map((v: string) => ({ label: v, value: v }));
        } else if (software.value === 'vanilla') {
            const data = await (await fetch('https://launchermeta.mojang.com/mc/game/version_manifest.json')).json();
            options = data.versions.filter((v: any) => v.type === 'release').map((v: any) => ({ label: v.id, value: v.id }));
        } else if (software.value === 'fabric') {
            const data = await (await fetch('https://meta.fabricmc.net/v2/versions/game')).json();
            options = data.filter((v: any) => v.stable).map((v: any) => ({ label: v.version, value: v.version }));
        } else if (software.value === 'forge') {
            options = [
                { label: '1.20.4', value: '1.20.4' },
                { label: '1.19.4', value: '1.19.4' },
                { label: '1.18.2', value: '1.18.2' },
                { label: '1.16.5', value: '1.16.5' },
                { label: '1.12.2', value: '1.12.2' },
                { label: '1.8.9',  value: '1.8.9'  },
            ];
        }

        versionOptions.value = options;
    } catch {
        versionOptions.value = [{ label: 'Error fetching versions', value: '' }];
    } finally {
        isLoading.value = false;
    }
};

watch(() => props.isOpen, (open) => {
    if (!open) return;
    fetchJavaVersions();
    fetchVersions();
});

watch(software, () => {
    if (props.isOpen) fetchVersions();
});

const handleCreate = async () => {
    if (!canCreate()) {
        error.value = 'All fields are required';
        return;
    }
    error.value = '';
    isCreating.value = true;
    progressMsg.value = 'Contacting backend...';
    progressPct.value = 0;

    const unlistenFn = await listen<ProgressPayload>('creation-progress', (event) => {
        progressMsg.value = event.payload.process;
        if (event.payload.percentage !== null) {
            progressPct.value = event.payload.percentage;
        }
    });
    unlisten = unlistenFn;

    try {
        await invoke('create_server', {
            name: name.value,
            version: version.value,
            software: software.value,
            javaVersion: javaVersion.value,
            ram: ram.value,
        });
        
        handleClose();
    } catch (err: any) {
        error.value = err.toString();
        isCreating.value = false;
    } finally {
        if (unlisten) {
            unlisten();
            unlisten = null;
        }
    }
};
</script>

<template>
    <Modal 
        :is-open="isOpen" 
        :anchor-el="anchorEl" 
        layout-id="server-create-modal" 
        :width="320"
        :height="300"
        @close="handleClose"
    >
        <div class="flex flex-col p-4 gap-4">
            <!-- Header -->
            <h3 class="text-sm font-bold text-black dark:text-white tracking-tight transition-all">
                {{ isCreating ? 'Creating Server' : 'New Server' }}
            </h3>

            <!-- Form View -->
            <div v-if="!isCreating" class="flex flex-col gap-4">
                <!-- Name -->
                <div class="flex flex-col gap-1.5">
                    <label for="create-name" class="text-xs font-medium text-gray-500 dark:text-gray-400">Name</label>
                    <input
                        id="create-name"
                        v-model="name"
                        type="text"
                        placeholder="My Awesome Server"
                        class="border border-black/5 bg-black/2 text-black text-sm dark:border-white/5 dark:bg-white/2 dark:text-white rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-black/20 dark:focus:ring-white/20 transition-all placeholder:text-gray-400"
                    />
                </div>

                <!-- Software & Version -->
                <div class="grid grid-cols-2 gap-3">
                    <div class="flex flex-col gap-1.5">
                        <label class="text-xs font-medium text-gray-500 dark:text-gray-400">Software</label>
                        <select
                            v-model="software"
                            class="border border-black/5 bg-black/2 text-black text-sm dark:border-white/5 dark:bg-white/2 dark:text-white rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-black/20 dark:focus:ring-white/20 transition-all appearance-none cursor-pointer"
                        >
                            <option v-for="opt in softwareOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
                        </select>
                    </div>
                    <div class="flex flex-col gap-1.5">
                        <label class="text-xs font-medium text-gray-500 dark:text-gray-400">Version</label>
                        <select
                            v-model="version"
                            :disabled="isLoading || versionOptions.length === 0"
                            class="border border-black/5 bg-black/2 text-black text-sm dark:border-white/5 dark:bg-white/2 dark:text-white rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-black/20 dark:focus:ring-white/20 transition-all appearance-none cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <option value="" disabled>{{ isLoading ? 'Loading...' : 'Version' }}</option>
                            <option v-for="opt in versionOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
                        </select>
                    </div>
                </div>

                <div class="grid grid-cols-2 gap-3">
                    <div class="flex flex-col gap-1.5">
                        <label class="text-xs font-medium text-gray-500 dark:text-gray-400">Java</label>
                        <select
                            v-model="javaVersion"
                            :disabled="javaVersionOptions.length === 0"
                            class="border border-black/5 bg-black/2 text-black text-sm dark:border-white/5 dark:bg-white/2 dark:text-white rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-black/20 dark:focus:ring-white/20 transition-all appearance-none cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <option value="" disabled>{{ javaVersionOptions.length === 0 ? 'Loading...' : 'Java' }}</option>
                            <option v-for="opt in javaVersionOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
                        </select>
                    </div>
                    <div class="flex flex-col gap-1.5">
                        <label class="text-xs font-medium text-gray-500 dark:text-gray-400">RAM</label>
                        <select
                            v-model="ram"
                            class="border border-black/5 bg-black/2 text-black text-sm dark:border-white/5 dark:bg-white/2 dark:text-white rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-black/20 dark:focus:ring-white/20 transition-all appearance-none cursor-pointer"
                        >
                            <option v-for="opt in ramOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
                        </select>
                    </div>
                </div>

                <p v-if="error" class="text-xs text-red-500 font-medium">{{ error }}</p>

                <div class="flex gap-2 pt-1">
                    <button
                        type="button"
                        :disabled="isLoading"
                        @click="handleClose"
                        class="flex-1 text-xs font-medium py-2 px-3 rounded-lg text-gray-500 hover:text-black dark:hover:text-white hover:bg-black/5 dark:hover:bg-white/5 transition-colors disabled:opacity-40 cursor-pointer"
                    >
                        Cancel
                    </button>
                    <button
                        type="button"
                        :disabled="isLoading || !canCreate()"
                        @click="handleCreate"
                        class="flex-1 flex items-center justify-center gap-1.5 text-xs font-bold py-2 px-3 rounded-lg bg-black text-white dark:bg-white dark:text-black hover:opacity-80 transition-opacity disabled:opacity-30 disabled:cursor-not-allowed cursor-pointer"
                    >
                        <Loader2 v-if="isLoading" :size="12" class="animate-spin" />
                        <Plus v-else :size="12" />
                        Create
                    </button>
                </div>
            </div>

            <!-- Progress View -->
            <div v-else class="flex flex-col py-2 transition-all">
                <div class="flex flex-col items-center justify-center py-6 gap-6">
                    <div class="relative flex items-center justify-center">
                        <div class="absolute inset-0 bg-black/5 dark:bg-white/5 rounded-full scale-150 animate-pulse"></div>
                        <div class="relative p-4 bg-black/5 dark:bg-white/5 rounded-2xl">
                            <ServerIcon :size="32" class="text-black dark:text-white opacity-40" />
                        </div>
                    </div>

                    <div class="w-full flex flex-col gap-3">
                        <div class="flex flex-col gap-1.5">
                            <div class="flex justify-between items-center text-[10px] font-bold uppercase tracking-wider text-gray-400">
                                <span>{{ progressMsg }}</span>
                                <span class="font-mono">{{ Math.round(progressPct) }}%</span>
                            </div>
                            <div class="w-full bg-black/5 dark:bg-white/5 rounded-full h-2 overflow-hidden border border-black/5 dark:border-white/5">
                                <div 
                                    class="bg-black dark:bg-white h-full transition-all duration-500 ease-out"
                                    :style="{ width: `${progressPct}%` }"
                                ></div>
                            </div>
                        </div>
                    </div>
                </div>
                
                <p v-if="error" class="text-xs text-red-500 font-medium text-center mt-2">{{ error }}</p>
                <p v-else class="text-[10px] text-center text-gray-500 animate-pulse">This might take a few minutes...</p>

                <div v-if="error" class="mt-4 pt-2 border-t border-black/5 dark:border-white/5">
                    <button
                        @click="isCreating = false"
                        class="w-full text-xs font-bold py-2 rounded-lg bg-black/10 dark:bg-white/10 hover:bg-black/20 dark:hover:bg-white/20 transition-all"
                    >
                        Try Again
                    </button>
                </div>
            </div>

        </div>
    </Modal>
</template>