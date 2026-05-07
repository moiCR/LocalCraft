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
  <div class="relative flex h-full w-full select-none flex-col items-center justify-center overflow-hidden p-8">
    <h1 class="mb-14 text-6xl font-black text-[#253126] transition-colors duration-300 dark:text-white">
      LocalCraft
    </h1>

    <div class="flex flex-wrap gap-4 items-center justify-center max-w-4xl">
      <Button 
        @click="navigateTo('/servers')"
        class="group flex h-52! w-52! flex-col items-center justify-center gap-5 rounded-[28px]! border-2! border-[#c7d6c8]! bg-[#fbfdf8]! text-[#253126] shadow-[0_8px_0_#b9c7b8] transition-all duration-300 hover:-translate-y-1 hover:border-brand/70 hover:bg-[#eef8f1]! active:translate-y-1 active:shadow-[0_2px_0_#b9c7b8] dark:border-[#26382d]! dark:bg-[#151815]! dark:text-white dark:shadow-[0_8px_0_#060806] dark:hover:bg-[#18231c]! dark:active:shadow-[0_2px_0_#060806]"
        tooltip="Manage Servers"
        tooltip-position="bottom"
      >
        <Server :size="40" class="text-brand transition-colors duration-300" />
        <span class="text-lg font-black uppercase">Servers</span>
      </Button>

      <Button 
        @click="navigateTo('/java')"
        class="group flex h-52! w-52! flex-col items-center justify-center gap-5 rounded-[28px]! border-2! border-[#d9ceb8]! bg-[#fffaf0]! text-[#2f2b21] shadow-[0_8px_0_#cdbf9f] transition-all duration-300 hover:-translate-y-1 hover:border-amber-400/70 hover:bg-[#fff4d7]! active:translate-y-1 active:shadow-[0_2px_0_#cdbf9f] dark:border-[#26382d]! dark:bg-[#151815]! dark:text-white dark:shadow-[0_8px_0_#060806] dark:hover:bg-[#211d14]! dark:active:shadow-[0_2px_0_#060806]"
        tooltip="Java Environments"
        tooltip-position="bottom"
      >
        <Coffee :size="40" class="text-[#f5a623] transition-colors duration-300" />
        <span class="text-lg font-black uppercase">Java</span>
      </Button>

      <Button 
        @click="navigateTo('/about')"
        class="group flex h-52! w-52! flex-col items-center justify-center gap-5 rounded-[28px]! border-2! border-[#c3d3e6]! bg-[#f4f9ff]! text-[#213043] shadow-[0_8px_0_#b4c4d8] transition-all duration-300 hover:-translate-y-1 hover:border-blue-400/70 hover:bg-[#e9f4ff]! active:translate-y-1 active:shadow-[0_2px_0_#b4c4d8] dark:border-[#26382d]! dark:bg-[#151815]! dark:text-white dark:shadow-[0_8px_0_#060806] dark:hover:bg-[#141b24]! dark:active:shadow-[0_2px_0_#060806]"
        tooltip="About LocalCraft"
        tooltip-position="bottom"
      >
        <BookOpen :size="40" class="text-[#6aa7ff] transition-colors duration-300" />
        <span class="text-lg font-black uppercase">About</span>
      </Button>
    </div>

    <div class="absolute bottom-5 left-1/2 flex -translate-x-1/2 items-center gap-6 rounded-2xl border-2 border-[#c7d6c8] bg-[#fbfdf8]/85 px-6 py-2.5 text-[#253126] shadow-[0_4px_0_#b9c7b8] backdrop-blur-sm transition-colors duration-300 dark:border-[#26382d] dark:bg-[#151815]/85 dark:text-white dark:shadow-[0_4px_0_#060806]">
      <div class="flex items-center gap-2.5">
        <div class="relative flex h-2 w-2">
          <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-brand opacity-75"></span>
          <span class="relative inline-flex h-2 w-2 rounded-full bg-brand"></span>
        </div>
        <span class="text-[10px] font-black uppercase tracking-[0.2em] text-brand/80">System Ready</span>
      </div>
      
      <div class="h-3 w-px bg-[#253126]/15 dark:bg-white/10"></div>
      
      <div class="flex items-center gap-5 text-[10px] font-black uppercase tracking-[0.2em] text-[#647264] dark:text-white/35">
        <span>{{ serverCount }} Servers</span>
        <span>{{ javaCount }} Environments</span>
      </div>
    </div>
  </div>
</template>
