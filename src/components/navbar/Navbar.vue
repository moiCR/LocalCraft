<script setup lang="ts">
import {
    AlertCircle,
    CircleArrowDown,
    LoaderCircle,
} from "@lucide/vue";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import gsap from "gsap";
import { computed, onMounted, onUnmounted, ref } from "vue";

const navRef = ref<HTMLElement | null>(null);
const indicatorRef = ref<HTMLElement | null>(null);
const activeTarget = ref<HTMLElement | null>(null);
const availableUpdate = ref<Update | null>(null);
const updateStatus = ref<
    "idle" | "checking" | "available" | "downloading" | "unavailable" | "error"
>("idle");
const updateError = ref("");
const downloadedBytes = ref(0);
const totalBytes = ref<number | null>(null);
let resizeFrame: number | null = null;
let resizeObserver: ResizeObserver | null = null;
let statusResetTimeout: number | null = null;

const updateProgress = computed(() => {
    if (!totalBytes.value) return null;
    return Math.min(100, Math.round((downloadedBytes.value / totalBytes.value) * 100));
});

const showUpdater = computed(() => availableUpdate.value !== null || updateStatus.value === "downloading");

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

const moveIndicator = (target: HTMLElement, animate = true) => {
    if (!navRef.value || !indicatorRef.value) return;
    if (!target.isConnected) return;

    const navRect = navRef.value.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();
    const position = {
        x: targetRect.left - navRect.left,
        y: targetRect.top - navRect.top,
        width: targetRect.width,
        height: targetRect.height,
        opacity: 1,
    };

    if (!animate) {
        gsap.set(indicatorRef.value, position);
        return;
    }

    gsap.to(indicatorRef.value, {
        ...position,
        duration: 0.42,
        ease: "power3.out",
    });
};

const handleActiveItem = (event: Event) => {
    const target = (event as CustomEvent<HTMLElement>).detail;
    if (!target) return;

    activeTarget.value = target;
    moveIndicator(target);
};

const syncIndicator = () => {
    if (resizeFrame !== null) cancelAnimationFrame(resizeFrame);

    resizeFrame = requestAnimationFrame(() => {
        resizeFrame = null;
        if (activeTarget.value) moveIndicator(activeTarget.value, false);
    });
};

onMounted(() => {
    window.addEventListener("navbar-active-item", handleActiveItem);
    window.addEventListener("resize", syncIndicator);

    if (navRef.value) {
        resizeObserver = new ResizeObserver(syncIndicator);
        resizeObserver.observe(navRef.value);
    }

    void checkForUpdate(true);
});

onUnmounted(() => {
    window.removeEventListener("navbar-active-item", handleActiveItem);
    window.removeEventListener("resize", syncIndicator);
    resizeObserver?.disconnect();
    if (resizeFrame !== null) cancelAnimationFrame(resizeFrame);
    clearStatusReset();
});
</script>

<template>
    <nav
        ref="navRef"
        class="relative shrink-0 h-20 w-full border-t border-white/10 bg-[#161616]/95 backdrop-blur-md px-6 z-30"
    >
        <div
            ref="indicatorRef"
            class="pointer-events-none absolute left-0 top-0 rounded-3xl bg-green-700 opacity-0"
        />
        <div class="relative z-10 mx-auto flex flex-row h-full max-w-2xl items-center justify-center gap-3">
            <div class="flex flex-row justify-between items-center gap-4">
                <div class="flex gap-6">
                    <slot />
                </div>

                <div v-if="showUpdater">
                    <button
                        type="button"
                        :title="updaterTooltip"
                        :disabled="updateStatus === 'checking' || updateStatus === 'downloading'"
                        class="relative flex h-11 min-w-11 items-center justify-center rounded-2xl border border-white/10 bg-white/5 px-3 text-white/70 transition-all duration-300 hover:border-green-400/30 hover:bg-green-400/10 hover:text-green-300 disabled:cursor-default disabled:hover:border-white/10 disabled:hover:bg-white/5"
                        :class="{
                            'border-green-400/40 bg-green-400/10 text-green-300': updateStatus === 'available',
                            'border-red-400/40 bg-red-400/10 text-red-300': updateStatus === 'error',
                        }"
                        @click="handleUpdaterClick"
                    >
                        <LoaderCircle
                            v-if="updateStatus === 'checking' || updateStatus === 'downloading'"
                            :size="22"
                            class="animate-spin"
                        />
                        <AlertCircle
                            v-else-if="updateStatus === 'error'"
                            :size="22"
                        />
                        <CircleArrowDown
                            v-else
                            :size="22"
                        />
                        <span
                            v-if="updateStatus === 'available'"
                            class="absolute -right-1 -top-1 h-3 w-3 rounded-full bg-green-400 shadow-[0_0_10px_rgba(74,222,128,0.8)]"
                        />
                        <span
                            v-if="updateStatus === 'downloading' && updateProgress !== null"
                            class="ml-2 w-8 text-left text-xs font-bold tabular-nums"
                        >
                            {{ updateProgress }}%
                        </span>
                    </button>
                </div>
            </div>
        </div>
    </nav>
</template>
