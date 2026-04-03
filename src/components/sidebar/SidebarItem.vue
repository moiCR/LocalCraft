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
    router.push(props.to);
};
</script>

<template>
    <div 
        @click="navigateTo"
        class="flex flex-row items-center gap-2 hover:bg-white/20 w-full p-2 rounded-xl transition-all duration-200 cursor-pointer"
        :class="{
            'justify-start': props.isOpen,
            'justify-center': !props.isOpen,
            'bg-white/20': props.active
        }"
    >
        <component :is="props.icon" :size="20" />
        
        <Transition name="fade">
            <span v-if="props.isOpen" class="font-medium whitespace-nowrap">
                {{ props.title }}
            </span>
        </Transition>
    </div>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>