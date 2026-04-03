<script setup lang="ts">
import { ref } from 'vue';

interface Props {
  tooltip?: string;
  tooltipPosition?: 'top' | 'bottom' | 'left' | 'right';
  disabled?: boolean;
  showButton?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  tooltipPosition: 'top',
  disabled: false,
  showButton: true
});

const hovered = ref(false);
</script>

<script lang="ts">
export default {
  inheritAttrs: false
}
</script>

<template>
  <div 
    class="relative inline-block"
    @mouseenter="hovered = true"
    @mouseleave="hovered = false"
  >
    <button
      v-if="showButton"
      :disabled="disabled"
      v-bind="$attrs"
      :class="[
        'relative z-10 cursor-pointer font-bold text-white dark:text-black bg-black dark:bg-white px-4 py-2 rounded-xl transition-all duration-300 ease-[cubic-bezier(0.34,1.56,0.64,1)]',
        disabled ? 'opacity-50 cursor-not-allowed' : 'hover:scale-x-105 hover:scale-y-[0.95]',
        $attrs.class
      ]"
      style="border-radius: 12px;"
    >
      <span><slot /></span>
    </button>

    <Transition
      enter-active-class="transition ease-out duration-150"
      enter-from-class="opacity-0 translate-y-1 scale-95"
      enter-to-class="opacity-100 translate-y-0 scale-100"
      leave-active-class="transition ease-in duration-100"
      leave-from-class="opacity-100 translate-y-0 scale-100"
      leave-to-class="opacity-0 translate-y-1 scale-95"
    >
      <div
        v-if="tooltip && hovered"
        :class="[
          'absolute pointer-events-none z-100 whitespace-nowrap rounded-lg bg-neutral-900 dark:bg-neutral-100 px-3 py-1.5 text-xs font-medium text-white dark:text-black shadow-lg',
          tooltipPosition === 'top' ? 'bottom-full left-1/2 -translate-x-1/2 mb-2' : '',
          tooltipPosition === 'bottom' ? 'top-full left-1/2 -translate-x-1/2 mt-2' : '',
          tooltipPosition === 'left' ? 'right-full top-1/2 -translate-y-1/2 mr-2' : '',
          tooltipPosition === 'right' ? 'left-full top-1/2 -translate-y-1/2 ml-2' : ''
        ]"
      >
        {{ tooltip }}
      </div>
    </Transition>
  </div>
</template>
