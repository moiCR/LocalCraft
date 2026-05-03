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
    <h1 class="mb-14 text-6xl font-black text-white">
      LocalCraft
    </h1>

    <div class="flex flex-wrap gap-4 items-center justify-center max-w-4xl">
      <Button 
        @click="navigateTo('/servers')"
        class="group flex h-52! w-52! flex-col items-center justify-center gap-5 rounded-[28px]! border-2! border-[#26382d]! bg-[#151815]! text-white shadow-[0_8px_0_#060806] transition-all duration-300 hover:-translate-y-1 hover:border-brand/70 hover:bg-[#18231c]! active:translate-y-1 active:shadow-[0_2px_0_#060806]"
        tooltip="Manage Servers"
        tooltip-position="bottom"
      >
        <Server :size="40" class="text-brand transition-colors duration-300" />
        <span class="text-lg font-black uppercase">Servers</span>
      </Button>

      <Button 
        @click="navigateTo('/java')"
        class="group flex h-52! w-52! flex-col items-center justify-center gap-5 rounded-[28px]! border-2! border-[#26382d]! bg-[#151815]! text-white shadow-[0_8px_0_#060806] transition-all duration-300 hover:-translate-y-1 hover:border-amber-400/70 hover:bg-[#211d14]! active:translate-y-1 active:shadow-[0_2px_0_#060806]"
        tooltip="Java Environments"
        tooltip-position="bottom"
      >
        <Coffee :size="40" class="text-[#f5a623] transition-colors duration-300" />
        <span class="text-lg font-black uppercase">Java</span>
      </Button>

      <Button 
        @click="navigateTo('/about')"
        class="group flex h-52! w-52! flex-col items-center justify-center gap-5 rounded-[28px]! border-2! border-[#26382d]! bg-[#151815]! text-white shadow-[0_8px_0_#060806] transition-all duration-300 hover:-translate-y-1 hover:border-blue-400/70 hover:bg-[#141b24]! active:translate-y-1 active:shadow-[0_2px_0_#060806]"
        tooltip="About LocalCraft"
        tooltip-position="bottom"
      >
        <BookOpen :size="40" class="text-[#6aa7ff] transition-colors duration-300" />
        <span class="text-lg font-black uppercase">About</span>
      </Button>
    </div>

    <div class="absolute bottom-5 left-1/2 flex -translate-x-1/2 items-center gap-6 rounded-2xl border-2 border-[#26382d] bg-[#151815]/85 px-6 py-2.5 text-white shadow-[0_4px_0_#060806] backdrop-blur-sm">
      <div class="flex items-center gap-2.5">
        <div class="relative flex h-2 w-2">
          <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-brand opacity-75"></span>
          <span class="relative inline-flex h-2 w-2 rounded-full bg-brand"></span>
        </div>
        <span class="text-[10px] font-black uppercase tracking-[0.2em] text-brand/80">System Ready</span>
      </div>
      
      <div class="h-3 w-px bg-white/10"></div>
      
      <div class="flex items-center gap-5 text-[10px] font-black uppercase tracking-[0.2em] text-white/35">
        <span>{{ serverCount }} Servers</span>
        <span>{{ javaCount }} Environments</span>
      </div>
    </div>
  </div>
</template>
