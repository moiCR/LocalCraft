<script setup lang="ts">
import { Minus, Square, X } from '@lucide/vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useRoute } from 'vue-router';
import LogoIcon from '../assets/logo.svg?component';

const appWindow = getCurrentWindow();

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();
const route = useRoute();
</script>

<template>
    <div 
        data-tauri-drag-region 
        class="h-10 w-full select-none flex justify-between rounded-t-lg items-center bg-[#141414] border-b border-[#242424] shrink-0 z-50"
    >
        <div data-tauri-drag-region class="pl-4 gap-2 text-xs text-gray-500 font-bold tracking-widest flex items-center h-full w-full">
            <LogoIcon class="w-5 h-5"/>
            <section class="flex items-center">
                <span>LocalCraft</span>
                <span v-if="route.path === '/'" class="ml-2"> - Home</span>
                <span v-if="route.path === '/servers'" class="ml-2"> - Servers</span>
            </section>
        </div>

        <div class="flex h-full">
            <button 
                @click="minimize" 
                class="px-4 h-full hover:bg-white/10 transition-colors text-gray-400 hover:text-white" 
                title="Minimize"
            >
                <Minus :size="16" />
            </button>
            
            <button 
                @click="toggleMaximize" 
                class="px-4 h-full hover:bg-white/10 transition-colors text-gray-400 hover:text-white" 
                title="Maximize"
            >
                <Square :size="14" />
            </button>
            
            <button 
                @click="close" 
                class="px-4 h-full hover:bg-red-500 transition-colors text-gray-400 hover:text-white" 
                title="Close"
            >
                <X :size="16" />
            </button>
        </div>
    </div>
</template>