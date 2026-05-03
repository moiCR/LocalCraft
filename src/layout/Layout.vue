<script setup lang="ts">
import { Coffee, HomeIcon, Info, Server } from "@lucide/vue";

import { useRoute } from "vue-router";
import Titlebar from "../titlebar/Titlebar.vue";
import Navbar from "../components/navbar/Navbar.vue";
import NavbarItem from "../components/navbar/NavbarItem.vue";

const route = useRoute();
</script>

<template>
    <div class="app-shell flex h-screen w-full flex-col text-white">
        <Titlebar />
        <main
            class="flex-1 min-h-0 overflow-y-auto items-center justify-center px-52 pt-12 pb-28 w-full overflow-x-hidden"
        >
            <RouterView />
        </main>

        <Navbar>
            <NavbarItem
                title="Home"
                :active="route.name === 'home'"
                :icon="HomeIcon"
                to="home"
            />
            <NavbarItem
                title="Servers"
                :active="
                    route.name === 'servers' ||
                    String(route.name).startsWith('server-')
                "
                :icon="Server"
                to="servers"
            />
            <NavbarItem
                title="Java"
                :active="route.name === 'java'"
                :icon="Coffee"
                to="java"
            />
            <NavbarItem
                title="About"
                :active="route.name === 'about'"
                :icon="Info"
                to="about"
            />
        </Navbar>
    </div>
</template>
