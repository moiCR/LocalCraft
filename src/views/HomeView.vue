<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { Server, Coffee, BookOpen } from '@lucide/vue';
import Button from '../components/ui/Button.vue';

const router = useRouter();
const serverCount = ref(0);
const javaCount = ref(0);

onMounted(async () => {
  try {
    const servers: any[] = await invoke('get_servers');
    serverCount.value = servers.length;
    
    const java: any[] = await invoke('get_installed_java');
    javaCount.value = java.length;
  } catch (e) {
    console.error("Error fetching counts:", e);
  }
});

const navigateTo = (path: string) => {
  router.push(path);
};
</script>

<template>
  <div class="h-full w-full flex flex-col items-center justify-center p-8 select-none relative overflow-hidden">
    <!-- Clean & Flat Title -->
    <h1 class="text-6xl font-black text-white mb-16 tracking-tighter">
      LocalCraft
    </h1>

    <!-- Minimalist Action Grid -->
    <div class="flex flex-wrap gap-4 items-center justify-center max-w-4xl">
      <Button 
        @click="navigateTo('/servers')"
        class="group flex flex-col items-center justify-center gap-6 w-56! h-56! bg-white/5! hover:bg-white/10! border border-white/5 rounded-4xl transition-all duration-300"
        tooltip="Manage Servers"
        tooltip-position="bottom"
      >
        <Server :size="40" class="text-white/20 group-hover:text-emerald-400 transition-colors duration-300" />
        <span class="text-lg font-bold text-white/40 group-hover:text-white transition-colors duration-300 uppercase tracking-wider">Servers</span>
      </Button>

      <Button 
        @click="navigateTo('/java')"
        class="group flex flex-col items-center justify-center gap-6 w-56! h-56! bg-white/5! hover:bg-white/10! border border-white/5 rounded-4xl transition-all duration-300"
        tooltip="Java Environments"
        tooltip-position="bottom"
      >
        <Coffee :size="40" class="text-white/20 group-hover:text-orange-400 transition-colors duration-300" />
        <span class="text-lg font-bold text-white/40 group-hover:text-white transition-colors duration-300 uppercase tracking-wider">Java</span>
      </Button>

      <Button 
        @click="navigateTo('/about')"
        class="group flex flex-col items-center justify-center gap-6 w-56! h-56! bg-white/5! hover:bg-white/10! border border-white/5 rounded-4xl transition-all duration-300"
        tooltip="About LocalCraft"
        tooltip-position="bottom"
      >
        <BookOpen :size="40" class="text-white/20 group-hover:text-blue-400 transition-colors duration-300" />
        <span class="text-lg font-bold text-white/40 group-hover:text-white transition-colors duration-300 uppercase tracking-wider">About</span>
      </Button>
    </div>

    <!-- Minimalist Footer Status Bar -->
    <div class="absolute bottom-8 left-1/2 -translate-x-1/2 flex items-center gap-6 px-6 py-2.5 bg-white/5 border border-white/5 rounded-2xl backdrop-blur-sm">
      <div class="flex items-center gap-2.5">
        <div class="relative flex h-2 w-2">
          <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
          <span class="relative inline-flex rounded-full h-2 w-2 bg-emerald-500"></span>
        </div>
        <span class="text-[10px] font-black text-emerald-500/80 uppercase tracking-[0.2em]">System Ready</span>
      </div>
      
      <div class="w-px h-3 bg-white/10"></div>
      
      <div class="flex items-center gap-5 text-[10px] font-black text-white/20 uppercase tracking-[0.2em]">
        <span>{{ serverCount }} Servers</span>
        <span>{{ javaCount }} Environments</span>
      </div>
    </div>
  </div>
</template>
