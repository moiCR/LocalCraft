<script setup lang="ts">
import { computed } from 'vue';
import { Minus, Square, X } from '@lucide/vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useRoute } from 'vue-router';

const appWindow = getCurrentWindow();

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();
const route = useRoute();

const subtitle = computed(() => {
    switch (route.path) {
        case '/': return 'Home';
        case '/servers': return 'Servers';
        case '/java': return 'Java';
        case '/about': return 'About';
        default: return '';
    }
});
</script>

<template>
    <div 
        data-tauri-drag-region 
        class="h-10 w-full select-none flex justify-between rounded-t-lg items-center bg-[#141414] border-b border-[#242424] shrink-0 z-50"
    >
        <div data-tauri-drag-region class="pl-4 gap-2 text-xs text-gray-500 font-bold tracking-widest flex items-center h-full w-full">
            <section class="flex items-center">
                <span>LocalCraft</span>
                <Transition name="title-fade" mode="out-in">
                    <span :key="route.path" v-if="subtitle" class="ml-2 opacity-60"> - {{ subtitle }}</span>
                </Transition>
            </section>
        </div>

        <div class="flex h-full">
            <button 
                @click="minimize" 
                class="px-4 h-full hover:bg-white/10 transition-colors rounded-3xl text-gray-400 hover:text-white" 
                title="Minimize"
            >
                <Minus :size="16" />
            </button>
            
            <button 
                @click="toggleMaximize" 
                class="px-4 h-full hover:bg-white/10 transition-colors rounded-3xl text-gray-400 hover:text-white" 
                title="Maximize"
            >
                <Square :size="14" />
            </button>
            
            <button 
                @click="close" 
                class="px-4 h-full hover:bg-red-500 transition-colors rounded-3xl text-gray-400 hover:text-white" 
                title="Close"
            >
                <X :size="16" />
            </button>
        </div>
    </div>
</template>

<style scoped>
.title-fade-enter-active,
.title-fade-leave-active {
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.title-fade-enter-from {
    opacity: 0;
    transform: translateY(4px);
    filter: blur(3px);
}

.title-fade-leave-to {
    opacity: 0;
    transform: translateY(-4px);
    filter: blur(3px);
}
</style>