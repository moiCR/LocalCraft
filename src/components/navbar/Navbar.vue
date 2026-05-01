<script setup lang="ts">
import gsap from "gsap";
import { onMounted, onUnmounted, ref } from "vue";

const navRef = ref<HTMLElement | null>(null);
const indicatorRef = ref<HTMLElement | null>(null);

const moveIndicator = (target: HTMLElement) => {
    if (!navRef.value || !indicatorRef.value) return;

    const navRect = navRef.value.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();

    gsap.to(indicatorRef.value, {
        x: targetRect.left - navRect.left,
        y: targetRect.top - navRect.top,
        width: targetRect.width,
        height: targetRect.height,
        opacity: 1,
        duration: 0.42,
        ease: "power3.out",
    });
};

const handleActiveItem = (event: Event) => {
    const target = (event as CustomEvent<HTMLElement>).detail;
    if (target) moveIndicator(target);
};

onMounted(() => {
    window.addEventListener("navbar-active-item", handleActiveItem);
});

onUnmounted(() => {
    window.removeEventListener("navbar-active-item", handleActiveItem);
});
</script>

<template>
    <nav
        ref="navRef"
        class="relative shrink-0 h-20 w-full border-t border-white/10 bg-[#161616]/95 backdrop-blur-md px-6 z-30"
    >
        <div
            ref="indicatorRef"
            class="pointer-events-none absolute left-0 top-0 rounded-3xl bg-green-700 opacity-0"
        />
        <div class="relative z-10 mx-auto flex flex-row h-full max-w-2xl items-center justify-center gap-3">
            <slot />
        </div>
    </nav>
</template>
