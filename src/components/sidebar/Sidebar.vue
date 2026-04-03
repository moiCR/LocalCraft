<script setup lang="ts">
import { ChevronLeft } from '@lucide/vue';
import gsap from 'gsap';
import LogoIcon from '../../assets/logo.svg?component';

const props = defineProps({
    isOpen: {
        type: Boolean,
        default: true
    },
    toggleSidebar: {
        type: Function,
        default: () => {}
    }
});

const beforeEnter = (el: Element) => {
    gsap.set(el, { opacity: 0, x: -15 });
};

const enter = (el: Element, done: () => void) => {
    gsap.to(el, { 
        opacity: 1, 
        x: 0, 
        duration: 0.3, 
        ease: "power2.out", 
        onComplete: done 
    });
};

const leave = (el: Element, done: () => void) => {
    gsap.to(el, { 
        opacity: 0, 
        x: -15, 
        duration: 0.2, 
        ease: "power2.in", 
        onComplete: done 
    });
};
</script>

<template>
    <aside 
        class="flex flex-col h-screen bg-[#141414] rounded-r-xl transition-[width] duration-300 ease-in-out border-r border-[#242424]"
        :class="{ 'w-64': isOpen, 'w-20': !isOpen }"
    >
        <header 
            class="p-4 flex items-center h-[72px] border-b border-[#242424] overflow-hidden whitespace-nowrap transition-all duration-300" 
            :class="isOpen ? 'justify-start' : 'justify-center'"
        >
            <button 
                @click="toggleSidebar()"
                class="p-2 hover:bg-white/5 rounded-lg transition-colors group"
            >
                <ChevronLeft 
                    :size="20" 
                    class="transition-transform duration-300"
                    :class="{ 'rotate-180': !isOpen }"
                />
            </button>

            <div 
                class="flex items-center gap-3 min-w-max transition-all duration-300" 
                :class="{ 'ml-4': isOpen }"
            >
                <Transition 
                    @before-enter="beforeEnter"
                    @enter="enter"
                    @leave="leave"
                    :css="false"
                >
                    <LogoIcon class="w-10 h-10" v-show="isOpen"/>
                </Transition>
                
                <Transition 
                    @before-enter="beforeEnter"
                    @enter="enter"
                    @leave="leave"
                    :css="false"
                >
                    <span v-show="isOpen" class="font-bold text-lg tracking-tight">LocalCraft</span>
                </Transition>
            </div>
        </header>

        <nav class="flex flex-col flex-1 overflow-x-hidden p-4 space-y-2">
            <slot />
        </nav>
    </aside>
</template>