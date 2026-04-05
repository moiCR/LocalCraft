<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMounted, ref } from 'vue';
import ServerItem from '../components/servers/ServerItem.vue';
import ServerCreateModal from '../components/servers/ServerCreateModal.vue';
import { PlusIcon, RefreshCcw, ServerIcon } from '@lucide/vue';
import Button from '../components/ui/Button.vue';
import Loader from '../components/ui/Loader.vue';


interface Server{
    id: string;
    name: string;
    version: string;
    software: string;
    ram: string;
    java_version: string;
}

const servers = ref<Server[]>([]);
const isLoading = ref(false);

const showCreateModal = ref(false);
const createBtnEl = ref<HTMLElement | null>(null);

const loadServers = async () => {
    isLoading.value = true;
    await new Promise(resolve => setTimeout(resolve, 1000));
    servers.value = await invoke<Server[]>("get_servers");
    isLoading.value = false;
}


onMounted(() => {
    loadServers();
});

</script>

<template>
    <div class="w-full h-full flex flex-col gap-4">
        <header class="flex flex-row justify-between items-center">
            <section class="flex flex-col gap-1">
                <div class="flex flex-row items-center gap-3">
                    <ServerIcon :size="32" class="text-brand" />
                    <h1 class="text-4xl font-black tracking-tight text-white">Your Servers</h1>
                </div>
                <p class="text-white/40 text-sm ml-11 font-medium">Manage and monitor your Minecraft instances from one central hub.</p>
            </section>
            <section class="flex flex-row items-center gap-2">
                <Button ref="createBtnEl" :tooltip="'Create a new server'" :tooltip-position="'bottom'" @click="showCreateModal = true" >
                    <PlusIcon :size="24"/>
                </Button>
                <Button :tooltip="'Refresh servers'" :tooltip-position="'bottom'" @click="loadServers" :disabled="isLoading">
                    <RefreshCcw :size="24" :class="isLoading ? 'animate-spin' : ''"/>
                </Button>
            </section>
        </header>

        <div 
            class="w-full h-full bg-[#161616] rounded-xl p-6 overflow-y-auto min-h-0" 
            :class="(isLoading || servers.length === 0) ? 'flex items-center justify-center' : 'flex flex-col gap-3 content-start'"
        >
            <Loader v-if="isLoading" />
            <div v-else-if="servers.length === 0" class="flex flex-col items-center justify-center gap-3">
                <span class="text-sm font-semibold text-gray-400 animate-pulse tracking-widest uppercase mt-2">No servers found</span>
            </div>
            <ServerItem v-else v-for="server in servers" :key="server.id" :server="server" @deleted="loadServers" />
        </div>
    </div>

    <ServerCreateModal
        :is-open="showCreateModal"
        :anchor-el="(createBtnEl as any)?.$el ?? createBtnEl"
        @close="showCreateModal = false"
        @created="loadServers"
    />
</template>