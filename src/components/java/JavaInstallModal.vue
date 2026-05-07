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
            <h3 class="text-base font-black text-white tracking-tight">
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
                        class="flex-1 cursor-pointer rounded-xl border border-white/10 bg-white/5 px-4 py-2.5 text-sm font-bold text-white/45 transition-colors hover:bg-white/10 hover:text-white"
                    >
                        Cancel
                    </button>
                    <button
                        type="button"
                        :disabled="isLoading || !selectedVersion"
                        @click="handleInstall"
                        class="flex-1 flex cursor-pointer items-center justify-center gap-2 rounded-xl border-2 border-amber-300/70 bg-amber-300 px-4 py-2.5 text-sm font-black text-black shadow-[0_4px_0_#060806] transition-all hover:bg-amber-200 active:translate-y-0.5 active:shadow-[0_1px_0_#060806] disabled:cursor-not-allowed disabled:opacity-30"
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
                        <div class="absolute inset-0 bg-white/5 rounded-full scale-150 animate-pulse"></div>
                        <div class="relative rounded-[20px] border border-amber-300/20 bg-amber-300/10 p-4">
                            <Coffee :size="32" class="text-amber-300 opacity-80" />
                        </div>
                    </div>

                    <div class="w-full flex flex-col gap-3">
                        <div class="flex flex-col gap-1.5">
                            <div class="flex justify-between items-center text-[10px] font-bold uppercase tracking-wider text-gray-400">
                                <span>{{ progressMsg }}</span>
                                <span class="font-mono">{{ Math.round(progressPct) }}%</span>
                            </div>
                            <div class="w-full bg-white/5 rounded-full h-2 overflow-hidden border border-white/5">
                                <div
                                    class="h-full bg-amber-300 transition-all duration-500 ease-out"
                                    :style="{ width: `${progressPct}%` }"
                                ></div>
                            </div>
                        </div>
                    </div>
                </div>

                <p v-if="error" class="text-xs text-red-500 font-medium text-center mt-2">{{ error }}</p>
                <p v-else class="text-[10px] text-center text-gray-500 animate-pulse">This might take a few minutes...</p>

                <div v-if="error" class="mt-4 pt-2 border-t border-white/5">
                    <button
                        @click="isInstalling = false"
                        class="w-full rounded-xl bg-white/10 py-2 text-xs font-bold transition-all hover:bg-white/20"
                    >
                        Try Again
                    </button>
                </div>
            </div>
        </div>
    </Modal>
</template>
