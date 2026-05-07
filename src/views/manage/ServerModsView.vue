<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { ChevronLeft, ChevronRight, Download, ListFilter, RefreshCcw, Search } from "@lucide/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute } from "vue-router";

interface Server {
    id: string;
    name: string;
    version: string;
    software: string;
}

interface ModSearchItem {
    provider: string;
    id: string;
    slug: string;
    title: string;
    description: string;
    author?: string;
    downloads: number;
    icon_url?: string;
    project_url: string;
    side: "Client" | "Client and Server" | "Server";
}

interface ModSearchResponse {
    items: ModSearchItem[];
    total: number;
}

interface InstalledModRecord {
    provider: string;
    id: string;
    filename: string;
}

const route = useRoute();
const searchQuery = ref("");
const installedFilter = ref<"installed" | "not-installed">("not-installed");
const isRefreshing = ref(false);
const server = ref<Server | null>(null);
const mods = ref<ModSearchItem[]>([]);
const totalMods = ref(0);
const error = ref<string | null>(null);
const installingIds = ref<Set<string>>(new Set());
const installedIds = ref<Set<string>>(new Set());
const currentPage = ref(1);
const pageSize = 20;

const installedFilterLabel = computed(() =>
    installedFilter.value === "installed" ? "Installed" : "Not installed",
);

const displayedMods = computed(() =>
    mods.value.filter((mod) =>
        installedFilter.value === "installed"
            ? installedIds.value.has(modKey(mod))
            : !installedIds.value.has(modKey(mod)),
    ),
);

const totalPages = computed(() => Math.max(1, Math.ceil(totalMods.value / pageSize)));
const canGoPrevious = computed(() => currentPage.value > 1 && !isRefreshing.value);
const canGoNext = computed(() => currentPage.value < totalPages.value && !isRefreshing.value);
const pageStart = computed(() =>
    totalMods.value === 0 ? 0 : (currentPage.value - 1) * pageSize + 1,
);
const pageEnd = computed(() =>
    Math.min(currentPage.value * pageSize, totalMods.value),
);

const toggleInstalledFilter = () => {
    installedFilter.value =
        installedFilter.value === "installed" ? "not-installed" : "installed";
};

const loader = computed(() => {
    const software = server.value?.software.toLowerCase();
    return ["forge", "fabric", "quilt", "neoforge"].includes(software || "")
        ? software
        : null;
});

const refreshMods = async () => {
    if (isRefreshing.value) return;
    isRefreshing.value = true;
    error.value = null;

    try {
        const response = await invoke<ModSearchResponse>("search_mods", {
            provider: "modrinth",
            query: searchQuery.value || null,
            gameVersion: server.value?.version || null,
            loader: loader.value,
            index: (currentPage.value - 1) * pageSize,
            limit: pageSize,
        });
        mods.value = response.items;
        totalMods.value = response.total;
    } catch (e) {
        error.value = String(e);
        mods.value = [];
        totalMods.value = 0;
    } finally {
        isRefreshing.value = false;
    }
};

const modKey = (mod: ModSearchItem) => `${mod.provider}-${mod.id}`;
const recordKey = (record: InstalledModRecord) => `${record.provider}-${record.id}`;

const loadInstalledMods = async () => {
    const records = await invoke<InstalledModRecord[]>("get_installed_mods", {
        serverId: route.params.id,
    });
    installedIds.value = new Set(records.map(recordKey));
};

const resetAndRefreshMods = async () => {
    currentPage.value = 1;
    await refreshMods();
};

const goToPreviousPage = async () => {
    if (!canGoPrevious.value) return;
    currentPage.value -= 1;
    await refreshMods();
};

const goToNextPage = async () => {
    if (!canGoNext.value) return;
    currentPage.value += 1;
    await refreshMods();
};

const installMod = async (mod: ModSearchItem) => {
    const key = modKey(mod);
    if (installingIds.value.has(key) || installedIds.value.has(key)) return;

    installingIds.value = new Set(installingIds.value).add(key);
    error.value = null;

    try {
        await invoke("install_mod", {
            serverId: route.params.id,
            provider: "modrinth",
            modId: mod.id,
            gameVersion: server.value?.version,
            loader: loader.value,
        });
        await loadInstalledMods();
    } catch (e) {
        error.value = String(e);
    } finally {
        const next = new Set(installingIds.value);
        next.delete(key);
        installingIds.value = next;
    }
};

onMounted(async () => {
    server.value = await invoke<Server>("get_server", {
        id: route.params.id,
    });
    await loadInstalledMods();
    await refreshMods();
});

</script>

<template>
    <div class="flex h-full w-full flex-col overflow-hidden">
        <section
            class="flex shrink-0 flex-row items-center justify-between gap-2 border-b border-[#26382d] bg-[#151815]/70 px-4 py-3"
        >
            <div class="flex min-w-0 flex-1 items-center gap-2">
                <div
                    class="group flex h-11 w-full max-w-md items-center overflow-hidden rounded-2xl border-2 border-[#223127] bg-[#0c0f0d] text-white/80 shadow-[0_4px_0_#060806] transition-all focus-within:border-brand/40 focus-within:ring-4 focus-within:ring-brand/10"
                >
                    <div
                        class="flex h-full w-12 shrink-0 items-center justify-center border-r border-white/10 bg-white/[0.03] text-white/35 transition-colors group-focus-within:text-green-300"
                    >
                        <Search :size="18" />
                    </div>
                <input
                    v-model="searchQuery"
                    type="text"
                    placeholder="Search mods"
                    class="h-full min-w-0 flex-1 bg-transparent px-3 text-sm text-white outline-none placeholder:text-white/30"
                    @keyup.enter="resetAndRefreshMods"
                />
                </div>
                <button
                    type="button"
                    :title="`Filter: ${installedFilterLabel}`"
                    class="flex h-11 shrink-0 items-center gap-2 rounded-2xl border px-3 text-sm font-bold shadow-[0_4px_0_#060806] transition-all focus:outline-none focus:ring-4 focus:ring-brand/10 active:translate-y-0.5 active:shadow-[0_1px_0_#060806]"
                    :class="
                        installedFilter === 'installed'
                            ? 'border-brand/35 bg-brand/10 text-brand hover:bg-brand/15'
                            : 'border-white/10 bg-white/5 text-white/70 hover:bg-white/10 hover:text-white'
                    "
                    @click="toggleInstalledFilter"
                >
                    <ListFilter :size="16" />
                    <span>{{ installedFilterLabel }}</span>
                </button>
            </div>
            <div class="flex shrink-0 items-center gap-2">
                <button
                    type="button"
                    title="Refresh mods"
                    class="flex h-11 w-11 shrink-0 items-center justify-center rounded-2xl border border-white/10 bg-white/5 text-white/70 shadow-[0_4px_0_#060806] transition-all hover:bg-white/10 hover:text-white active:translate-y-0.5 active:shadow-[0_1px_0_#060806] focus:outline-none focus:ring-4 focus:ring-brand/10 disabled:cursor-not-allowed disabled:opacity-50"
                    :disabled="isRefreshing"
                    @click="resetAndRefreshMods"
                >
                    <RefreshCcw
                        :size="18"
                        :class="{ 'animate-spin': isRefreshing }"
                    />
                </button>
            </div>
        </section>
        <section class="min-h-0 flex-1 overflow-y-auto p-4">
            <div
                v-if="error"
                class="rounded-lg border border-red-400/20 bg-red-400/10 px-4 py-3 text-sm text-red-300"
            >
                {{ error }}
            </div>
            <div
                v-else-if="isRefreshing"
                class="flex h-full items-center justify-center text-sm font-semibold uppercase tracking-widest text-white/40"
            >
                Loading mods
            </div>
            <div
                v-else-if="displayedMods.length === 0"
                class="flex h-full items-center justify-center text-sm font-semibold uppercase tracking-widest text-white/40"
            >
                No mods found
            </div>
            <div
                v-else
                class="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3"
            >
                <article
                    v-for="mod in displayedMods"
                    :key="`${mod.provider}-${mod.id}`"
                    class="flex min-h-32 gap-3 rounded-[22px] border-2 border-[#223127] bg-[#0c0f0d] p-3 text-left shadow-[0_5px_0_#060806] transition-all hover:-translate-y-0.5 hover:border-brand/25 hover:bg-[#101611]"
                >
                    <img
                        v-if="mod.icon_url"
                        :src="mod.icon_url"
                        :alt="`${mod.title} icon`"
                                class="h-12 w-12 shrink-0 rounded-2xl border border-white/10 object-cover"
                    />
                    <div
                        v-else
                            class="h-12 w-12 shrink-0 rounded-2xl border border-white/10 bg-white/5"
                    />
                    <div class="min-w-0 flex-1">
                        <div class="flex min-w-0 items-start justify-between gap-2">
                            <a
                                :href="mod.project_url"
                                target="_blank"
                                rel="noreferrer"
                                class="min-w-0 text-sm font-bold text-white hover:text-green-300"
                            >
                                <span class="block truncate">{{ mod.title }}</span>
                            </a>
                            <span
                                class="shrink-0 rounded border px-2 py-0.5 text-[11px] font-bold"
                                :class="{
                                    'border-blue-400/20 bg-blue-400/10 text-blue-300': mod.side === 'Client',
                                    'border-purple-400/20 bg-purple-400/10 text-purple-300': mod.side === 'Client and Server',
                                    'border-green-400/20 bg-green-400/10 text-green-300': mod.side === 'Server',
                                }"
                            >
                                {{ mod.side }}
                            </span>
                        </div>
                        <p class="mt-1 line-clamp-2 text-xs text-white/45">
                            {{ mod.description }}
                        </p>
                        <div class="mt-3 flex items-center justify-between gap-2">
                            <p class="min-w-0 truncate text-xs text-white/30">
                                {{ mod.downloads.toLocaleString() }} downloads
                                <span v-if="mod.author">by {{ mod.author }}</span>
                            </p>
                            <button
                                type="button"
                                class="flex h-8 shrink-0 items-center gap-1.5 rounded-xl border px-2 text-xs font-black transition-colors disabled:cursor-not-allowed"
                                :class="
                                    installedIds.has(modKey(mod))
                                        ? 'border-green-400/20 bg-green-400/10 text-green-300'
                                        : 'border-white/10 bg-white/5 text-white/70 hover:bg-white/10 hover:text-white disabled:opacity-60'
                                "
                                :disabled="installingIds.has(modKey(mod)) || installedIds.has(modKey(mod))"
                                @click="installMod(mod)"
                            >
                                <RefreshCcw
                                    v-if="installingIds.has(modKey(mod))"
                                    :size="14"
                                    class="animate-spin"
                                />
                                <Download
                                    v-else
                                    :size="14"
                                />
                                <span>
                                    {{ installedIds.has(modKey(mod)) ? "Installed" : "Install" }}
                                </span>
                            </button>
                        </div>
                    </div>
                </article>
            </div>
        </section>
        <section
            class="flex h-14 shrink-0 items-center justify-between border-t border-[#26382d] bg-[#151815]/70 px-4 text-sm text-white/40"
        >
            <p>
                <span v-if="totalMods > 0">
                    {{ pageStart }}-{{ pageEnd }} of {{ totalMods.toLocaleString() }}
                </span>
                <span v-else>No results</span>
            </p>
            <div class="flex items-center gap-2">
                <button
                    type="button"
                    title="Previous page"
                    class="flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-white/70 transition-colors hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:opacity-40"
                    :disabled="!canGoPrevious"
                    @click="goToPreviousPage"
                >
                    <ChevronLeft :size="18" />
                </button>
                <span class="min-w-20 text-center text-xs font-bold uppercase tracking-wider text-white/35">
                    {{ currentPage }} / {{ totalPages }}
                </span>
                <button
                    type="button"
                    title="Next page"
                    class="flex h-9 w-9 items-center justify-center rounded-xl border border-white/10 bg-white/5 text-white/70 transition-colors hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:opacity-40"
                    :disabled="!canGoNext"
                    @click="goToNextPage"
                >
                    <ChevronRight :size="18" />
                </button>
            </div>
        </section>
    </div>
</template>
