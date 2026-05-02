<script setup lang="ts">
import gsap from "gsap";
import { onMounted, onUnmounted, ref } from "vue";

const navRef = ref<HTMLElement | null>(null);
const indicatorRef = ref<HTMLElement | null>(null);
const activeTarget = ref<HTMLElement | null>(null);
let resizeFrame: number | null = null;
let resizeObserver: ResizeObserver | null = null;

const moveIndicator = (target: HTMLElement, animate = true) => {
    if (!navRef.value || !indicatorRef.value) return;
    if (!target.isConnected) return;

    const navRect = navRef.value.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();
    const position = {
        x: targetRect.left - navRect.left,
        y: targetRect.top - navRect.top,
        width: targetRect.width,
        height: targetRect.height,
        opacity: 1,
    };

    if (!animate) {
        gsap.set(indicatorRef.value, position);
        return;
    }

    gsap.to(indicatorRef.value, {
        ...position,
        duration: 0.42,
        ease: "power3.out",
    });
};

const handleActiveItem = (event: Event) => {
    const target = (event as CustomEvent<HTMLElement>).detail;
    if (!target) return;

    activeTarget.value = target;
    moveIndicator(target);
};

const syncIndicator = () => {
    if (resizeFrame !== null) cancelAnimationFrame(resizeFrame);

    resizeFrame = requestAnimationFrame(() => {
        resizeFrame = null;
        if (activeTarget.value) moveIndicator(activeTarget.value, false);
    });
};

onMounted(() => {
    window.addEventListener("navbar-active-item", handleActiveItem);
    window.addEventListener("resize", syncIndicator);

    if (navRef.value) {
        resizeObserver = new ResizeObserver(syncIndicator);
        resizeObserver.observe(navRef.value);
    }
});

onUnmounted(() => {
    window.removeEventListener("navbar-active-item", handleActiveItem);
    window.removeEventListener("resize", syncIndicator);
    resizeObserver?.disconnect();
    if (resizeFrame !== null) cancelAnimationFrame(resizeFrame);
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
