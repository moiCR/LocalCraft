<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef } from "vue";
import { AlertCircle, CircleArrowDown, LoaderCircle, Minus, Square, X } from "@lucide/vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { useRoute } from "vue-router";

const appWindow = getCurrentWindow();
const updaterEnabled = import.meta.env.VITE_ENABLE_UPDATER === "true";

const minimize = () => appWindow.minimize();
const toggleMaximize = () => appWindow.toggleMaximize();
const close = () => appWindow.close();
const route = useRoute();
const availableUpdate = shallowRef<Update | null>(null);
const updateStatus = ref<
    "idle" | "checking" | "available" | "downloading" | "unavailable" | "error"
>("idle");
const updateError = ref("");
const downloadedBytes = ref(0);
const totalBytes = ref<number | null>(null);
let statusResetTimeout: number | null = null;

const isServerRoute = computed(() =>
    route.name?.toString().startsWith("server-"),
);

const baseTitle = computed(() => {
    if (isServerRoute.value) return "Manage Server";
    switch (route.name) {
        case "home":
            return "Home";
        case "servers":
            return "Servers";
        case "java":
            return "Java";
        case "about":
            return "About";
        default:
            return "";
    }
});

const subTitle = computed(() => {
    switch (route.name) {
        case "server-console":
            return "Console";
        case "server-files":
            return "Files";
        case "server-mods":
            return "Mods";
        default:
            return "";
    }
});

const updateProgress = computed(() => {
    if (!totalBytes.value) return null;
    return Math.min(100, Math.round((downloadedBytes.value / totalBytes.value) * 100));
});

const showUpdater = computed(() =>
    updaterEnabled &&
    (availableUpdate.value !== null || updateStatus.value === "downloading"),
);

const updaterTooltip = computed(() => {
    if (updateStatus.value === "checking") return "Buscando actualizaciones";
    if (updateStatus.value === "available" && availableUpdate.value) {
        return `Instalar ${availableUpdate.value.version}`;
    }
    if (updateStatus.value === "downloading") {
        return updateProgress.value === null
            ? "Descargando actualizacion"
            : `Descargando ${updateProgress.value}%`;
    }
    if (updateStatus.value === "unavailable") return "LocalCraft esta actualizado";
    if (updateStatus.value === "error") return updateError.value || "Error del updater";
    return "Buscar actualizaciones";
});

const clearStatusReset = () => {
    if (statusResetTimeout === null) return;
    window.clearTimeout(statusResetTimeout);
    statusResetTimeout = null;
};

const scheduleStatusReset = () => {
    clearStatusReset();
    statusResetTimeout = window.setTimeout(() => {
        if (updateStatus.value === "unavailable" || updateStatus.value === "error") {
            updateStatus.value = availableUpdate.value ? "available" : "idle";
            updateError.value = "";
        }
        statusResetTimeout = null;
    }, 3500);
};

const checkForUpdate = async (silent = false) => {
    if (!updaterEnabled) return;
    if (updateStatus.value === "checking" || updateStatus.value === "downloading") return;

    clearStatusReset();
    updateError.value = "";
    updateStatus.value = "checking";

    try {
        const update = await check();
        availableUpdate.value = update;
        updateStatus.value = update ? "available" : "unavailable";
        if (!update && !silent) scheduleStatusReset();
        if (!update && silent) updateStatus.value = "idle";
    } catch (error) {
        updateError.value = error instanceof Error ? error.message : String(error);
        updateStatus.value = silent ? "idle" : "error";
        if (!silent) scheduleStatusReset();
    }
};

const installUpdate = async () => {
    if (!updaterEnabled) return;
    if (!availableUpdate.value || updateStatus.value === "downloading") return;

    clearStatusReset();
    updateError.value = "";
    downloadedBytes.value = 0;
    totalBytes.value = null;
    updateStatus.value = "downloading";

    try {
        const update = availableUpdate.value;
        console.log(
            `found update ${update.version} from ${update.date} with notes ${update.body}`,
        );

        await update.downloadAndInstall((event) => {
            switch (event.event) {
                case "Started":
                    totalBytes.value = event.data.contentLength ?? null;
                    downloadedBytes.value = 0;
                    console.log(`started downloading ${event.data.contentLength} bytes`);
                    break;
                case "Progress":
                    downloadedBytes.value += event.data.chunkLength;
                    console.log(`downloaded ${downloadedBytes.value} from ${totalBytes.value}`);
                    break;
                case "Finished":
                    console.log("download finished");
                    break;
            }
        });

        console.log("update installed");
        await relaunch();
    } catch (error) {
        updateError.value = error instanceof Error ? error.message : String(error);
        updateStatus.value = "error";
        scheduleStatusReset();
    }
};

const handleUpdaterClick = () => {
    if (updateStatus.value === "available") {
        void installUpdate();
        return;
    }

    if (
        updateStatus.value === "idle" ||
        updateStatus.value === "unavailable" ||
        updateStatus.value === "error"
    ) {
        void checkForUpdate();
    }
};

onMounted(() => {
    if (!updaterEnabled) return;
    void checkForUpdate(true);
});

onUnmounted(() => {
    clearStatusReset();
});
</script>

<template>
    <div
        data-tauri-drag-region
        class="h-11 w-full select-none flex justify-between rounded-t-lg items-center bg-[#121412] border-b-2 border-[#223127] shrink-0 z-50 shadow-[0_4px_0_rgba(0,255,136,0.05)]"
    >
        <div
            data-tauri-drag-region
            class="pl-4 gap-2 text-xs text-white/35 font-black tracking-widest flex items-center h-full w-full"
        >
            <section class="flex items-center">
                <span class="text-white">LocalCraft</span>
                <span v-if="baseTitle" class="ml-1 opacity-60"> | </span>
                <Transition name="title-fade" mode="out-in">
                    <span
                        :key="baseTitle"
                        v-if="baseTitle"
                        class="ml-2 opacity-60"
                    >
                        {{ baseTitle }}</span
                    >
                </Transition>
                <span v-if="subTitle" class="ml-1 opacity-60"> | </span>
                <Transition name="title-fade" mode="out-in">
                    <span
                        :key="subTitle"
                        v-if="subTitle"
                        class="ml-1 opacity-60"
                    >
                        {{ subTitle }}</span
                    >
                </Transition>
            </section>
        </div>

        <div class="flex h-full items-center gap-1 pr-2">
            <button
                v-if="showUpdater"
                type="button"
                :title="updaterTooltip"
                :disabled="updateStatus === 'checking' || updateStatus === 'downloading'"
                class="relative flex h-7 w-7 items-center justify-center rounded-full border-2 border-[#2d3f33] bg-[#1b211c] text-white/75 shadow-[0_3px_0_#060806] transition-all duration-200 hover:-translate-y-0.5 hover:border-brand/60 hover:text-brand active:translate-y-0.5 active:shadow-none disabled:cursor-default"
                :class="{
                    'bg-brand text-black border-brand': updateStatus === 'available',
                    'bg-red-500/20 text-red-300 border-red-400/50': updateStatus === 'error',
                    'bg-sky-400/15 text-sky-200 border-sky-300/40': updateStatus === 'checking' || updateStatus === 'downloading',
                }"
                @click="handleUpdaterClick"
            >
                <LoaderCircle
                    v-if="updateStatus === 'checking' || updateStatus === 'downloading'"
                    :size="14"
                    class="animate-spin"
                />
                <AlertCircle
                    v-else-if="updateStatus === 'error'"
                    :size="14"
                />
                <CircleArrowDown
                    v-else
                    :size="14"
                />
                <span
                    v-if="updateStatus === 'available'"
                    class="absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full border border-[#050705] bg-brand shadow-[0_0_10px_rgba(0,255,136,0.8)]"
                />
            </button>
            <span
                v-if="showUpdater && updateStatus === 'downloading' && updateProgress !== null"
                class="w-7 text-center text-[10px] font-black tabular-nums text-white/70"
            >
                {{ updateProgress }}%
            </span>
            <button
                @click="minimize"
                class="flex h-7 w-7 items-center justify-center rounded-full text-white/45 transition-colors hover:bg-white/10 hover:text-white"
                title="Minimize"
            >
                <Minus :size="16" />
            </button>

            <button
                @click="toggleMaximize"
                class="flex h-7 w-7 items-center justify-center rounded-full text-white/45 transition-colors hover:bg-white/10 hover:text-white"
                title="Maximize"
            >
                <Square :size="14" />
            </button>

            <button
                @click="close"
                class="flex h-7 w-7 items-center justify-center rounded-full text-white/45 transition-colors hover:bg-red-500 hover:text-white"
                title="Close"
            >
                <X :size="16" />
            </button>
        </div>
    </div>
</template>

<style scoped>
.title-fade-enter-active,
.title-fade-leave-active {
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.title-fade-enter-from {
    opacity: 0;
    transform: translateY(4px);
    filter: blur(3px);
}

.title-fade-leave-to {
    opacity: 0;
    transform: translateY(-4px);
    filter: blur(3px);
}
</style>
