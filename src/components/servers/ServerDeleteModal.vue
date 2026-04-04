<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { RefreshCcw, AlertTriangle } from '@lucide/vue';
import Button from '../ui/Button.vue';
import Modal from '../ui/Modal.vue';
import { useRouter } from 'vue-router';

interface Server {
    id: string;
    name: string;
    version: string;
    software: string;
    ram: string;
    java_version: string;
}

const props = defineProps<{
    isOpen: boolean;
    onClose?: () => void;
    server: Server;
    anchorEl?: HTMLElement | null;
}>();

const isDeleting = ref(false);
const router = useRouter();

const handleDelete = async () => {
    isDeleting.value = true;
    await invoke('delete_server', {
        id: props.server.id
    }).then(() => {
        setTimeout(() => {
            props.onClose?.();
            isDeleting.value = false;
            router.push({ name: 'servers' });
        }, 800);
    }).catch((error) => {
        console.error(error);
        isDeleting.value = false;
    });
}

</script>

<template>
    <Modal :is-open="isOpen" @close="onClose" :anchor-el="anchorEl" :layout-id="`delete-server-${server.id}` " :class="'max-w-sm'">
        <div 
            class="flex flex-col gap-6 transition-all duration-500 ease-out p-2"
            :class="isDeleting ? 'opacity-40 scale-95 blur-[2px] pointer-events-none' : 'opacity-100 scale-100 blur-0'"
        >
            <div class="flex flex-col gap-2 mt-2">
                <div class="flex items-center gap-3 text-red-500">
                    <AlertTriangle :size="16" />
                    <h1 class="text-lg font-bold tracking-tight">Delete server</h1>
                </div>
                <p class="text-white/60 leading-relaxed">
                    Are you sure you want to delete <span class="text-white font-bold underline decoration-red-500/50">{{ server.name }}</span>? 
                    This action is permanent and all data will be lost.
                </p>
            </div>

            <div class="flex flex-row gap-3 justify-end items-center">
                <Button 
                    @click="onClose" 
                    :disabled="isDeleting"
                    class="bg-transparent hover:bg-white/5 border border-white/10 text-sm"
                >
                    Cancel
                </Button>
                <Button 
                    @click="handleDelete" 
                    :disabled="isDeleting"
                    class="bg-red-500 hover:bg-red-600 dark:bg-red-500 dark:hover:bg-red-600 text-white flex items-center justify-center gap-2"
                >
                    <RefreshCcw v-if="isDeleting" :size="16" class="animate-spin" />
                    <span class="text-sm">{{ isDeleting ? 'Deleting...' : 'Delete Server' }}</span>
                </Button>
            </div>
        </div>

        <div 
            v-if="isDeleting" 
            class="absolute inset-0 flex items-center justify-center pointer-events-none animate-in fade-in zoom-in duration-300"
        >
            <div class="flex flex-col items-center gap-2">
                <div class="w-12 h-12 border-4 border-red-500/20 border-t-red-500 rounded-full animate-spin"></div>
            </div>
        </div>
    </Modal>
</template>