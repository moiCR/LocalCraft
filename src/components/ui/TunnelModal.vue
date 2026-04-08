<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import {
    GlobeIcon,
    CopyIcon,
    ExternalLinkIcon,
    CheckIcon,
    XIcon,
    ShieldCheckIcon,
    Loader2,
    AlertCircle,
} from "@lucide/vue";
import Modal from "./Modal.vue";
import Button from "./Button.vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";

interface Props {
    isOpen: boolean;
    anchorEl?: HTMLElement | null;
    layoutId?: string;
    serverId?: string;
}

const props = defineProps<Props>();
const emit = defineEmits<{
    (e: "close"): void;
}>();

const status = ref({
    active: false,
    address: null as string | null,
    claim_url: null as string | null,
});

const isLoading = ref(false);
const copied = ref(false);
const serverActive = ref(true);

const downloadProgress = ref({
    process: "",
    percentage: null as number | null,
});

let unlistenStatus: UnlistenFn | null = null;
let unlistenProgress: UnlistenFn | null = null;
let unlistenServerStopped: UnlistenFn | null = null;

const fetchStatus = async () => {
    try {
        const res = (await invoke("get_tunnel_status")) as any;
        status.value = res;
    } catch (e) {
        console.error("Failed to fetch tunnel status", e);
    }
};

const fetchServerStatus = async () => {
    if (!props.serverId) {
        serverActive.value = true;
        return;
    }
    try {
        serverActive.value = (await invoke("is_server_running", {
            id: props.serverId,
        })) as boolean;
    } catch {
        serverActive.value = false;
    }
};

onMounted(async () => {
    await fetchStatus();
    await fetchServerStatus();

    unlistenStatus = await listen("tunnel-status", (event: any) => {
        status.value = event.payload;
    });
    unlistenProgress = await listen("creation-progress", (event: any) => {
        downloadProgress.value = event.payload;
        // If we are downloading, we should be in a loading state
        if (event.payload.percentage !== 100) {
            isLoading.value = true;
        } else {
            // Once finished, we'll wait for the tunnel-status event to clear loading
            setTimeout(() => {
                downloadProgress.value = { process: "", percentage: null };
            }, 2000);
        }
    });

    if (props.serverId) {
        unlistenServerStopped = await listen("server-stopped", (event: any) => {
            if (event.payload === props.serverId) {
                serverActive.value = false;
            }
        });
    }
});

onUnmounted(() => {
    if (unlistenStatus) unlistenStatus();
    if (unlistenProgress) unlistenProgress();
    if (unlistenServerStopped) unlistenServerStopped();
});

const startTunnel = async () => {
    isLoading.value = true;
    try {
        const res = (await invoke("start_tunnel")) as any;
        status.value = res;
    } catch (e) {
        console.error("Failed to start tunnel", e);
    } finally {
        isLoading.value = false;
    }
};

const stopTunnel = async () => {
    isLoading.value = true;
    try {
        await invoke("stop_tunnel");
        status.value = { active: false, address: null, claim_url: null };
    } catch (e) {
        console.error("Failed to stop tunnel", e);
    } finally {
        isLoading.value = false;
    }
};

const copyAddress = () => {
    if (status.value.address) {
        navigator.clipboard.writeText(status.value.address);
        copied.value = true;
        setTimeout(() => (copied.value = false), 2000);
    }
};

const openClaimUrl = async () => {
    if (status.value.claim_url) {
        await openUrl(status.value.claim_url);
    }
};

// Re-check server status every time the modal is opened
watch(
    () => props.isOpen,
    (open) => {
        if (open) fetchServerStatus();
    },
);

// Auto-open the browser when the claim URL arrives while the modal is open
// (backup in case the Rust-side auto-open fires before the modal is mounted)
watch(
    () => status.value.claim_url,
    (newUrl, oldUrl) => {
        if (newUrl && !oldUrl) {
            openUrl(newUrl).catch(console.error);
        }
    },
);
</script>

<template>
    <Modal
        :is-open="isOpen"
        :anchor-el="anchorEl"
        :layout-id="layoutId"
        @close="emit('close')"
    >
        <div class="relative flex flex-col p-4 w-full max-w-[380px] gap-3">
            <!-- Header -->
            <header class="relative flex items-center justify-center py-0.5">
                <button
                    @click="emit('close')"
                    class="absolute left-0 p-1.5 rounded-full hover:bg-black/5 dark:hover:bg-white/5 transition-colors group"
                >
                    <XIcon
                        :size="16"
                        class="text-black/60 dark:text-white/60 group-hover:text-black dark:group-hover:text-white"
                    />
                </button>

                <h2
                    class="text-base font-bold tracking-tight text-black dark:text-white"
                >
                    Playit Tunnel
                </h2>
            </header>

            <!-- Content -->
            <div class="flex flex-col gap-3">
                <!-- Status & IP Zone -->
                <div
                    :class="[
                        'relative flex flex-col items-center justify-center gap-3 py-5 px-4 rounded-3xl border-2 transition-all duration-300',
                        status.active
                            ? 'border-blue-500/20 bg-blue-500/5'
                            : 'border-black/10 dark:border-white/10 bg-black/2 dark:bg-white/2',
                    ]"
                >
                    <!-- Globe Icon -->
                    <div
                        :class="[
                            'w-10 h-10 rounded-full flex items-center justify-center transition-transform duration-500',
                            status.active
                                ? 'bg-blue-500 text-white scale-110 shadow-[0_0_12px_rgba(59,130,246,0.3)]'
                                : 'bg-black/5 dark:bg-white/5 text-black/40 dark:text-white/40',
                        ]"
                    >
                        <GlobeIcon :size="20" />
                    </div>

                    <!-- Status Info -->
                    <div class="text-center w-full">
                        <div
                            class="flex items-center justify-center gap-1.5 mb-0.5"
                        >
                            <div
                                :class="[
                                    'w-1.5 h-1.5 rounded-full',
                                    status.active
                                        ? 'bg-green-500 animate-pulse'
                                        : 'bg-red-500',
                                ]"
                            />
                            <span
                                class="text-xs font-bold uppercase tracking-widest"
                                :class="
                                    status.active
                                        ? 'text-green-500'
                                        : 'text-red-500'
                                "
                            >
                                {{ status.active ? "Active" : "Inactive" }}
                            </span>
                        </div>

                        <p
                            v-if="!status.active"
                            class="text-[11px] text-black/40 dark:text-white/40"
                        >
                            The global tunnel is currently offline
                        </p>

                        <!-- Address Display -->
                        <div
                            v-if="status.active && status.address"
                            class="mt-2 flex items-center gap-2 w-full"
                        >
                            <div
                                class="flex-1 bg-black/5 dark:bg-white/5 border border-black/10 dark:border-white/10 rounded-xl px-3 py-2 font-mono text-[11px] text-blue-500 dark:text-blue-400 truncate text-center"
                            >
                                {{ status.address }}
                            </div>
                            <Button
                                @click="copyAddress"
                                class="p-2 bg-black/5 dark:bg-white/5 hover:bg-black/10 dark:hover:bg-white/10 rounded-xl transition-all active:scale-90 shrink-0"
                            >
                                <CheckIcon
                                    v-if="copied"
                                    :size="14"
                                    class="text-green-500"
                                />
                                <CopyIcon
                                    v-else
                                    :size="14"
                                    class="text-black/60 dark:text-white/60"
                                />
                            </Button>
                        </div>

                        <!-- Wait message -->
                        <p
                            v-if="
                                status.active &&
                                !status.address &&
                                !status.claim_url
                            "
                            class="mt-2 text-[10px] italic text-black/40 dark:text-white/40"
                        >
                            Establishing secure connection...
                        </p>
                    </div>

                    <!-- Loader Overlay -->
                    <Transition
                        enter-active-class="transition duration-300 ease-out"
                        enter-from-class="opacity-0"
                        enter-to-class="opacity-100"
                        leave-active-class="transition duration-200 ease-in"
                        leave-from-class="opacity-100"
                        leave-to-class="opacity-0"
                    >
                        <div
                            v-if="isLoading"
                            class="absolute inset-0 bg-white/80 dark:bg-[#1c1c1e]/80 backdrop-blur-sm rounded-3xl flex flex-col items-center justify-center gap-2 z-10 px-6 text-center"
                        >
                            <Loader2
                                :size="20"
                                class="animate-spin text-blue-500"
                            />

                            <template v-if="downloadProgress.process">
                                <p
                                    class="text-xs font-bold text-black dark:text-white uppercase tracking-wider"
                                >
                                    {{ downloadProgress.process }}
                                </p>
                                <div
                                    v-if="downloadProgress.percentage !== null"
                                    class="w-full h-1 bg-black/5 dark:bg-white/10 rounded-full overflow-hidden"
                                >
                                    <div
                                        class="h-full bg-blue-500 transition-all duration-300"
                                        :style="{
                                            width: `${downloadProgress.percentage}%`,
                                        }"
                                    />
                                </div>
                                <p
                                    v-if="downloadProgress.percentage !== null"
                                    class="text-[10px] font-mono text-black/40 dark:text-white/40"
                                >
                                    {{
                                        Math.round(downloadProgress.percentage)
                                    }}%
                                </p>
                            </template>
                            <p
                                v-else
                                class="text-xs font-bold text-black dark:text-white animate-pulse"
                            >
                                Processing...
                            </p>
                        </div>
                    </Transition>
                </div>

                <!-- Authentication Alert (horizontal, compact) -->
                <div
                    v-if="status.claim_url"
                    class="flex items-center gap-3 p-3 bg-blue-500/10 border border-blue-500/20 rounded-2xl"
                >
                    <ShieldCheckIcon
                        :size="18"
                        class="text-blue-500 shrink-0"
                    />
                    <div class="flex-1 min-w-0">
                        <p
                            class="text-[10px] font-bold text-blue-500 uppercase tracking-wider"
                        >
                            Authentication Required
                        </p>
                        <p
                            class="text-[10px] text-black/50 dark:text-white/50 leading-snug mt-0.5"
                        >
                            Link your agent to your Playit.gg account.
                        </p>
                    </div>
                    <Button
                        @click="openClaimUrl"
                        class="shrink-0 px-3 py-2 bg-blue-500 hover:bg-blue-600 text-white flex items-center gap-1.5 text-[11px] font-semibold rounded-xl shadow-md shadow-blue-500/20 transition-colors"
                    >
                        <ExternalLinkIcon :size="12" />
                        Link
                    </Button>
                </div>

                <!-- Server offline warning -->
                <div
                    v-else-if="!serverActive && !status.active"
                    class="flex items-center gap-3 p-3 bg-amber-500/10 border border-amber-500/20 rounded-2xl"
                >
                    <AlertCircle :size="16" class="text-amber-500 shrink-0" />
                    <p
                        class="text-[10px] text-amber-600 dark:text-amber-400 leading-snug"
                    >
                        Server is offline. Start your server first to enable the
                        tunnel.
                    </p>
                </div>

                <!-- Generic Help Alert -->
                <div
                    v-else
                    class="flex items-center gap-3 p-3 bg-black/5 dark:bg-white/5 rounded-2xl"
                >
                    <AlertCircle
                        :size="16"
                        class="text-black/30 dark:text-white/30 shrink-0"
                    />
                    <p
                        class="text-[10px] text-black/40 dark:text-white/40 leading-snug"
                    >
                        Starts a global tunnel so anyone can connect using the
                        address above.
                    </p>
                </div>

                <!-- Main Action Button -->
                <Button
                    v-if="!status.active"
                    @click="startTunnel"
                    :disabled="isLoading || !serverActive"
                    :class="[
                        'w-full py-3 flex items-center justify-center gap-2 rounded-2xl border transition-all duration-300',
                        serverActive
                            ? 'bg-linear-to-br from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 text-white shadow-lg shadow-blue-500/25 hover:shadow-blue-500/40 border-white/10 active:scale-95'
                            : 'bg-black/5 dark:bg-white/5 text-black/30 dark:text-white/30 border-black/10 dark:border-white/10 cursor-not-allowed',
                    ]"
                >
                    <GlobeIcon
                        :size="16"
                        :class="serverActive ? 'animate-pulse' : ''"
                    />
                    <span class="text-sm font-semibold tracking-tight"
                        >Start Global Tunnel</span
                    >
                </Button>
                <Button
                    v-else
                    @click="stopTunnel"
                    :disabled="isLoading"
                    class="w-full py-3 bg-red-500/10 hover:bg-red-500/20 text-red-500 border border-red-500/20 flex items-center justify-center rounded-2xl transition-all duration-300 active:scale-95"
                >
                    <span class="text-sm font-bold tracking-tight"
                        >Stop Tunnel</span
                    >
                </Button>
            </div>
        </div>
    </Modal>
</template>
