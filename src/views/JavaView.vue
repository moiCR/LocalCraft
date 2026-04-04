<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Coffee, CheckCircle, Download, Loader2, Trash2 } from '@lucide/vue';
import Button from '../components/ui/Button.vue';

interface JavaEnv {
    version: string;
    description: string;
    isInstalled: boolean;
    isInstalling: boolean;
}

const installedVersions = ref<string[]>([]);
const progress = ref({ message: '', percentage: 0 });
const isAnyInstalling = ref(false);

const javaEnvs = ref<JavaEnv[]>([
    { version: '21', description: 'Recomendado para Minecraft 1.20.5+', isInstalled: false, isInstalling: false },
    { version: '17', description: 'Recomendado para Minecraft 1.18 - 1.20.4', isInstalled: false, isInstalling: false },
    { version: '16', description: 'Recomendado para Minecraft 1.17', isInstalled: false, isInstalling: false },
    { version: '11', description: 'Recomendado para Minecraft 1.12 - 1.16', isInstalled: false, isInstalling: false },
    { version: '8',  description: 'Recomendado para versiones clásicas (< 1.12)', isInstalled: false, isInstalling: false },
]);

const loadInstalled = async () => {
    try {
        installedVersions.value = await invoke('get_installed_java');
        javaEnvs.value.forEach(env => {
            env.isInstalled = installedVersions.value.includes(env.version);
        });
    } catch (e) {
        console.error("Error cargando Java:", e);
    }
};

let unlistenProgress: any;

onMounted(async () => {
    await loadInstalled();
    
    unlistenProgress = await listen('creation-progress', (event: any) => {
        const payload = event.payload;
        if (payload.process.toLowerCase().includes('java')) {
            progress.value.message = payload.process;
            if (payload.percentage !== null) {
                progress.value.percentage = payload.percentage;
            }
        }
    });
});

onUnmounted(() => {
    if (unlistenProgress) unlistenProgress();
});

const installJava = async (env: JavaEnv) => {
    console.log("Instalar Java", env.version);
};

</script>

<template>

</template>
