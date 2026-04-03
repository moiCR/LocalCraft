<script setup lang="ts">
import { HomeIcon, Server } from '@lucide/vue';
import Sidebar from '../components/sidebar/Sidebar.vue';
import SidebarItem from '../components/sidebar/SidebarItem.vue';
import { ref } from 'vue';
import { useRoute } from 'vue-router';
import Titlebar from '../titlebar/Titlebar.vue';

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
            <SidebarItem title="Home" :icon="HomeIcon" to="/" :active="route.path === '/'" :isOpen="isOpen" />
            <SidebarItem title="Servers" :icon="Server" to="/servers" :active="route.path === '/servers'" :isOpen="isOpen" />
        </Sidebar>
        <main class="flex-1 overflow-y-auto p-24 w-[calc(100%-256px)] text-black dark:text-white overflow-x-hidden">
            <RouterView />
        </main>
    </div>
    </div>
</template>