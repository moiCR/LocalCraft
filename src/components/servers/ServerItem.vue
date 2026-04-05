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
    
    setTimeout(() => {
        isDragging.value = false;
        currentX = 0;
    }, 50);
};

// Prevent clicks when dragging, and handle navigation
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
    <div class="relative w-full rounded-xl touch-none">
        
        <!-- Background Reveal content for Drag to delete -->
        <div class="absolute inset-y-0 right-0 w-24 bg-red-500 rounded-xl flex items-center justify-end pr-5 pointer-events-none">
            <Trash2 class="w-6 h-6 text-white" />
        </div>

        <!-- Main Server Item Content, Draggable -->
        <div 
            ref="itemRef"
            @pointerdown="onPointerDown"
            @pointermove="onPointerMove"
            @pointerup="onPointerUp"
            @pointercancel="onPointerUp"
            @click.capture="onClick"
            class="relative z-10 bg-[#242424] hover:bg-[#2a2a2a] border border-white/5 transition-[background-color,border-color,box-shadow] duration-300 rounded-xl p-4 flex flex-row items-center gap-4 cursor-pointer group hover:shadow-lg"
        >
            <!-- Icon -->
            <div :class="[
                'relative flex items-center justify-center w-12 h-12 rounded-lg group-hover:scale-110 transition-all duration-300',
                isRunning ? 'bg-emerald-500/10 text-emerald-500' : 'bg-red-500/10 text-red-500'
            ]">
                <ServerIcon :size="24" />
                <div :class="[
                    'absolute -top-1 -right-1 w-3 h-3 rounded-full border-2 border-[#242424] transition-colors duration-300',
                    isRunning ? 'bg-emerald-500 group-hover:animate-pulse' : 'bg-red-500'
                ]"></div>
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

            <div class="opacity-0 group-hover:opacity-100 transition-all duration-300 pr-2">
                 <div class="w-8 h-8 flex items-center justify-center rounded-full bg-white/5 text-white/50 group-hover:bg-white/10 group-hover:text-white transition-colors">
                    <ChevronRight :size="20" />
                 </div>
            </div>
        </div>

        <ServerDeleteModal 
            :is-open="showDeleteModal"
            :server="server"
            :anchor-el="itemRef"
            :on-close="() => showDeleteModal = false"
            @deleted="emit('deleted')"
            @close="showDeleteModal = false"
        />
    </div>
</template>
