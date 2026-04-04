<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';
import Button from '../../components/ui/Button.vue';
import { ChevronLeft, TrashIcon, SquareTerminal, File, Package } from '@lucide/vue';
import ServerNavItem from '../../components/servers/ServerNavItem.vue';
import ServerDeleteModal from '../../components/servers/ServerDeleteModal.vue';

interface Server {
    id: string;
    name: string;
    version: string;
    software: string;
    ram: string;
    java_version: string;
}

const route = useRoute();

const serverID = route.params.id;
const loading = ref(false);
const server = ref<Server | null>(null);
const isDeleteModalOpen = ref(false);
const deleteButtonRef = ref<HTMLElement | null>(null);

const openDeleteModal = () => {
    isDeleteModalOpen.value = true;
    deleteButtonRef.value?.blur();
}

onMounted(async () => {
    loading.value = true;
    server.value = await invoke<Server>("get_server", { id: serverID });
    loading.value = false;
});

</script>

<template>
    <div class="h-full w-full flex flex-col gap-4">
        <section class="flex flex-row gap-4 items-center">
            <RouterLink :to="{ name: 'servers' }" class="w-8 h-8 flex items-center justify-center rounded-full bg-white/5 text-white/50 hover:bg-white/10 hover:text-white transition-colors duration-300">
                <ChevronLeft :size="24" />
            </RouterLink>

            <nav class="flex flex-row gap-2">
                <ServerNavItem :to="{ name: 'server-console' }" :title="'Console'" :icon="SquareTerminal" :active="route.path.includes('console')" />
                <ServerNavItem :to="{ name: 'server-files' }" :title="'Files'" :icon="File" :active="route.path.includes('files')" />
                <ServerNavItem :to="{ name: 'server-mods' }" :title="'Mods'" :icon="Package" :active="route.path.includes('mods')" />
            </nav>
        </section>
        <section class="flex flex-row justify-between items-center">
            <div class="flex flex-col">
                <section class="flex flex-row items-center gap-2">
                    <h1 class="text-4xl font-bold">{{ server?.name }}</h1>
                    <span class="px-2 py-0.5 text-sm font-bold bg-white/5 text-white/40 rounded uppercase tracking-wider">{{ server?.software }} {{ server?.version }}</span>
                </section>
                <p class="text-md text-white/50">{{ server?.ram }} RAM - Java {{ server?.java_version }}</p>
            </div>


            <div class="flex flex-row">
                <Button ref="deleteButtonRef" :layout-id="`delete-server-${server?.id}`" :tooltip="'Delete server'" :tooltip-position="'bottom'" :class="'bg-transparent dark:bg-red-500/10 hover:bg-red-500/10 dark:hover:bg-red-500/10'" @click="openDeleteModal">
                    <TrashIcon :size="24" :class="'text-red-500'" />
                </Button>
            </div>
        </section>

        <div class="w-full h-full bg-[#161616] rounded-xl overflow-y-auto min-h-0 flex flex-col gap-3 content-start">
            <RouterView />
        </div>


        <ServerDeleteModal 
            v-if="server"
            :is-open="isDeleteModalOpen" 
            :on-close="() => { isDeleteModalOpen = false }" 
            :server="server"
            :anchor-el="(deleteButtonRef as any)?.$el ?? deleteButtonRef"
        />
    </div>
</template>