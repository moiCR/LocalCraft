<script setup lang="ts">
import type { Component } from 'vue';
import { useRouter } from 'vue-router';

interface Props {
  title: string;
  icon: Component;
  to: string;
  active: boolean;
  isOpen: boolean;
}

const props = defineProps<Props>();
const router = useRouter();

const navigateTo = () => {
    router.push({ name: props.to });
};
</script>

<template>
    <div 
        @click="navigateTo"
        class="sidebar-item group relative flex items-center gap-3 w-full p-3 rounded-2xl cursor-pointer select-none transition-all duration-300"
        :class="{
            'justify-start': props.isOpen,
            'justify-center': !props.isOpen,
            'active-item': props.active,
            'hover:bg-white/10': !props.active
        }"
    >
        <!-- Indicador lateral para el estado activo -->
        <div 
            class="active-indicator absolute left-0 w-1 bg-brand rounded-full transition-all duration-500 ease-out shadow-[0_0_15px_rgba(0,255,136,0.5)]"
            :class="props.active ? 'h-6 opacity-100' : 'h-0 opacity-0'"
        ></div>

        <!-- Contenedor del Icono -->
        <div class="icon-container flex items-center justify-center transition-transform duration-300 group-hover:scale-110 group-active:scale-95">
            <component 
                :is="props.icon" 
                :size="22" 
                class="transition-colors duration-300"
                :class="props.active ? 'text-brand' : 'text-white/70 group-hover:text-white'"
            />
        </div>
        
        <!-- Texto con transición Slide & Fade -->
        <Transition name="slide-fade">
            <span 
                v-if="props.isOpen" 
                class="font-semibold text-sm tracking-wide whitespace-nowrap transition-colors duration-300"
                :class="props.active ? 'text-white' : 'text-white/60 group-hover:text-white'"
            >
                {{ props.title }}
            </span>
        </Transition>

        <!-- Brillo sutil de fondo en estado activo -->
        <div 
            v-if="props.active" 
            class="absolute inset-0 bg-brand/5 rounded-2xl -z-10 blur-md"
        ></div>
    </div>
</template>

<style scoped>
.sidebar-item {
    border: 1px solid transparent;
}

.sidebar-item:hover {
    border-color: rgba(255, 255, 255, 0.05);
    transform: translateX(4px);
}

.active-item {
    background: rgba(255, 255, 255, 0.03);
    border-color: rgba(255, 255, 255, 0.1);
    box-shadow: 0 4px 20px -5px rgba(0, 0, 0, 0.3);
}

/* Transición personalizada para el texto */
.slide-fade-enter-active {
  transition: all 0.4s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.slide-fade-leave-active {
  transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

.slide-fade-enter-from {
  opacity: 0;
  transform: translateX(-10px) scale(0.9);
}

.slide-fade-leave-to {
  opacity: 0;
  transform: translateX(-5px);
}
</style>