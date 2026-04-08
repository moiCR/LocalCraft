<script setup lang="ts">
import { HomeIcon, Server, Coffee, Info } from "@lucide/vue";

import Sidebar from "../components/sidebar/Sidebar.vue";
import SidebarItem from "../components/sidebar/SidebarItem.vue";
import { ref } from "vue";
import { useRoute } from "vue-router";
import Titlebar from "../titlebar/Titlebar.vue";

const isOpen = ref(true);

const toggleSidebar = () => {
    isOpen.value = !isOpen.value;
};

const route = useRoute();
</script>

<template>
    <div class="flex flex-col h-screen w-full">
        <Titlebar />
        <div class="flex flex-row h-screen w-full">
            <Sidebar :isOpen="isOpen" :toggleSidebar="toggleSidebar">
                <div class="flex flex-col gap-1">
                    <span
                        v-if="isOpen"
                        class="px-4 mb-2 text-[10px] font-bold text-white/30 tracking-widest uppercase animate-in fade-in duration-500"
                        >MAIN</span
                    >
                    <SidebarItem
                        title="Home"
                        :icon="HomeIcon"
                        to="home"
                        :active="route.path.includes('home')"
                        :isOpen="isOpen"
                    />
                    <SidebarItem
                        title="Servers"
                        :icon="Server"
                        to="servers"
                        :active="route.path.includes('servers')"
                        :isOpen="isOpen"
                    />
                </div>

                <div class="flex flex-col gap-1 mt-6">
                    <span
                        v-if="isOpen"
                        class="px-4 mb-2 text-[10px] font-bold text-white/30 tracking-widest uppercase animate-in fade-in duration-500"
                        >MANAGEMENT</span
                    >
                    <SidebarItem
                        title="Java Environments"
                        :icon="Coffee"
                        to="java"
                        :active="route.path.includes('java')"
                        :isOpen="isOpen"
                    />
                    <!--<SidebarItem title="Backups" :icon="Database" to="/backups" :active="route.path === '/backups'" :isOpen="isOpen" />-->
                </div>

                <div class="flex flex-col gap-1 mt-6">
                    <span
                        v-if="isOpen"
                        class="px-4 mb-2 text-[10px] font-bold text-white/30 tracking-widest uppercase animate-in fade-in duration-500"
                        >ABOUT</span
                    >
                    <SidebarItem
                        title="About"
                        :icon="Info"
                        to="about"
                        :active="route.path.includes('about')"
                        :isOpen="isOpen"
                    />
                </div>
            </Sidebar>

            <main
                class="flex-1 overflow-y-auto px-24 py-12 w-[calc(100%-256px)] text-black dark:text-white overflow-x-hidden"
            >
                <RouterView />
            </main>
        </div>
    </div>
</template>
