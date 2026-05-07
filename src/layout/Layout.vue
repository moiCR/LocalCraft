<script setup lang="ts">
import { Coffee, HomeIcon, Info, Server, Settings } from "@lucide/vue";

import { useRoute } from "vue-router";
import Titlebar from "../titlebar/Titlebar.vue";
import Navbar from "../components/navbar/Navbar.vue";
import NavbarItem from "../components/navbar/NavbarItem.vue";
import OptionsModal from "../components/OptionsModal.vue";
import { ref } from "vue";

const route = useRoute();
const anchorEl = ref<HTMLElement | null>(null);
const isOptionsModalOpen = ref(false);
const openOptionsModal = (e: MouseEvent) => {
    anchorEl.value = e.currentTarget as HTMLElement;
    isOptionsModalOpen.value = true;
};
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

        <div
            class="webkit-panel pointer-events-none fixed bottom-5 right-5 z-30 flex h-16 items-center rounded-[26px] border-2 border-[#26382d] bg-[#151815]/95 px-3 shadow-[0_10px_0_#060806,0_18px_28px_rgba(0,0,0,0.38)]"
        >
            <button
                type="button"
                title="Options"
                class="pointer-events-auto group flex h-11 w-fit gap-2 items-center justify-center rounded-[18px] font-bold text-white/72 transition-[color,background-color,box-shadow,transform] duration-150 hover:-translate-y-0.5 hover:text-brand focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40"
                :class="{
                    'bg-brand text-black shadow-[inset_0_-5px_0_rgba(0,0,0,0.22),0_0_18px_rgba(0,255,136,0.22)] hover:text-black':
                        isOptionsModalOpen,
                }"
                @click="openOptionsModal"
            >
                <Settings
                    :size="22"
                    class="shrink-0 transition-colors duration-150"
                />

                Options
            </button>
        </div>

        <OptionsModal
            :is-open="isOptionsModalOpen"
            :anchor-el="anchorEl"
            @close="isOptionsModalOpen = false"
        />
    </div>
</template>
