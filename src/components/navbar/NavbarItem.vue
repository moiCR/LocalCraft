<script setup lang = "ts">
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

const navigateTo = () =>{
    router.push({name : props.to})
}

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
    <div
        ref="itemRef"
        class="group flex flex-row gap-2 items-center p-3 cursor-pointer rounded-3xl transition-all duration-300" 
        :class="{
            'text-green-300' : props.active,
            'text-white' : !props.active
        }"
        @click="navigateTo"
    >

        <component 
            :is="props.icon"
            :size = "22"
            class="transition-colors duration-300"
            :class="props.active ? 'text-green-300' : 'text-white group-hover:text-green-300' "
        />

        <span
            v-if="props.active"
            ref="titleRef"
            class="text-sm font-bold whitespace-nowrap"
        >
            {{ props.title }}
        </span>
    </div>
</template>
