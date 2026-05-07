<script setup lang="ts">
import { ref } from "vue";
import { Coffee, Trash2 } from "@lucide/vue";
import { invoke } from "@tauri-apps/api/core";

const props = defineProps<{ version: string }>();
const emit = defineEmits<{ (e: "deleted"): void }>();

const itemRef = ref<HTMLElement | null>(null);
const isDeleting = ref(false);

const doDelete = async () => {
    if (isDeleting.value) return;
    isDeleting.value = true;
    try {
        await invoke("delete_java", { version: props.version });
        emit("deleted");
    } catch (e) {
        console.error("Failed to delete Java", e);
    } finally {
        isDeleting.value = false;
    }
};
</script>

<template>
    <div class="relative w-full overflow-hidden rounded-[22px] group/item">
        <div
            ref="itemRef"
            class="relative z-10 flex select-none flex-row items-center gap-4 rounded-[22px] border-2 border-[#3b3426] bg-[#171512] p-4 shadow-[0_6px_0_#060806] transition-all duration-300 hover:-translate-y-0.5 hover:border-amber-400/35 hover:bg-[#211b13] active:translate-y-0.5 active:shadow-[0_2px_0_#060806]"
        >
            <!-- Icon -->
            <div
                class="relative flex h-12 w-12 items-center justify-center rounded-[18px] border-2 border-amber-400/30 bg-amber-400/10 text-amber-300 shadow-[inset_0_-4px_0_rgba(0,0,0,0.2)] transition-all duration-500"
            >
                <Coffee :size="20" />
                <div
                    class="absolute -top-0.5 -right-0.5 h-2.5 w-2.5 rounded-full border-2 border-[#060806] bg-amber-300 transition-colors duration-300"
                />
            </div>

            <!-- Text -->
            <div class="flex flex-col flex-1 min-w-0">
                <h3
                    class="truncate text-base font-black tracking-tight text-white/90 transition-colors group-hover/item:text-white"
                >
                    Java {{ version }}
                </h3>
                <span class="mt-0.5 text-xs font-semibold text-white/35">Temurin JRE</span>
            </div>

            <button
                type="button"
                class="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-red-400/15 bg-red-500/10 text-red-300/70 transition-all hover:border-red-400/35 hover:bg-red-500/15 hover:text-red-200 disabled:cursor-not-allowed disabled:opacity-45"
                title="Delete Java"
                :disabled="isDeleting"
                @click="doDelete"
            >
                <Trash2 :size="15" />
            </button>
        </div>
    </div>
</template>

<style scoped>
h3 {
    -webkit-font-smoothing: antialiased;
}
</style>
