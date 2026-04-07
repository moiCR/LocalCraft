<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useRouter } from 'vue-router';
import { ChevronRight, ServerIcon, Trash2 } from '@lucide/vue';
import { invoke } from '@tauri-apps/api/core';
import gsap from 'gsap';
import ServerDeleteModal from './ServerDeleteModal.vue';

interface Server {
    id: string;
    name: string;
    version: string;
    software: string;
    ram: string;
    java_version: string;
}

const props = defineProps<{
    server: Server;
}>();

const emit = defineEmits<{
    (e: 'deleted'): void;
}>();

const router = useRouter();

const isRunning = ref(false);
const itemRef = ref<HTMLElement | null>(null);
const deleteBgRef = ref<HTMLElement | null>(null);
const showDeleteModal = ref(false);
const isDragging = ref(false);

let startX = 0;
let currentX = 0;
let isPointerDown = false;

const checkActivity = async () => {
    try {
        isRunning.value = await invoke<boolean>('is_server_running', { id: props.server.id });
    } catch {
        isRunning.value = false;
    }
};

onMounted(() => {
    checkActivity();
});

const onPointerDown = (e: PointerEvent) => {
    isPointerDown = true;
    startX = e.clientX;
    currentX = 0;
    (e.currentTarget as HTMLElement)?.setPointerCapture(e.pointerId);
};

const onPointerMove = (e: PointerEvent) => {
    if (!isPointerDown || !itemRef.value) return;
    const dx = e.clientX - startX;
    
    if (dx < 0 || currentX < 0) { // dragging left or recovering right
        currentX = dx > 0 ? 0 : dx;
        isDragging.value = true;
        const moveX = Math.max(currentX, -120);
        gsap.set(itemRef.value, { x: moveX });
        
        // Link opacity of delete background to drag distance
        if (deleteBgRef.value) {
            const opacity = Math.min(Math.abs(currentX) / 80, 1);
            gsap.set(deleteBgRef.value, { opacity: opacity });
        }
    }
};

const onPointerUp = (e: PointerEvent) => {
    if (!isPointerDown) return;
    isPointerDown = false;
    
    if (e.currentTarget) {
        (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    }

    if (currentX < -80) {
        showDeleteModal.value = true;
    }
    
    if (itemRef.value) {
        gsap.to(itemRef.value, { x: 0, duration: 0.4, ease: "back.out(1.5)" });
    }

    if (deleteBgRef.value) {
        gsap.to(deleteBgRef.value, { opacity: 0, duration: 0.3 });
    }
    
    setTimeout(() => {
        isDragging.value = false;
        currentX = 0;
    }, 50);
};

const onClick = (e: Event) => {
    if (isDragging.value || Math.abs(currentX) > 5) {
        e.preventDefault();
        e.stopPropagation();
        return;
    }
    router.push({ name: 'server-console', params: { id: props.server.id } });
};
</script>

<template>
    <div class="relative w-full rounded-2xl overflow-hidden group/item">
        
        <!-- Background Reveal content for Drag to delete -->
        <div 
            ref="deleteBgRef"
            class="absolute inset-y-0 right-0 w-24 bg-red-500/80 backdrop-blur-sm flex items-center justify-end pr-6 pointer-events-none opacity-0"
        >
            <Trash2 class="w-5 h-5 text-white/80" />
        </div>

        <!-- Main Server Item Content, Draggable -->
        <div 
            ref="itemRef"
            @pointerdown="onPointerDown"
            @pointermove="onPointerMove"
            @pointerup="onPointerUp"
            @pointercancel="onPointerUp"
            @click.capture="onClick"
            class="relative z-10 bg-white/2 dark:bg-white/3 hover:bg-white/5 border border-white/5 transition-all duration-300 rounded-2xl p-4 flex flex-row items-center gap-4 cursor-pointer touch-none"
        >
            <!-- Icon Section -->
            <div :class="[
                'relative flex items-center justify-center w-12 h-12 rounded-xl transition-all duration-500 shadow-sm border border-white/5',
                isRunning ? 'bg-brand/10 text-brand' : 'bg-white/5 text-white/20'
            ]">
                <ServerIcon :size="20" />
                <div :class="[
                    'absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-black transition-colors duration-300',
                    isRunning ? 'bg-brand' : 'bg-white/10'
                ]"></div>
            </div>

            <!-- Header Content -->
            <div class="flex flex-col flex-1 min-w-0">
                <div class="flex items-center gap-2">
                    <h3 class="text-base font-semibold text-white/90 group-hover/item:text-white transition-colors truncate tracking-tight">{{ server.name }}</h3>
                    <div class="px-1.5 py-0.5 border border-white/10 rounded text-[9px] text-white/30 uppercase tracking-[0.15em] font-bold">
                        {{ server.software }}
                    </div>
                </div>
                
                <div class="flex items-center gap-3 mt-1">
                    <div class="flex items-center gap-1.5 text-xs text-white/30">
                        <span class="w-1 h-1 rounded-full bg-white/20"></span>
                        <span>{{ server.version }}</span>
                    </div>
                    <div class="flex items-center gap-1.5 text-xs text-white/30 border-l border-white/10 pl-3">
                        <span>{{ server.ram }} RAM</span>
                    </div>
                    <div class="flex items-center gap-1.5 text-xs text-white/30 border-l border-white/10 pl-3">
                        <span>Java {{ server.java_version }}</span>
                    </div>
                </div>
            </div>

            <!-- Arrow Indicator -->
            <div class="opacity-0 group-hover/item:opacity-40 transition-opacity pr-2">
                <ChevronRight :size="18" class="text-white" />
            </div>
        </div>

        <ServerDeleteModal 
            :is-open="showDeleteModal"
            :server="server"
            :anchor-el="itemRef"
            @deleted="emit('deleted')"
            @close="showDeleteModal = false"
        />
    </div>
</template>

<style scoped>
/* Clearer font smoothing */
h3 {
    -webkit-font-smoothing: antialiased;
}
</style>
