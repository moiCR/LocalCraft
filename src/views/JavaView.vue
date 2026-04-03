<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Coffee, CheckCircle, Download, Loader2, Trash2 } from '@lucide/vue';
import Button from '../components/ui/Button.vue';

interface JavaEnv {
    version: string;
    description: string;
    isInstalled: boolean;
    isInstalling: boolean;
}

const installedVersions = ref<string[]>([]);
const progress = ref({ message: '', percentage: 0 });
const isAnyInstalling = ref(false);

const javaEnvs = ref<JavaEnv[]>([
    { version: '21', description: 'Recomendado para Minecraft 1.20.5+', isInstalled: false, isInstalling: false },
    { version: '17', description: 'Recomendado para Minecraft 1.18 - 1.20.4', isInstalled: false, isInstalling: false },
    { version: '16', description: 'Recomendado para Minecraft 1.17', isInstalled: false, isInstalling: false },
    { version: '11', description: 'Recomendado para Minecraft 1.12 - 1.16', isInstalled: false, isInstalling: false },
    { version: '8',  description: 'Recomendado para versiones clásicas (< 1.12)', isInstalled: false, isInstalling: false },
]);

const loadInstalled = async () => {
    try {
        installedVersions.value = await invoke('get_installed_java');
        javaEnvs.value.forEach(env => {
            env.isInstalled = installedVersions.value.includes(env.version);
        });
    } catch (e) {
        console.error("Error cargando Java:", e);
    }
};

let unlistenProgress: any;

onMounted(async () => {
    await loadInstalled();
    
    unlistenProgress = await listen('creation-progress', (event: any) => {
        const payload = event.payload;
        if (payload.process.toLowerCase().includes('java')) {
            progress.value.message = payload.process;
            if (payload.percentage !== null) {
                progress.value.percentage = payload.percentage;
            }
        }
    });
});

onUnmounted(() => {
    if (unlistenProgress) unlistenProgress();
});

const installJava = async (env: JavaEnv) => {
    console.log("Instalar Java", env.version);
};

</script>

<template>
    <div class="java-view animate-in fade-in slide-in-from-bottom-4 duration-700">
        <header class="flex flex-col gap-2 mb-10">
            <h1 class="text-4xl font-black tracking-tight text-white flex items-center gap-3">
                <Coffee class="text-brand w-10 h-10" />
                Entornos Java
            </h1>
            <p class="text-white/50 max-w-2xl text-lg">
                Gestiona las instalaciones de Java necesarias para ejecutar diferentes versiones de Minecraft. 
                LocalCraft descarga automáticamente la versión correcta si no está presente.
            </p>
        </header>

        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            <div 
                v-for="env in javaEnvs" 
                :key="env.version"
                class="java-card group"
                :class="{ 'installed': env.isInstalled }"
            >
                <div class="flex items-start justify-between mb-4">
                    <div class="p-3 rounded-2xl bg-white/5 border border-white/10 group-hover:bg-brand/10 group-hover:border-brand/30 transition-all duration-300">
                        <Coffee class="w-6 h-6" :class="env.isInstalled ? 'text-brand' : 'text-white/40'" />
                    </div>
                    <div v-if="env.isInstalled" class="flex items-center gap-1.5 px-3 py-1 rounded-full bg-brand/10 border border-brand/20">
                        <CheckCircle class="w-3.5 h-3.5 text-brand" />
                        <span class="text-[10px] font-bold text-brand uppercase tracking-wider">Instalado</span>
                    </div>
                </div>

                <h3 class="text-xl font-bold text-white mb-1">Java {{ env.version }}</h3>
                <p class="text-sm text-white/40 mb-6 leading-relaxed">{{ env.description }}</p>

                <div class="flex items-center gap-2 mt-auto">
                    <Button 
                        v-if="!env.isInstalled"
                        variant="primary" 
                        class="flex-1 py-3! rounded-xl!" 
                        @click="installJava(env)"
                        :disabled="isAnyInstalling"
                    >
                        <template #icon>
                            <Loader2 v-if="env.isInstalling" class="w-4 h-4 animate-spin" />
                            <Download v-else class="w-4 h-4" />
                        </template>
                        {{ env.isInstalling ? 'Instalando...' : 'Descargar' }}
                    </Button>
                    <Button 
                        v-else
                        variant="secondary" 
                        class="flex-1 py-3! rounded-xl! bg-white/5! border-white/10! group-hover:bg-white/10!"
                        @click="installJava(env)"
                    >
                        <template #icon>
                            <CheckCircle class="w-4 h-4 text-brand" />
                        </template>
                        Listo
                    </Button>
                    <button v-if="env.isInstalled" class="p-3 rounded-xl bg-red-500/5 hover:bg-red-500/20 border border-red-500/0 hover:border-red-500/20 text-red-500/50 hover:text-red-500 transition-all">
                        <Trash2 class="w-5 h-5" />
                    </button>
                </div>
            </div>
        </div>

        <!-- Progress Overlay for global downloads -->
        <Transition name="fade-slide">
            <div v-if="progress.message && progress.percentage < 100" class="fixed bottom-8 right-8 w-80 bg-[#1a1a1a] border border-white/10 rounded-2xl p-4 shadow-2xl z-50 backdrop-blur-xl">
                <div class="flex items-center gap-3 mb-3">
                    <Loader2 class="w-5 h-5 text-brand animate-spin" />
                    <span class="text-xs font-medium text-white/80 line-clamp-1 truncate">{{ progress.message }}</span>
                </div>
                <div class="h-1.5 w-full bg-white/5 rounded-full overflow-hidden">
                    <div 
                        class="h-full bg-brand rounded-full transition-all duration-300 shadow-[0_0_10px_rgba(0,255,136,0.5)]"
                        :style="{ width: `${progress.percentage}%` }"
                    ></div>
                </div>
                <span class="absolute top-4 right-4 text-[10px] font-bold text-white/40">{{ Math.round(progress.percentage) }}%</span>
            </div>
        </Transition>
    </div>
</template>

<style scoped>
.java-card {
    position: relative;
    overflow: hidden;
    padding: 1.5rem;
    border-radius: 2rem;
    background: rgba(26, 26, 26, 0.4);
    border: 1px solid rgba(255, 255, 255, 0.05);
    display: flex;
    flex-direction: column;
    transition: all 0.5s cubic-bezier(0.4, 0, 0.2, 1);
    backdrop-filter: blur(10px);
}

.java-card:hover {
    transform: translateY(-8px);
    border-color: rgba(0, 255, 136, 0.2);
    background: rgba(26, 26, 26, 0.6);
    box-shadow: 0 20px 40px -15px rgba(0, 0, 0, 0.4);
}

.java-card.installed {
    border-color: rgba(0, 255, 136, 0.1);
    background: linear-gradient(135deg, rgba(26, 26, 26, 0.4) 0%, rgba(0, 255, 136, 0.02) 100%);
}

.fade-slide-enter-active, .fade-slide-leave-active {
    transition: all 0.5s cubic-bezier(0.34, 1.56, 0.64, 1);
}
.fade-slide-enter-from { opacity: 0; transform: translateY(20px); }
.fade-slide-leave-to { opacity: 0; transform: translateY(10px); }
</style>
