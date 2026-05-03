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
        class="pointer-events-none fixed bottom-5 left-1/2 z-30 h-16 w-fit -translate-x-1/2 rounded-[26px] border-2 border-[#26382d] bg-[#151815]/92 px-3 shadow-[0_10px_0_#060806,0_18px_28px_rgba(0,0,0,0.38)] backdrop-blur-xl"
    >
        <div
            ref="indicatorRef"
            class="pointer-events-none absolute left-0 top-0 rounded-[22px] border-2 border-brand/80 bg-brand opacity-0 shadow-[inset_0_-5px_0_rgba(0,0,0,0.22),0_0_18px_rgba(0,255,136,0.22)]"
        />
        <div class="pointer-events-auto relative z-10 flex h-full items-center justify-center gap-2">
            <slot />
        </div>
    </nav>
</template>
