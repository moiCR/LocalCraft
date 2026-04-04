<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router';
import type { Component } from 'vue';

interface Props{
    to: RouteLocationRaw;
    title: string;
    icon?: Component;
    active?: boolean;
}

const props = defineProps<Props>();

</script>

<template>
    <RouterLink 
        :to="to" 
        class="group flex flex-row p-2 items-center gap-2 rounded-xl transition-all duration-300 ease-[cubic-bezier(0.34,1.56,0.64,1)]"
        :class="active ? 'bg-white/10' : 'bg-black/40 hover:bg-white/5 hover:scale-y-[0.95] hover:scale-x-[1.05]'"
    >
        <component :is="icon" :class="active ? 'text-brand' : 'text-white/70 group-hover:text-white'" :size="16" />
        <span class="font-semibold text-sm transition-colors duration-300" :class="active ? 'text-white' : 'text-white/40 group-hover:text-white'">{{ title }}</span>
        
        <div 
            v-if="active"
            class="w-1.5 h-1.5 rounded-full bg-brand shadow-[0_0_10px_rgba(0,255,136,0.8)] transition-all duration-500"
            :class="active ? 'scale-100 opacity-100' : 'scale-0 opacity-0'"
        ></div>
    </RouterLink>
</template>