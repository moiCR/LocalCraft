<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { ChevronRight, ServerIcon, Trash2 } from "@lucide/vue";
import { invoke } from "@tauri-apps/api/core";
import gsap from "gsap";
import ServerDeleteModal from "./ServerDeleteModal.vue";

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
    (e: "deleted"): void;
}>();

const router = useRouter();

const isRunning = ref(false);
const itemRef = ref<HTMLElement | null>(null);
const deleteBgRef = ref<HTMLElement | null>(null);
const trashIconRef = ref<HTMLElement | null>(null);
const showDeleteModal = ref(false);
const isDragging = ref(false);
const isOverThreshold = ref(false);

let startX = 0;
let currentX = 0;
let isPointerDown = false;
let wasOverThreshold = false;

const THRESHOLD = -80;
const MAX_DRAG = -130;

/** Rubber-band resistance past the maximum drag distance */
const rubberBand = (x: number): number => {
    if (x >= MAX_DRAG) return x;
    return MAX_DRAG + (x - MAX_DRAG) * 0.25;
};

const checkActivity = async () => {
    try {
        isRunning.value = await invoke<boolean>("is_server_running", {
            id: props.server.id,
        });
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

    if (dx < 0 || currentX < 0) {
        currentX = dx > 0 ? 0 : dx;
        isDragging.value = true;

        const moveX = rubberBand(currentX);
        gsap.set(itemRef.value, { x: moveX, scale: 0.985 });

        if (deleteBgRef.value) {
            const opacity = Math.min(Math.abs(currentX) / 55, 1);
            gsap.set(deleteBgRef.value, { opacity });
        }

        // Threshold-crossing feedback
        const overThreshold = currentX < THRESHOLD;
        if (overThreshold !== wasOverThreshold) {
            wasOverThreshold = overThreshold;
            isOverThreshold.value = overThreshold;

            if (trashIconRef.value) {
                if (overThreshold) {
                    gsap.to(trashIconRef.value, {
                        scale: 1.3,
                        rotation: -15,
                        duration: 0.25,
                        ease: "back.out(3)",
                    });
                } else {
                    gsap.to(trashIconRef.value, {
                        scale: 1,
                        rotation: 0,
                        duration: 0.2,
                        ease: "power2.out",
                    });
                }
            }
        }
    }
};

const onPointerUp = (e: PointerEvent) => {
    if (!isPointerDown) return;
    isPointerDown = false;

    if (e.currentTarget) {
        (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    }

    if (currentX < THRESHOLD) {
        showDeleteModal.value = true;
    }

    if (itemRef.value) {
        gsap.to(itemRef.value, {
            x: 0,
            scale: 1,
            duration: 0.5,
            ease: "back.out(2)",
        });
    }

    if (deleteBgRef.value) {
        gsap.to(deleteBgRef.value, { opacity: 0, duration: 0.35 });
    }

    if (trashIconRef.value) {
        gsap.to(trashIconRef.value, {
            scale: 1,
            rotation: 0,
            duration: 0.3,
            ease: "power2.out",
        });
    }

    isOverThreshold.value = false;
    wasOverThreshold = false;

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
    router.push({ name: "server-console", params: { id: props.server.id } });
};
</script>

<template>
    <div class="relative w-full rounded-2xl overflow-hidden group/item">
        <!-- Delete background reveal -->
        <div
            ref="deleteBgRef"
            class="absolute inset-y-0 right-0 w-36 pointer-events-none opacity-0"
        >
            <!-- Subtle gradient — normal state -->
            <div
                class="absolute inset-0 bg-linear-to-r from-transparent to-red-500/65 transition-opacity duration-200"
                :class="isOverThreshold ? 'opacity-0' : 'opacity-100'"
            ></div>
            <!-- Vivid gradient — over-threshold state -->
            <div
                class="absolute inset-0 bg-linear-to-r from-red-600/25 to-red-500 transition-opacity duration-200"
                :class="isOverThreshold ? 'opacity-100' : 'opacity-0'"
            ></div>
            <!-- Icon + label -->
            <div
                class="absolute inset-y-0 right-0 w-28 flex flex-col items-center justify-center gap-1.5"
            >
                <div
                    ref="trashIconRef"
                    class="flex items-center justify-center"
                >
                    <Trash2 class="w-5 h-5 text-white drop-shadow-sm" />
                </div>
                <span
                    class="text-[9px] text-white/75 font-semibold tracking-widest uppercase select-none"
                >
                    Delete
                </span>
            </div>
        </div>

        <!-- Main draggable item -->
        <div
            ref="itemRef"
            @pointerdown="onPointerDown"
            @pointermove="onPointerMove"
            @pointerup="onPointerUp"
            @pointercancel="onPointerUp"
            @click.capture="onClick"
            class="relative z-10 bg-white/2 dark:bg-white/3 hover:bg-white/5 border border-white/5 transition-colors duration-300 rounded-2xl p-4 flex flex-row items-center gap-4 cursor-pointer touch-none"
            :layout-id="'delete-server-' + server.id"
        >
            <!-- Status icon -->
            <div
                :class="[
                    'relative flex items-center justify-center w-12 h-12 rounded-xl transition-all duration-500 shadow-sm border border-white/5',
                    isRunning
                        ? 'bg-brand/10 text-brand'
                        : 'bg-white/5 text-white/20',
                ]"
            >
                <ServerIcon :size="20" />
                <div
                    :class="[
                        'absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-black transition-colors duration-300',
                        isRunning ? 'bg-brand' : 'bg-white/10',
                    ]"
                ></div>
            </div>

            <!-- Text content -->
            <div class="flex flex-col flex-1 min-w-0">
                <div class="flex items-center gap-2">
                    <h3
                        class="text-base font-semibold text-white/90 group-hover/item:text-white transition-colors truncate tracking-tight"
                    >
                        {{ server.name }}
                    </h3>
                    <div
                        class="px-1.5 py-0.5 border border-white/10 rounded text-[9px] text-white/30 uppercase tracking-[0.15em] font-bold"
                    >
                        {{ server.software }}
                    </div>
                </div>

                <div class="flex items-center gap-3 mt-1">
                    <div
                        class="flex items-center gap-1.5 text-xs text-white/30"
                    >
                        <span class="w-1 h-1 rounded-full bg-white/20"></span>
                        <span>{{ server.version }}</span>
                    </div>
                    <div
                        class="flex items-center gap-1.5 text-xs text-white/30 border-l border-white/10 pl-3"
                    >
                        <span>{{ server.ram }} RAM</span>
                    </div>
                    <div
                        class="flex items-center gap-1.5 text-xs text-white/30 border-l border-white/10 pl-3"
                    >
                        <span>Java {{ server.java_version }}</span>
                    </div>
                </div>
            </div>

            <!-- Chevron -->
            <div
                class="opacity-0 group-hover/item:opacity-40 transition-opacity pr-2"
            >
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
h3 {
    -webkit-font-smoothing: antialiased;
}
</style>
