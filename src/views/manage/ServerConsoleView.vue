<script setup lang="ts">
import {
    PlayIcon,
    Rotate3D,
    SquareIcon,
    ChevronRight,
    GlobeIcon,
} from "@lucide/vue";
import Button from "../../components/ui/Button.vue";
import TunnelModal from "../../components/ui/TunnelModal.vue";
import { useRoute } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { ref, computed, onMounted, watch, nextTick, onUnmounted } from "vue";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

import {
    globalLogsCache,
    startGlobalListener,
} from "../../server/serverConsoleLogs";

const route = useRoute();
const serverID = route.params.id as string;

const logsContainerRef = ref<HTMLDivElement | null>(null);

const scrollToBottom = () => {
    if (logsContainerRef.value) {
        logsContainerRef.value.scrollTop = logsContainerRef.value.scrollHeight;
    }
};

const serverLogs = computed(() => globalLogsCache[serverID] || []);

const isActive = ref(false);
const isRestarting = ref(false);
const isStopping = ref(false);
let unlistenStopped: UnlistenFn | null = null;

const isTunnelModalOpen = ref(false);
const tunnelButtonRef = ref<HTMLElement | null>(null);

onMounted(async () => {
    startGlobalListener();
    isActive.value = await invoke("is_server_running", { id: serverID });

    await nextTick();
    scrollToBottom();

    unlistenStopped = await listen("server-stopped", async (event) => {
        if (event.payload === serverID) {
            isActive.value = false;
            isStopping.value = false;
            if (isRestarting.value) {
                isRestarting.value = false;
                await startServer();
            }
        }
    });
});

onUnmounted(() => {
    if (unlistenStopped) unlistenStopped();
});

watch(
    serverLogs,
    async () => {
        await nextTick();
        scrollToBottom();
    },
    { deep: true },
);

const startServer = async () => {
    if (isActive.value) return;
    try {
        globalLogsCache[serverID] = [];
        isActive.value = true;
        await invoke("start_server", { id: serverID });
        console.log("Server started");
    } catch (error) {
        console.error("Error starting server:", error);
        isActive.value = false;
    }
};

const stopServer = async () => {
    if (!isActive.value || isStopping.value) return;
    isStopping.value = true;
    try {
        await invoke("send_command", { id: serverID, command: "stop" });
    } catch (error) {
        console.error("Error stopping server:", error);
        isStopping.value = false;
    }
};

const restartServer = async () => {
    if (isStopping.value || isRestarting.value) return;

    if (!isActive.value) {
        await startServer();
    } else {
        isRestarting.value = true;
        await stopServer();
    }
};

const commandInput = ref("");

const sendCommand = async () => {
    try {
        await invoke("send_command", {
            id: serverID,
            command: commandInput.value,
        });
        commandInput.value = "";
    } catch (error) {
        console.error("Error sending command:", error);
    }
};
</script>

<template>
    <div class="flex h-full w-full flex-col gap-2 overflow-hidden">
        <section class="flex shrink-0 flex-row gap-2 border-b border-[#26382d] bg-[#151815]/70 p-4">
            <Button
                @click="startServer"
                :disabled="isActive || isStopping || isRestarting"
                :tooltip="'Start server'"
                :tooltip-position="'bottom'"
                :class="[
                    'border border-green-400/15 bg-green-500/10 hover:bg-green-500/15 transition-all',
                    isActive || isStopping || isRestarting
                        ? 'opacity-50 cursor-not-allowed'
                        : '',
                ]"
            >
                <PlayIcon :class="'text-green-500'" :size="16" />
            </Button>
            <Button
                @click="stopServer"
                :disabled="!isActive || isStopping || isRestarting"
                :tooltip="'Stop server'"
                :tooltip-position="'bottom'"
                :class="[
                    'border border-red-400/15 bg-red-500/10 hover:bg-red-500/15 transition-all',
                    !isActive || isStopping || isRestarting
                        ? 'opacity-50 cursor-not-allowed'
                        : '',
                ]"
            >
                <SquareIcon :class="'text-red-500'" :size="16" />
            </Button>
            <Button
                @click="restartServer"
                :disabled="!isActive || isStopping || isRestarting"
                :tooltip="'Restart server'"
                :tooltip-position="'bottom'"
                :class="[
                    'border border-yellow-400/15 bg-yellow-500/10 hover:bg-yellow-500/15 transition-all',
                    !isActive || isStopping || isRestarting
                        ? 'opacity-50 cursor-not-allowed'
                        : '',
                ]"
            >
                <Rotate3D :class="'text-yellow-500'" :size="16" />
            </Button>

            <div class="w-px h-6 bg-white/10 mx-2" />

            <Button
                @click="
                    (e: MouseEvent) => {
                        tunnelButtonRef = e.currentTarget as HTMLElement;
                        isTunnelModalOpen = true;
                    }
                "
                :tooltip="'Network Tunnel (Playit)'"
                :tooltip-position="'bottom'"
                class="border border-blue-400/15 bg-blue-500/10 hover:bg-blue-500/15 transition-all"
                layout-id="tunnel-modal"
            >
                <GlobeIcon :class="'text-blue-500'" :size="16" />
            </Button>
        </section>

        <TunnelModal
            :is-open="isTunnelModalOpen"
            :anchor-el="tunnelButtonRef"
            layout-id="tunnel-modal"
            :server-id="serverID"
            @close="isTunnelModalOpen = false"
        />

        <section
            class="flex h-full w-full flex-col gap-4 overflow-hidden p-4 scroll-smooth"
        >
            <div
                ref="logsContainerRef"
                class="custom-scrollbar flex flex-1 flex-col gap-1.5 overflow-y-auto rounded-[24px] border-2 border-[#223127] bg-[#0c0f0d] p-6 font-mono text-xs text-white/70 shadow-[inset_0_0_0_1px_rgba(255,255,255,0.02),0_6px_0_#060806]"
            >
                <div
                    v-if="serverLogs.length === 0"
                    class="text-white/30 italic"
                >
                    Waiting for server logs...
                </div>

                <div
                    v-for="(log, index) in serverLogs"
                    :key="index"
                    class="whitespace-pre-wrap break-all"
                    :class="{
                        'text-red-400':
                            log.includes('[ERROR]') ||
                            log.toLowerCase().includes('warn'),
                    }"
                >
                    {{ log }}
                </div>
            </div>

            <div class="relative group">
                <div
                    class="absolute left-4 top-1/2 -translate-y-1/2 font-mono text-sm text-white/20 transition-colors group-focus-within:text-brand/60"
                >
                    <ChevronRight :size="16" />
                </div>
                <input
                    type="text"
                    placeholder="Type a command..."
                    v-model="commandInput"
                    @keyup.enter="sendCommand"
                    class="w-full rounded-[20px] border-2 border-[#223127] bg-[#0c0f0d] py-4 pl-10 pr-4 font-mono text-sm text-white/90 outline-none shadow-[0_5px_0_#060806] transition-all focus:border-brand/35 focus:ring-4 focus:ring-brand/10 disabled:opacity-50"
                />
            </div>
        </section>
    </div>
</template>
