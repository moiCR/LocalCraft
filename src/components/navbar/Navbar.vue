<script setup lang="ts">
import gsap from "gsap";
import { onMounted, onUnmounted, ref } from "vue";

const navRef = ref<HTMLElement | null>(null);
const indicatorRef = ref<HTMLElement | null>(null);
const activeTarget = ref<HTMLElement | null>(null);

let resizeFrame: number | null = null;
let resizeObserver: ResizeObserver | null = null;

const moveIndicator = (target: HTMLElement, animate = true) => {
    const nav = navRef.value;
    const indicator = indicatorRef.value;

    if (!nav || !indicator) return;
    if (!target.isConnected) return;

    const navRect = nav.getBoundingClientRect();
    const targetRect = target.getBoundingClientRect();

    const x = Math.round(targetRect.left - navRect.left);
    const y = Math.round(targetRect.top - navRect.top);
    const width = Math.round(targetRect.width);
    const height = Math.round(targetRect.height);

    // Cambiar tamaño directo: evita animar layout.
    gsap.set(indicator, {
        width,
        height,
        opacity: 1,
    });

    gsap.killTweensOf(indicator);

    if (!animate) {
        gsap.set(indicator, {
            x,
            y,
            force3D: true,
        });
        return;
    }

    // Animar solo transform: mucho más barato.
    gsap.to(indicator, {
        x,
        y,
        duration: 0.22,
        ease: "power3.out",
        force3D: true,
        overwrite: true,
    });
};

const handleActiveItem = (event: Event) => {
    const target = (event as CustomEvent<HTMLElement>).detail;
    if (!target) return;

    activeTarget.value = target;
    moveIndicator(target, true);
};

const syncIndicator = () => {
    if (resizeFrame !== null) cancelAnimationFrame(resizeFrame);

    resizeFrame = requestAnimationFrame(() => {
        resizeFrame = null;

        if (activeTarget.value) {
            moveIndicator(activeTarget.value, false);
        }
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

    if (resizeFrame !== null) {
        cancelAnimationFrame(resizeFrame);
        resizeFrame = null;
    }

    if (indicatorRef.value) {
        gsap.killTweensOf(indicatorRef.value);
    }
});
</script>

<template>
    <nav
        ref="navRef"
        class="webkit-panel pointer-events-none fixed bottom-5 left-1/2 z-30 h-16 w-fit -translate-x-1/2 rounded-[26px] border-2 border-[#26382d] bg-[#151815]/95 px-3 shadow-[0_10px_0_#060806,0_18px_28px_rgba(0,0,0,0.38)]"
    >
        <div
            ref="indicatorRef"
            class="navbar-indicator pointer-events-none absolute left-0 top-0 rounded-[22px] border-2 border-brand/80 bg-brand opacity-0 shadow-[inset_0_-5px_0_rgba(0,0,0,0.22),0_0_18px_rgba(0,255,136,0.22)]"
        />

        <div
            class="pointer-events-auto relative z-10 flex h-full items-center justify-center gap-2"
        >
            <slot />
        </div>
    </nav>
</template>

<style scoped>
.navbar-indicator {
    transform: translate3d(0, 0, 0);
    will-change: transform;
    contain: layout paint style;
    backface-visibility: hidden;
}
</style>
