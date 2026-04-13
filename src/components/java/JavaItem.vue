<script setup lang="ts">
import { ref } from "vue";
import { Coffee, FolderOpen, Trash2 } from "@lucide/vue";
import { invoke } from "@tauri-apps/api/core";
import gsap from "gsap";

const props = defineProps<{ version: string }>();
const emit = defineEmits<{ (e: "deleted"): void }>();

const itemRef = ref<HTMLElement | null>(null);
const deleteBgRef = ref<HTMLElement | null>(null);
const trashIconRef = ref<HTMLElement | null>(null);
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

const doDelete = async () => {
    try {
        await invoke("delete_java", { version: props.version });
        emit("deleted");
    } catch (e) {
        console.error("Failed to delete Java", e);
    }
};

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
    if (e.currentTarget)
        (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);

    if (currentX < THRESHOLD) {
        // Fly-out then delete
        if (itemRef.value) {
            gsap.to(itemRef.value, {
                x: -360,
                opacity: 0,
                scale: 0.92,
                duration: 0.32,
                ease: "power2.in",
                onComplete: () => void doDelete(),
            });
        } else {
            doDelete();
        }
        if (deleteBgRef.value) {
            gsap.to(deleteBgRef.value, {
                opacity: 0,
                duration: 0.3,
                delay: 0.12,
            });
        }
        return;
    }

    // Snap back
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

const onFolderOpen = (e: MouseEvent) => {
    e.stopPropagation();
    if (isDragging.value) return;
    invoke("open_java_folder").catch((err) =>
        console.error("Failed to open Java folder", err),
    );
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
            class="relative z-10 bg-white/2 dark:bg-white/3 hover:bg-white/5 border border-white/5 transition-colors duration-300 rounded-2xl p-4 flex flex-row items-center gap-4 touch-none select-none cursor-grab active:cursor-grabbing"
        >
            <!-- Icon -->
            <div
                class="relative flex items-center justify-center w-12 h-12 rounded-xl transition-all duration-500 shadow-sm border border-white/5 bg-brand/10 text-brand"
            >
                <Coffee :size="20" />
                <div
                    class="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-black bg-brand transition-colors duration-300"
                />
            </div>

            <!-- Text -->
            <div class="flex flex-col flex-1 min-w-0">
                <h3
                    class="text-base font-semibold text-white/90 group-hover/item:text-white transition-colors truncate tracking-tight"
                >
                    Java {{ version }}
                </h3>
                <span class="text-xs text-white/30 mt-0.5">Temurin JRE</span>
            </div>

            <!-- Folder open button -->
            <button
                @click="onFolderOpen"
                class="flex items-center justify-center w-7 h-7 rounded-lg text-white/30 hover:text-white/70 hover:bg-white/8 transition-all duration-200 cursor-pointer"
                title="Open Java folder"
            >
                <FolderOpen :size="14" />
            </button>
        </div>
    </div>
</template>

<style scoped>
h3 {
    -webkit-font-smoothing: antialiased;
}
</style>
