<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { Folder } from '@lucide/vue';
import Button from '../components/ui/Button.vue';

const openJavaFolder = async () => {
    try {
        await invoke('open_java_folder');
    } catch (e) {
        console.error("Error abriendo carpeta Java:", e);
    }
}

</script>

<template>
    <Transition>
            <div class="flex flex-col items-center justify-center h-full w-full gap-3">
        <div class="flex flex-col items-center justify-center gap-3">
            <h1 class="text-4xl font-black text-white mb-2">Coming Soon...</h1>
            <Button 
                @click="openJavaFolder" 
                :tooltip="'Open Java Folder'" 
                :tooltip-position="'bottom'"
                :class="'bg-white/10 hover:bg-white/20 transition-colors duration-200'"
            >
                <Folder :size="24" />
            </Button>
        </div>
    </div>
    </Transition>
</template>
