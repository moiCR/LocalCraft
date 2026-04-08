<script setup lang="ts">
import { ref } from "vue";
import { Coffee, FolderOpen, Trash2 } from "@lucide/vue";
import { invoke } from "@tauri-apps/api/core";
import gsap from "gsap";

const props = defineProps<{ version: string }>();
const emit = defineEmits<{ (e: "deleted"): void }>();

const itemRef = ref<HTMLElement | null>(null);
const deleteBgRef = ref<HTMLElement | null>(null);
const isDragging = ref(false);

let startX = 0;
let currentX = 0;
let isPointerDown = false;

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
        const moveX = Math.max(currentX, -120);
        gsap.set(itemRef.value, { x: moveX });
        if (deleteBgRef.value) {
            const opacity = Math.min(Math.abs(currentX) / 80, 1);
            gsap.set(deleteBgRef.value, { opacity });
        }
    }
};

const onPointerUp = (e: PointerEvent) => {
    if (!isPointerDown) return;
    isPointerDown = false;
    if (e.currentTarget)
        (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    if (currentX < -80) {
        doDelete();
    }
    if (itemRef.value)
        gsap.to(itemRef.value, { x: 0, duration: 0.4, ease: "back.out(1.5)" });
    if (deleteBgRef.value)
        gsap.to(deleteBgRef.value, { opacity: 0, duration: 0.3 });
    setTimeout(() => {
        isDragging.value = false;
        currentX = 0;
    }, 50);
};

const onFolderOpen = (e: MouseEvent) => {
    e.stopPropagation();
    invoke("open_java_folder").catch((err) =>
        console.error("Failed to open Java folder", err),
    );
};
</script>

<template>
    <div class="relative w-full rounded-2xl overflow-hidden group/item">
        <!-- Delete background -->
        <div
            ref="deleteBgRef"
            class="absolute inset-y-0 right-0 w-24 bg-red-500/80 backdrop-blur-sm flex items-center justify-end pr-6 pointer-events-none opacity-0"
        >
            <Trash2 class="w-5 h-5 text-white/80" />
        </div>

        <!-- Draggable item -->
        <div
            ref="itemRef"
            @pointerdown="onPointerDown"
            @pointermove="onPointerMove"
            @pointerup="onPointerUp"
            @pointercancel="onPointerUp"
            class="relative z-10 bg-white/2 dark:bg-white/3 hover:bg-white/5 border border-white/5 transition-all duration-300 rounded-2xl p-4 flex flex-row items-center gap-4 touch-none select-none"
        >
            <!-- Icon -->
            <div
                class="relative flex items-center justify-center w-12 h-12 rounded-xl transition-all duration-500 shadow-sm border border-white/5 bg-brand/10 text-brand"
            >
                <Coffee :size="20" />
                <div
                    class="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-black bg-brand transition-colors duration-300"
                ></div>
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
                class="flex items-center justify-center w-7 h-7 rounded-lg text-white/30 hover:text-white/70 hover:bg-white/8 transition-all duration-200"
                title="Open Java folder"
            >
                <FolderOpen :size="14" />
            </button>
        </div>
    </div>
</template>
