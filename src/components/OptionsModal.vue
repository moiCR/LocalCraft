<script setup lang="ts">
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { check } from "@tauri-apps/plugin-updater";
import {
    AlertCircle,
    CheckCircle2,
    Download,
    LoaderCircle,
    Monitor,
    Moon,
    Palette,
    RefreshCw,
    Sun,
    X,
} from "@lucide/vue";
import { computed, onMounted, ref } from "vue";
import Modal from "./ui/Modal.vue";
import GithubIcon from "./icons/GithubIcon.vue";
import {
    setThemePreference,
    themePreference,
    type ThemePreference,
} from "../theme";

interface Props {
    isOpen: boolean;
    anchorEl: HTMLElement | null;
}

defineProps<Props>();
const emit = defineEmits<{
    (e: "close"): void;
}>();

const appVersion = ref("");
const updaterEnabled = import.meta.env.VITE_ENABLE_UPDATER === "true";
const updateStatus = ref<
    "idle" | "checking" | "available" | "unavailable" | "error"
>("idle");
const updateVersion = ref("");
const updateError = ref("");

const themeOptions = [
    { label: "System", value: "system" as const, icon: Monitor },
    { label: "Dark", value: "dark" as const, icon: Moon },
    { label: "Light", value: "light" as const, icon: Sun },
];

const close = () => emit("close");
const selectTheme = (theme: ThemePreference) => setThemePreference(theme);

const updateLabel = computed(() => {
    if (updateStatus.value === "checking") return "Searching...";
    if (updateStatus.value === "available") return `Update v${updateVersion.value}`;
    if (updateStatus.value === "unavailable") return "Up to date";
    if (updateStatus.value === "error") return "Search failed";
    return "Search Update";
});

const updateIcon = computed(() => {
    if (updateStatus.value === "checking") return LoaderCircle;
    if (updateStatus.value === "available") return Download;
    if (updateStatus.value === "unavailable") return CheckCircle2;
    if (updateStatus.value === "error") return AlertCircle;
    return RefreshCw;
});

const openGithub = async () => {
    try {
        await openUrl("https://github.com/moiCR/LocalCraft");
    } finally {
        close();
    }
};

const searchUpdate = async () => {
    if (!updaterEnabled) return;
    if (updateStatus.value === "checking") return;

    updateStatus.value = "checking";
    updateVersion.value = "";
    updateError.value = "";

    try {
        const update = await check();
        if (update) {
            updateVersion.value = update.version;
            updateStatus.value = "available";
            return;
        }

        updateStatus.value = "unavailable";
    } catch (error) {
        updateError.value = error instanceof Error ? error.message : String(error);
        updateStatus.value = "error";
    }
};

onMounted(async () => {
    try {
        appVersion.value = await getVersion();
    } catch {
        appVersion.value = "";
    }
});
</script>

<template>
    <Modal
        :is-open="isOpen"
        layout-id="options-modal"
        :anchor-el="anchorEl"
        @close="close"
    >
        <section class="w-[320px] p-3 text-[#253126] dark:text-white">
            <header class="mb-1 flex items-center justify-between px-1">
                <div>
                    <p class="text-xs font-semibold text-[#667466] dark:text-white/38">
                        {{ appVersion ? `v${appVersion}` : "" }}
                    </p>
                </div>

                <button
                    type="button"
                    title="Close"
                    class="flex h-8 w-8 items-center justify-center rounded-full text-[#6e7c6e] transition-colors hover:bg-[#253126]/8 hover:text-[#253126] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40 dark:text-white/45 dark:hover:bg-white/10 dark:hover:text-white"
                    @click="close"
                >
                    <X :size="16" />
                </button>
            </header>

            <div class="space-y-4">
                <section class="rounded-[18px] border border-[#d6e0d3] bg-[#f2f7ef] p-3 dark:border-white/8 dark:bg-white/3">
                    <div class="mb-3 flex items-center gap-2">
                        <span class="flex h-8 w-8 items-center justify-center rounded-[13px] border border-[#cadac6] bg-white text-[#008c4a] dark:border-white/8 dark:bg-white/5 dark:text-brand">
                            <Palette :size="17" />
                        </span>
                        <div>
                            <h3 class="text-xs font-black uppercase tracking-wider text-[#253126] dark:text-white/78">
                                Theme
                            </h3>
                            <p class="text-[11px] font-semibold text-[#70806d] dark:text-white/35">
                                Syncs with your preference
                            </p>
                        </div>
                    </div>

                    <div class="grid grid-cols-3 gap-2">
                        <button
                            v-for="option in themeOptions"
                            :key="option.value"
                            type="button"
                            class="flex h-14 flex-col items-center justify-center gap-1 rounded-2xl border border-[#d6e0d3] bg-white text-[#667466] shadow-[0_2px_0_#c1d0be] transition-[color,background-color,border-color,box-shadow,transform] duration-150 hover:-translate-y-0.5 hover:border-[#00a65a]/40 hover:text-[#008c4a] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40 active:translate-y-0 active:shadow-[0_1px_0_#c1d0be] dark:border-white/8 dark:bg-white/5 dark:text-white/45 dark:shadow-none dark:hover:border-brand/35 dark:hover:text-brand"
                            :class="{
                                'border-[#00a65a]/60 bg-[#dcf8e9] text-[#007d43] shadow-[0_2px_0_#9bc9aa] dark:border-brand/50 dark:bg-brand/10 dark:text-brand':
                                    themePreference === option.value,
                            }"
                            @click="selectTheme(option.value)"
                        >
                            <component :is="option.icon" :size="16" />
                            <span class="text-[11px] font-black">{{ option.label }}</span>
                        </button>
                    </div>
                </section>

                <section class="space-y-2">
                    <button
                        v-if="updaterEnabled"
                        type="button"
                        class="group flex h-12 w-full items-center gap-3 rounded-[18px] px-3 text-left font-bold text-[#506050] transition-[color,background-color,transform] duration-150 hover:-translate-y-0.5 hover:bg-[#253126]/8 hover:text-[#008c4a] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40 dark:text-white/72 dark:hover:bg-white/10 dark:hover:text-brand"
                        @click="openGithub"
                    >
                        <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-[13px] border border-[#d6e0d3] bg-white text-[#667466] transition-colors group-hover:border-[#00a65a]/40 group-hover:text-[#008c4a] dark:border-white/8 dark:bg-white/5 dark:text-white/65 dark:group-hover:border-brand/40 dark:group-hover:text-brand">
                            <GithubIcon class="w-4.5 h-4.5" />
                        </span>
                        <span class="text-sm">Github</span>
                    </button>

                    <button
                        type="button"
                        class="group flex h-12 w-full items-center gap-3 rounded-[18px] px-3 text-left font-bold text-[#506050] transition-[color,background-color,transform] duration-150 hover:-translate-y-0.5 hover:bg-[#253126]/8 hover:text-[#008c4a] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40 dark:text-white/72 dark:hover:bg-white/10 dark:hover:text-brand"
                        :class="{
                            'text-[#008c4a] dark:text-brand':
                                updateStatus === 'available',
                            'text-red-600 dark:text-red-300':
                                updateStatus === 'error',
                        }"
                        @click="searchUpdate"
                    >
                        <span class="flex h-8 w-8 shrink-0 items-center justify-center rounded-[13px] border border-[#d6e0d3] bg-white text-[#667466] transition-colors group-hover:border-[#00a65a]/40 group-hover:text-[#008c4a] dark:border-white/8 dark:bg-white/5 dark:text-white/65 dark:group-hover:border-brand/40 dark:group-hover:text-brand">
                            <component
                                :is="updateIcon"
                                :size="18"
                                :class="{
                                    'animate-spin': updateStatus === 'checking',
                                }"
                            />
                        </span>
                        <span class="min-w-0 flex-1 text-sm">{{ updateLabel }}</span>
                    </button>

                    <p
                        v-if="updateStatus === 'error' && updateError"
                        class="px-3 text-[11px] font-semibold leading-snug text-red-600/75 dark:text-red-300/70"
                    >
                        {{ updateError }}
                    </p>
                </section>
            </div>
        </section>
    </Modal>
</template>
