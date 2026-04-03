<script setup lang="ts">
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
    <div class="bg-[#242424] hover:bg-[#2a2a2a] border border-white/5 transition-all duration-300 rounded-xl p-5 flex flex-col gap-3 cursor-pointer group hover:-translate-y-1 hover:shadow-lg">
        <div class="flex items-center justify-between">
            <h3 class="text-xl font-bold text-white group-hover:text-amber-400 transition-colors">{{ server.name }}</h3>
            <span class="px-3 py-1 text-xs font-semibold bg-white/10 text-white/80 rounded-full">{{ server.software }} {{ server.version }}</span>
        </div>
        
        <div class="flex items-center gap-5 text-sm text-gray-400 mt-2">
            <div class="flex items-center gap-2">
                <div class="min-w-3 min-h-3 w-3 h-3 rounded-full bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.5)]"></div>
                <span>{{ server.ram }} RAM</span>
            </div>
            <div class="flex items-center gap-2">
                <div class="min-w-3 min-h-3 w-3 h-3 rounded-full bg-orange-500 shadow-[0_0_8px_rgba(249,115,22,0.5)]"></div>
                <span>Java {{ server.java_version }}</span>
            </div>
        </div>
    </div>
</template>
