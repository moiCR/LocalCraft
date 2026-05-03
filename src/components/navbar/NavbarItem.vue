<script setup lang="ts">
import type { Component } from 'vue';
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRouter } from 'vue-router';
import gsap from 'gsap';

interface Props {
  title: string;
  icon: Component;
  to: string;
  active: boolean;
}

const props = defineProps<Props>();
const router = useRouter();
const itemRef = ref<HTMLElement | null>(null);
const titleRef = ref<HTMLElement | null>(null);

const navigateTo = () => {
    router.push({ name: props.to });
};

const emitActiveItem = () => {
    if (!itemRef.value) return;
    requestAnimationFrame(() => {
        if (!itemRef.value) return;
        window.dispatchEvent(
            new CustomEvent('navbar-active-item', {
                detail: itemRef.value,
            }),
        );
    });
};

const animateTitle = () => {
    if (!titleRef.value) return;
    gsap.fromTo(
        titleRef.value,
        { opacity: 0, x: -6 },
        {
            opacity: 1,
            x: 0,
            duration: 0.22,
            ease: 'power2.out',
        },
    );
};

onMounted(() => {
    if (!itemRef.value) return;
    gsap.fromTo(
        itemRef.value,
        { opacity: 0, y: 10 },
        { opacity: 1, y: 0, duration: 0.35, ease: 'power3.out' },
    );

    if (props.active) {
        nextTick(() => {
            emitActiveItem();
            animateTitle();
        });
    }
});

watch(
    () => props.active,
    (active) => {
        if (!active) return;
        nextTick(() => {
            emitActiveItem();
            animateTitle();
        });
    },
);

onUnmounted(() => {
    if (itemRef.value) gsap.killTweensOf(itemRef.value);
});
</script>

<template>
    <button
        ref="itemRef"
        type="button"
        :title="props.title"
        class="group flex h-11 items-center justify-center gap-2 rounded-[18px] px-3 transition-all duration-300 hover:-translate-y-1 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40"
        :class="{
            'min-w-32 font-black text-black': props.active,
            'w-11 font-bold text-white/72 hover:text-brand': !props.active
        }"
        @click="navigateTo"
    >
        <component 
            :is="props.icon"
            :size="22"
            class="shrink-0 transition-colors duration-300"
        />

        <span
            v-if="props.active"
            ref="titleRef"
            class="whitespace-nowrap text-sm"
        >
            {{ props.title }}
        </span>
    </button>
</template>
