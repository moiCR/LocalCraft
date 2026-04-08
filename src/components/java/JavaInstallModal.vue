<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue';
import { Loader2, Download, Coffee } from '@lucide/vue';
import Modal from '../ui/Modal.vue';
import Select from '../ui/Select.vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

interface ProgressPayload {
    process: string;
    percentage: number | null;
}

const props = defineProps<{
    isOpen: boolean;
    anchorEl?: HTMLElement | null;
    installedVersions?: string[];
}>();

const emit = defineEmits<{
    (e: 'close'): void;
    (e: 'installed'): void;
}>();

const selectedVersion = ref('');
const versionOptions = ref<{ label: string; value: string }[]>([]);
const isLoading = ref(false);
const isInstalling = ref(false);
const progressMsg = ref('Initializing...');
const progressPct = ref(0);
const error = ref('');

let unlisten: UnlistenFn | null = null;

onUnmounted(() => { if (unlisten) unlisten(); });

const handleClose = () => {
    selectedVersion.value = '';
    versionOptions.value = [];
    isInstalling.value = false;
    progressPct.value = 0;
    error.value = '';
    emit('close');
};

const fetchVersions = async () => {
    isLoading.value = true;
    try {
        const res = await fetch('https://api.adoptium.net/v3/info/available_releases');
        const data = await res.json();
        const releases: number[] = data.available_releases || [];
        const installed = props.installedVersions ?? [];
        const options = [...releases]
            .filter(v => v !== 26 && !installed.includes(v.toString()))
            .reverse()
            .map(v => ({ label: `Java ${v}`, value: v.toString() }));
        versionOptions.value = options;
        selectedVersion.value = options[0]?.value ?? '';
    } catch {
        versionOptions.value = [{ label: 'Java 21', value: '21' }];
        selectedVersion.value = '21';
    } finally {
        isLoading.value = false;
    }
};

watch(() => props.isOpen, (open) => {
    if (open) fetchVersions();
});

const handleInstall = async () => {
    if (!selectedVersion.value) return;
    error.value = '';
    isInstalling.value = true;
    progressMsg.value = 'Starting download...';
    progressPct.value = 0;

    const unlistenFn = await listen<ProgressPayload>('creation-progress', (event) => {
        progressMsg.value = event.payload.process;
        if (event.payload.percentage !== null) {
            progressPct.value = event.payload.percentage;
        }
    });
    unlisten = unlistenFn;

    try {
        await invoke('install_java_cmd', { version: parseInt(selectedVersion.value) });
        emit('installed');
        handleClose();
    } catch (err: any) {
        error.value = err.toString();
        isInstalling.value = false;
    } finally {
        if (unlisten) { unlisten(); unlisten = null; }
    }
};
</script>

<template>
    <Modal
        :is-open="isOpen"
        :anchor-el="anchorEl"
        layout-id="java-install-modal"
        @close="handleClose"
    >
        <div class="flex flex-col p-6 gap-6">
            <h3 class="text-base font-bold text-black dark:text-white tracking-tight">
                {{ isInstalling ? 'Installing Java' : 'Install Java' }}
            </h3>

            <!-- Form -->
            <div v-if="!isInstalling" class="flex flex-col gap-5">
                <Select
                    v-model="selectedVersion"
                    :options="versionOptions"
                    :disabled="isLoading || versionOptions.length === 0"
                    label="Version"
                    :placeholder="isLoading ? 'Loading versions...' : 'Select version'"
                />

                <p v-if="error" class="text-xs text-red-500 font-medium">{{ error }}</p>

                <div class="flex gap-3 pt-2">
                    <button
                        type="button"
                        @click="handleClose"
                        class="flex-1 text-sm font-medium py-2.5 px-4 rounded-lg text-gray-500 hover:text-black dark:hover:text-white hover:bg-black/5 dark:hover:bg-white/5 transition-colors cursor-pointer"
                    >
                        Cancel
                    </button>
                    <button
                        type="button"
                        :disabled="isLoading || !selectedVersion"
                        @click="handleInstall"
                        class="flex-1 flex items-center justify-center gap-2 text-sm font-bold py-2.5 px-4 rounded-lg bg-black text-white dark:bg-white dark:text-black hover:opacity-80 transition-opacity disabled:opacity-30 disabled:cursor-not-allowed cursor-pointer"
                    >
                        <Loader2 v-if="isLoading" :size="12" class="animate-spin" />
                        <Download v-else :size="12" />
                        Install
                    </button>
                </div>
            </div>

            <!-- Progress -->
            <div v-else class="flex flex-col py-2">
                <div class="flex flex-col items-center justify-center py-6 gap-6">
                    <div class="relative flex items-center justify-center">
                        <div class="absolute inset-0 bg-black/5 dark:bg-white/5 rounded-full scale-150 animate-pulse"></div>
                        <div class="relative p-4 bg-black/5 dark:bg-white/5 rounded-2xl">
                            <Coffee :size="32" class="text-black dark:text-white opacity-40" />
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
                        @click="isInstalling = false"
                        class="w-full text-xs font-bold py-2 rounded-lg bg-black/10 dark:bg-white/10 hover:bg-black/20 dark:hover:bg-white/20 transition-all"
                    >
                        Try Again
                    </button>
                </div>
            </div>
        </div>
    </Modal>
</template>
