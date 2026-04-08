<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { onMounted, ref } from "vue";
import JavaItem from "../components/java/JavaItem.vue";
import JavaInstallModal from "../components/java/JavaInstallModal.vue";
import { PlusIcon, RefreshCcw, Coffee, FolderOpen } from "@lucide/vue";
import Button from "../components/ui/Button.vue";
import Loader from "../components/ui/Loader.vue";

const javaVersions = ref<string[]>([]);
const isLoading = ref(false);

const showInstallModal = ref(false);
const installBtnEl = ref<HTMLElement | null>(null);

const loadJava = async () => {
    isLoading.value = true;
    await new Promise((r) => setTimeout(r, 600));
    javaVersions.value = await invoke<string[]>("get_installed_java");
    isLoading.value = false;
};

const openJavaFolder = async () => {
    try {
        await invoke("open_java_folder");
    } catch (e) {
        console.error("Error opening java folder:", e);
    }
};

onMounted(() => {
    loadJava();
});
</script>

<template>
    <div class="w-full h-full flex flex-col gap-4">
        <header class="flex flex-row justify-between items-center">
            <section class="flex flex-col gap-1">
                <div class="flex flex-row items-center gap-3">
                    <Coffee :size="32" class="text-brand" />
                    <h1 class="text-4xl font-black tracking-tight text-white">
                        Java Environments
                    </h1>
                </div>
                <p class="text-white/40 text-sm ml-11 font-medium">
                    Manage and monitor your installed Java runtimes.
                </p>
            </section>
            <section class="flex flex-row items-center gap-2">
                <Button
                    :tooltip="'Open Java folder'"
                    :tooltip-position="'bottom'"
                    @click="openJavaFolder"
                >
                    <FolderOpen :size="20" />
                </Button>
                <Button
                    ref="installBtnEl"
                    :tooltip="'Install a new Java runtime'"
                    :tooltip-position="'bottom'"
                    @click="showInstallModal = true"
                    layout-id="java-install-modal"
                >
                    <PlusIcon :size="24" />
                </Button>
                <Button
                    :tooltip="'Refresh'"
                    :tooltip-position="'bottom'"
                    @click="loadJava"
                    :disabled="isLoading"
                >
                    <RefreshCcw
                        :size="24"
                        :class="isLoading ? 'animate-spin' : ''"
                    />
                </Button>
            </section>
        </header>

        <div
            class="w-full h-full bg-[#161616] rounded-xl p-6 overflow-y-auto min-h-0"
            :class="
                isLoading || javaVersions.length === 0
                    ? 'flex items-center justify-center'
                    : 'flex flex-col gap-3 content-start'
            "
        >
            <Loader v-if="isLoading" />
            <div
                v-else-if="javaVersions.length === 0"
                class="flex flex-col items-center justify-center gap-3"
            >
                <span
                    class="text-sm font-semibold text-gray-400 animate-pulse tracking-widest uppercase mt-2"
                    >No Java runtimes installed</span
                >
            </div>
            <JavaItem
                v-else
                v-for="v in javaVersions"
                :key="v"
                :version="v"
                @deleted="loadJava"
            />
        </div>
    </div>

    <JavaInstallModal
        :is-open="showInstallModal"
        :anchor-el="(installBtnEl as any)?.$el ?? installBtnEl"
        :installed-versions="javaVersions"
        layout-id="java-install-modal"
        @close="showInstallModal = false"
        @installed="loadJava"
    />
</template>
