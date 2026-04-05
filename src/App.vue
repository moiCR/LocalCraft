<script setup lang="ts">
import { RouterView, useRouter } from "vue-router";
import "./css/global.css";
import { onMounted, onUnmounted } from "vue";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

const router = useRouter();
let unlisten: UnlistenFn | null = null;

onMounted(async () => {
    unlisten = await listen<string>('navigate', (event) => {
        router.push('/' + event.payload);
    });
});

onUnmounted(() => {
    if (unlisten) unlisten();
});
</script>

<template>
    <RouterView />
</template>

