<script setup lang="ts">
import { PlayIcon, Rotate3D, SquareIcon, ChevronRight } from '@lucide/vue';
import Button from '../../components/ui/Button.vue';
import { useRoute } from 'vue-router';
import { invoke } from '@tauri-apps/api/core';
import { ref, computed, onMounted, watch, nextTick, onUnmounted } from 'vue';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

import { globalLogsCache, startGlobalListener } from '../../server/serverConsoleLogs'; 

const route = useRoute();
const serverID = route.params.id as string;

const logsContainerRef = ref<HTMLDivElement | null>(null);

const serverLogs = computed(() => globalLogsCache[serverID] || []);

const isActive = ref(false);
const isRestarting = ref(false);
const isStopping = ref(false);
let unlistenStopped: UnlistenFn | null = null;

onMounted(async () => {
    startGlobalListener();
    isActive.value = await invoke('is_server_running', { id: serverID });
    
    unlistenStopped = await listen('server-stopped', async (event) => {
        if (event.payload === serverID) {
            isActive.value = false;
            isStopping.value = false;
            if (isRestarting.value) {
                isRestarting.value = false;
                await startServer();
            }
        }
    });
});

onUnmounted(() => {
    if (unlistenStopped) unlistenStopped();
});

watch(serverLogs, async () => {
    await nextTick();
    if (logsContainerRef.value) {
        logsContainerRef.value.scrollTop = logsContainerRef.value.scrollHeight;
    }
}, { deep: true });

const startServer = async () => {
    if (isActive.value) return;
    try {
        globalLogsCache[serverID] = [];
        isActive.value = true;
        await invoke('start_server', { id: serverID });
        console.log('Server started');
    } catch (error) {
        console.error('Error starting server:', error);
        isActive.value = false;
    }
}

const stopServer = async () => {
    if (!isActive.value || isStopping.value) return;
    isStopping.value = true;
    try {
        await invoke('send_command', { id: serverID, command: 'stop' });
    } catch (error) {
        console.error('Error stopping server:', error);
        isStopping.value = false;
    }
}

const restartServer = async () => {
    if (isStopping.value || isRestarting.value) return;
    
    if (!isActive.value) {
        await startServer();
    } else {
        isRestarting.value = true;
        await stopServer();
    }
}

const commandInput = ref('');

const sendCommand = async () => {
    try {
        await invoke('send_command', { id: serverID, command: commandInput.value });
        commandInput.value = '';
    } catch (error) {
        console.error('Error sending command:', error);
    }
}
</script>

<template>
    <div class="flex flex-col gap-2 w-full h-full">
        <section class="flex flex-row p-4 border-b border-white/10 gap-2">
            <Button @click="startServer" :disabled="isActive || isStopping || isRestarting" :tooltip="'Start server'" :tooltip-position="'bottom'" :class="['bg-transparent dark:bg-green-500/10 hover:bg-green-500/10 dark:hover:bg-green-500/10 transition-all', isActive || isStopping || isRestarting ? 'opacity-50 cursor-not-allowed' : '']">
                <PlayIcon :class="'text-green-500'" :size="16" />
            </Button>
            <Button @click="stopServer" :disabled="!isActive || isStopping || isRestarting" :tooltip="'Stop server'" :tooltip-position="'bottom'" :class="['bg-transparent dark:bg-red-500/10 hover:bg-red-500/10 dark:hover:bg-red-500/10 transition-all', !isActive || isStopping || isRestarting ? 'opacity-50 cursor-not-allowed' : '']">
                <SquareIcon :class="'text-red-500'" :size="16" />
            </Button>
            <Button @click="restartServer" :disabled="!isActive || isStopping || isRestarting" :tooltip="'Restart server'" :tooltip-position="'bottom'" :class="['bg-transparent dark:bg-yellow-500/10 hover:bg-yellow-500/10 dark:hover:bg-yellow-500/10 transition-all', !isActive || isStopping || isRestarting ? 'opacity-50 cursor-not-allowed' : '']">
                <Rotate3D :class="'text-yellow-500'" :size="16" />
            </Button>
        </section>

        <section class="flex flex-col gap-4 w-full h-full p-4 overflow-hidden scroll-smooth">
            <div 
                ref="logsContainerRef"
                class="flex-1 bg-[#0d0d0d] border border-white/5 rounded-xl p-6 overflow-y-auto flex flex-col gap-1.5 font-mono text-xs text-white/70 custom-scrollbar"
            >
                <div v-if="serverLogs.length === 0" class="text-white/30 italic">
                    Waiting for server logs...
                </div>

                <div 
                    v-for="(log, index) in serverLogs" 
                    :key="index"
                    class="whitespace-pre-wrap break-all"
                    :class="{ 'text-red-400': log.includes('[ERROR]') || log.toLowerCase().includes('warn') }"
                >
                    {{ log }}
                </div>
            </div>
            
            <div class="relative group">
                <div class="absolute left-4 top-1/2 -translate-y-1/2 text-white/20 font-mono text-sm group-focus-within:text-brand/40 transition-colors">
                    <ChevronRight :size="16" />
                </div>
                <input
                    type="text"
                    placeholder="Type a command..."
                    v-model="commandInput"
                    @keyup.enter="sendCommand"
                    class="w-full bg-[#0d0d0d] border border-white/5 rounded-xl py-4 pl-10 pr-4 font-mono text-white/90 text-sm outline-none focus:border-white/10 focus:ring-1 focus:ring-white/5 transition-all disabled:opacity-50"
                />
            </div>
        </section>
    </div>
</template>