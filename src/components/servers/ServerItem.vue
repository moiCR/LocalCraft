<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { ChevronRight, ServerIcon } from "@lucide/vue";
import { invoke } from "@tauri-apps/api/core";

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

const checkActivity = async () => {
    try {
        isRunning.value = await invoke<boolean>("is_server_running", {
            id: props.server.id,
        });
    } catch {
        isRunning.value = false;
    }
};

onMounted(checkActivity);

const onClick = (e: Event) => {
    router.push({ name: "server-console", params: { id: props.server.id } });
};
</script>

<template>
    <div class="relative w-full overflow-hidden rounded-[22px] group/item">
        <div
            ref="itemRef"
            @click.capture="onClick"
            class="relative z-10 flex cursor-pointer touch-none flex-row items-center gap-4 rounded-[22px] border-2 border-[#26382d] bg-[#151815] p-4 shadow-[0_6px_0_#060806] transition-all duration-300 hover:-translate-y-0.5 hover:border-brand/35 hover:bg-[#18211b] active:translate-y-0.5 active:shadow-[0_2px_0_#060806]"
            :layout-id="'delete-server-' + server.id"
        >
            <!-- Status icon -->
            <div
                :class="[
                    'relative flex items-center justify-center w-12 h-12 rounded-[18px] transition-all duration-500 border-2 shadow-[inset_0_-4px_0_rgba(0,0,0,0.2)]',
                    isRunning
                        ? 'border-brand/35 bg-brand/12 text-brand'
                        : 'border-white/8 bg-white/5 text-white/25',
                ]"
            >
                <ServerIcon :size="20" />
                <div
                    :class="[
                        'absolute -top-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-[#060806] transition-colors duration-300',
                        isRunning ? 'bg-brand' : 'bg-white/10',
                    ]"
                ></div>
            </div>

            <!-- Text content -->
            <div class="flex flex-col flex-1 min-w-0">
                <div class="flex items-center gap-2">
                    <h3
                        class="truncate text-base font-black tracking-tight text-white/92 transition-colors group-hover/item:text-white"
                    >
                        {{ server.name }}
                    </h3>
                    <div
                        class="rounded-md border border-white/10 bg-white/5 px-1.5 py-0.5 text-[9px] font-black uppercase tracking-[0.15em] text-white/40"
                    >
                        {{ server.software }}
                    </div>
                </div>

                <div class="mt-1 flex items-center gap-3">
                    <div
                        class="flex items-center gap-1.5 text-xs font-semibold text-white/35"
                    >
                        <span class="w-1 h-1 rounded-full bg-white/20"></span>
                        <span>{{ server.version }}</span>
                    </div>
                    <div
                        class="flex items-center gap-1.5 border-l border-white/10 pl-3 text-xs font-semibold text-white/35"
                    >
                        <span>{{ server.ram }} RAM</span>
                    </div>
                    <div
                        class="flex items-center gap-1.5 border-l border-white/10 pl-3 text-xs font-semibold text-white/35"
                    >
                        <span>Java {{ server.java_version }}</span>
                    </div>
                </div>
            </div>

            <!-- Chevron -->
            <div
                class="pr-2 opacity-0 transition-opacity group-hover/item:opacity-55"
            >
                <ChevronRight :size="18" class="text-white" />
            </div>

        </div>
    </div>
</template>

<style scoped>
h3 {
    -webkit-font-smoothing: antialiased;
}
</style>
