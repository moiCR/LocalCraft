<script setup lang="ts">
import { ref } from "vue";

interface Props {
    tooltip?: string;
    tooltipPosition?: "top" | "bottom" | "left" | "right";
    disabled?: boolean;
    showButton?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
    tooltipPosition: "top",
    disabled: false,
    showButton: true,
});

const hovered = ref(false);
</script>

<script lang="ts">
export default {
    inheritAttrs: false,
};
</script>

<template>
    <div
        class="relative inline-block"
        @mouseenter="hovered = true"
        @mouseleave="hovered = false"
    >
        <button
            v-if="props.showButton"
            :disabled="props.disabled"
            v-bind="$attrs"
            :class="[
                'relative z-10 cursor-pointer font-bold px-4 py-2 rounded-xl transition-[color,background-color,border-color,box-shadow,transform,opacity] duration-150 ease-out',
                !($attrs.class as string)?.includes('bg-')
                    ? 'text-white/80 bg-white/5 hover:bg-white/10'
                    : '',
                props.disabled
                    ? 'opacity-50 cursor-not-allowed'
                    : 'hover:-translate-y-0.5 active:translate-y-0',
                $attrs.class,
            ]"
            style="border-radius: 12px"
        >
            <slot />
        </button>

        <Transition
            enter-active-class="transition-[opacity,transform] ease-out duration-100"
            enter-from-class="opacity-0 translate-y-1"
            enter-to-class="opacity-100 translate-y-0"
            leave-active-class="transition-[opacity,transform] ease-in duration-75"
            leave-from-class="opacity-100 translate-y-0"
            leave-to-class="opacity-0 translate-y-1"
        >
            <div
                v-if="props.tooltip && hovered"
                :class="[
                    'absolute pointer-events-none z-100 whitespace-nowrap rounded-lg border border-[#c7d6c8] bg-[#fbfdf8] px-3 py-1.5 text-xs font-bold text-[#253126] shadow-[0_4px_0_#b9c7b8,0_10px_22px_rgba(38,56,45,0.14)] dark:border-white/10 dark:bg-[#1b1b1b] dark:text-white/80 dark:shadow-lg',
                    props.tooltipPosition === 'top'
                        ? 'bottom-full left-1/2 -translate-x-1/2 mb-2'
                        : '',
                    props.tooltipPosition === 'bottom'
                        ? 'top-full left-1/2 -translate-x-1/2 mt-2'
                        : '',
                    props.tooltipPosition === 'left'
                        ? 'right-full top-1/2 -translate-y-1/2 mr-2'
                        : '',
                    props.tooltipPosition === 'right'
                        ? 'left-full top-1/2 -translate-y-1/2 ml-2'
                        : '',
                ]"
            >
                {{ props.tooltip }}
            </div>
        </Transition>
    </div>
</template>
