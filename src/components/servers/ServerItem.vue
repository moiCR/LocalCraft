<script setup lang="ts">
import { ServerIcon } from '@lucide/vue';

interface Server {
    id: string;
    name: string;
    version: string;
    software: string;
    ram: string;
    java_version: string;
}

defineProps<{
    server: Server;
}>();
</script>

<template>
    <div class="bg-[#242424] hover:bg-[#2a2a2a] border border-white/5 transition-all duration-300 rounded-xl p-4 flex flex-row items-center gap-4 cursor-pointer group hover:translate-x-1 hover:shadow-lg">
        <!-- Icon -->
        <div class="relative flex items-center justify-center w-12 h-12 rounded-lg bg-emerald-500/10 text-emerald-500 group-hover:scale-110 transition-transform duration-300">
            <ServerIcon :size="24" />
            <div class="absolute -top-1 -right-1 w-3 h-3 rounded-full bg-emerald-500 border-2 border-[#242424] group-hover:animate-pulse"></div>
        </div>

        <!-- Content -->
        <div class="flex flex-col flex-1 min-w-0">
            <div class="flex items-center gap-3">
                <h3 class="text-lg font-bold text-white group-hover:text-emerald-400 transition-colors truncate">{{ server.name }}</h3>
                <span class="px-2 py-0.5 text-[10px] font-bold bg-white/5 text-white/40 rounded uppercase tracking-wider">{{ server.software }} {{ server.version }}</span>
            </div>
            
            <div class="flex items-center gap-4 text-xs text-gray-500 mt-1">
                <div class="flex items-center gap-1.5">
                    <div class="w-1.5 h-1.5 rounded-full bg-blue-500"></div>
                    <span>{{ server.ram }} RAM</span>
                </div>
                <div class="flex items-center gap-1.5">
                    <div class="w-1.5 h-1.5 rounded-full bg-orange-500"></div>
                    <span>Java {{ server.java_version }}</span>
                </div>
            </div>
        </div>

        <!-- Action/Status Arrow or something (Optional) -->
        <div class="opacity-0 group-hover:opacity-100 transition-opacity duration-300 pr-2">
             <div class="w-8 h-8 flex items-center justify-center rounded-full bg-white/5 text-white/50">
                <span class="text-xl">›</span>
             </div>
        </div>
    </div>
</template>
