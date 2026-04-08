<script setup lang="ts">
import { ref, watch } from "vue";
import { Loader2, Save, AlertTriangle } from "@lucide/vue";
import Modal from "../ui/Modal.vue";
import Select from "../ui/Select.vue";
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
    anchorEl?: HTMLElement | null;
    isOpen: boolean;
    layoutId?: string;
}>();

const emit = defineEmits<{
    (e: "close"): void;
    (e: "saved", server: Server): void;
}>();

const ramOptions = [
    { label: "512 MB", value: "512" },
    { label: "1 GB", value: "1024" },
    { label: "2 GB", value: "2048" },
    { label: "4 GB", value: "4096" },
    { label: "8 GB", value: "8192" },
    { label: "16 GB", value: "16384" },
    { label: "32 GB", value: "32768" },
];

const name = ref("");
const ram = ref("");
const javaVersion = ref("");
const javaOptions = ref<{ label: string; value: string }[]>([]);
const isServerRunning = ref(false);
const isSaving = ref(false);
const error = ref("");

const handleClose = () => {
    error.value = "";
    emit("close");
};

watch(
    () => props.isOpen,
    async (open) => {
        if (!open) return;

        // Reset to current server values
        name.value = props.server.name;
        ram.value = props.server.ram;
        javaVersion.value = props.server.java_version ?? "";
        error.value = "";
        isSaving.value = false;

        // Load installed Java versions
        try {
            const installed = await invoke<string[]>("get_installed_java");
            javaOptions.value = installed
                .sort((a, b) => Number(b) - Number(a))
                .map((v) => ({ label: `Java ${v}`, value: v }));

            // If current java_version is not installed, add it as a fallback option
            const current = props.server.java_version;
            if (current && !installed.includes(current)) {
                javaOptions.value.unshift({
                    label: `Java ${current} (not installed)`,
                    value: current,
                });
            }
        } catch {
            javaOptions.value = [];
        }

        // Check if server is running
        try {
            isServerRunning.value = await invoke<boolean>("is_server_running", {
                id: props.server.id,
            });
        } catch {
            isServerRunning.value = false;
        }
    },
);

const handleSave = async () => {
    if (!name.value.trim()) {
        error.value = "Name cannot be empty.";
        return;
    }
    if (!javaVersion.value) {
        error.value = "Please select a Java version.";
        return;
    }

    isSaving.value = true;
    error.value = "";

    try {
        const updated = await invoke<Server>("update_server", {
            id: props.server.id,
            name: name.value.trim(),
            ram: ram.value,
            javaVersion: javaVersion.value,
        });
        emit("saved", updated);
        emit("close");
    } catch (err: any) {
        error.value = err?.toString() ?? "An unexpected error occurred.";
    } finally {
        isSaving.value = false;
    }
};
</script>

<template>
    <Modal
        :is-open="isOpen"
        :anchor-el="anchorEl"
        :layout-id="layoutId"
        @close="handleClose"
    >
        <div class="flex flex-col p-6 gap-5 w-full min-w-[320px]">
            <!-- Header -->
            <h3
                class="text-base font-bold text-black dark:text-white tracking-tight"
            >
                Server Settings
            </h3>

            <!-- Running warning -->
            <div
                v-if="isServerRunning"
                class="flex items-center gap-2.5 px-3 py-2.5 bg-amber-500/10 border border-amber-500/20 rounded-xl"
            >
                <AlertTriangle :size="14" class="text-amber-500 shrink-0" />
                <p
                    class="text-[11px] text-amber-600 dark:text-amber-400 leading-snug"
                >
                    Changes to RAM and Java will take effect on the next
                    restart.
                </p>
            </div>

            <!-- Form -->
            <div class="flex flex-col gap-4">
                <!-- Name -->
                <div class="flex flex-col gap-1.5">
                    <label
                        for="settings-name"
                        class="text-xs font-medium text-gray-500 dark:text-gray-400"
                    >
                        Name
                    </label>
                    <input
                        id="settings-name"
                        v-model="name"
                        type="text"
                        placeholder="My Awesome Server"
                        class="border border-black/5 bg-black/2 text-black text-sm dark:border-white/5 dark:bg-white/2 dark:text-white rounded-lg px-3 py-2 focus:outline-none focus:ring-1 focus:ring-black/20 dark:focus:ring-white/20 transition-all placeholder:text-gray-400"
                    />
                </div>

                <!-- RAM + Java -->
                <div class="grid grid-cols-2 gap-3">
                    <Select v-model="ram" :options="ramOptions" label="RAM" />
                    <Select
                        v-model="javaVersion"
                        :options="javaOptions"
                        label="Java"
                        :disabled="javaOptions.length === 0"
                        :placeholder="
                            javaOptions.length === 0
                                ? 'No Java installed'
                                : 'Java'
                        "
                    />
                </div>

                <!-- Error -->
                <p v-if="error" class="text-xs text-red-500 font-medium -mt-1">
                    {{ error }}
                </p>
            </div>

            <!-- Actions -->
            <div class="flex gap-3 pt-1">
                <button
                    type="button"
                    :disabled="isSaving"
                    @click="handleClose"
                    class="flex-1 text-sm font-medium py-2.5 px-4 rounded-lg text-gray-500 hover:text-black dark:hover:text-white hover:bg-black/5 dark:hover:bg-white/5 transition-colors disabled:opacity-40 cursor-pointer"
                >
                    Cancel
                </button>
                <button
                    type="button"
                    :disabled="isSaving || !name.trim() || !javaVersion"
                    @click="handleSave"
                    class="flex-1 flex items-center justify-center gap-2 text-sm font-bold py-2.5 px-4 rounded-lg bg-black text-white dark:bg-white dark:text-black hover:opacity-80 transition-opacity disabled:opacity-30 disabled:cursor-not-allowed cursor-pointer"
                >
                    <Loader2 v-if="isSaving" :size="12" class="animate-spin" />
                    <Save v-else :size="12" />
                    Save
                </button>
            </div>
        </div>
    </Modal>
</template>
