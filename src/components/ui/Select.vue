<script setup lang="ts">
import { ref, computed, watch, onUnmounted, nextTick } from 'vue';
import { ChevronDown, Check } from '@lucide/vue';

export interface SelectOption {
    label: string | number;
    value: string;
}

const props = defineProps<{
    options: SelectOption[];
    modelValue?: string;
    label?: string;
    placeholder?: string;
    class?: string | object | Array<any>;
    disabled?: boolean;
}>();

const emit = defineEmits<{
    (e: 'update:modelValue', value: string): void;
}>();

const isOpen = ref(false);
const selectRef = ref<HTMLDivElement | null>(null);
const triggerRef = ref<HTMLDivElement | null>(null);
const dropdownPos = ref<{
    left: number;
    width: number;
    top?: number;
    bottom?: number;
    openAbove: boolean;
} | null>(null);

const selectedOption = computed(() => 
    props.options.find(opt => String(opt.value) === String(props.modelValue))
);

const updatePosition = () => {
    if (!triggerRef.value) return;
    const rect = triggerRef.value.getBoundingClientRect();
    const spaceBelow = window.innerHeight - rect.bottom;
    const openAbove = spaceBelow < 260 && rect.top > spaceBelow;

    dropdownPos.value = {
        left: rect.left,
        width: rect.width,
        openAbove,
        ...(openAbove 
            ? { bottom: window.innerHeight - rect.top + 6 }
            : { top: rect.bottom + 6 })
    };
};

const handleClickOutside = (event: MouseEvent) => {
    if (selectRef.value && !selectRef.value.contains(event.target as Node)) {
        isOpen.value = false;
    }
};

const handleToggle = () => {
    if (props.disabled) return;
    if (!isOpen.value) updatePosition();
    isOpen.value = !isOpen.value;
};

const selectOption = (val: string) => {
    emit('update:modelValue', val);
    isOpen.value = false;
};

watch(isOpen, (newVal) => {
    if (newVal) {
        nextTick(updatePosition);
        window.addEventListener('scroll', updatePosition, true);
        window.addEventListener('resize', updatePosition);
        document.addEventListener('mousedown', handleClickOutside);
    } else {
        window.removeEventListener('scroll', updatePosition, true);
        window.removeEventListener('resize', updatePosition);
        document.removeEventListener('mousedown', handleClickOutside);
    }
});

onUnmounted(() => {
    window.removeEventListener('scroll', updatePosition, true);
    window.removeEventListener('resize', updatePosition);
    document.removeEventListener('mousedown', handleClickOutside);
});
</script>

<template>
    <div
        :class="['relative flex flex-col gap-1.5 w-full', props.class]"
        ref="selectRef"
    >
        <label v-if="label" class="text-xs font-medium text-gray-500 dark:text-gray-400">
            {{ label }}
        </label>
        <div
            ref="triggerRef"
            @click="handleToggle"
            :class="[
                'flex items-center justify-between px-3 py-2 rounded-lg border transition-all shadow-sm',
                disabled 
                    ? 'opacity-50 cursor-not-allowed bg-black/5 border-black/5 dark:bg-white/5 dark:border-white/5 text-gray-500'
                    : 'cursor-pointer border-black/5 bg-black/2 hover:bg-black/5 dark:border-white/5 dark:bg-white/2 dark:hover:bg-white/5 text-black dark:text-white',
                isOpen && !disabled ? 'ring-1 ring-black/20 dark:ring-white/20' : ''
            ]"
        >
            <span :class="['truncate text-sm', !selectedOption && 'text-gray-500']">
                {{ selectedOption ? selectedOption.label : placeholder || 'Select an option' }}
            </span>
            <div :class="['transition-transform duration-200', isOpen ? 'rotate-180' : '']">
                <ChevronDown class="w-4 h-4 text-gray-400" />
            </div>
        </div>

        <Teleport to="body">
            <Transition name="dropdown">
                <div
                    v-if="isOpen && dropdownPos"
                    :style="[
                        {
                            position: 'fixed',
                            left: `${dropdownPos.left}px`,
                            width: `${dropdownPos.width}px`,
                            '--dropdown-y': dropdownPos.openAbove ? '6px' : '-6px'
                        },
                        dropdownPos.openAbove ? { bottom: `${dropdownPos.bottom}px` } : { top: `${dropdownPos.top}px` }
                    ]"
                    class="z-100 bg-white dark:bg-[#1e1e1e] border border-black/10 dark:border-white/10 rounded-xl shadow-xl overflow-hidden"
                >
                    <div class="max-h-60 overflow-y-auto p-1.5 custom-scrollbar">
                        <div
                            v-if="options.length === 0"
                            class="p-2.5 px-3 text-sm text-gray-500 text-center"
                        >
                            Loading...
                        </div>
                        <div
                            v-for="option in options"
                            :key="option.value"
                            @click="selectOption(option.value)"
                            :class="[
                                'flex items-center justify-between p-2.5 px-3 cursor-pointer rounded-lg text-sm transition-colors',
                                'hover:bg-blue-500/10 hover:text-blue-600 dark:hover:text-blue-400',
                                modelValue === option.value
                                    ? 'bg-blue-50 dark:bg-blue-500/20 text-blue-600 dark:text-blue-400 font-medium'
                                    : 'text-gray-600 dark:text-gray-300'
                            ]"
                        >
                            {{ option.label }}
                            <Check v-if="modelValue === option.value" class="w-4 h-4" />
                        </div>
                    </div>
                </div>
            </Transition>
        </Teleport>
    </div>
</template>

<style scoped>
.dropdown-enter-active,
.dropdown-leave-active {
    transition: all 0.15s ease-out;
}
.dropdown-enter-from,
.dropdown-leave-to {
    opacity: 0;
    transform: scale(0.95) translateY(var(--dropdown-y, 0px));
}
.dropdown-enter-to,
.dropdown-leave-from {
    opacity: 1;
    transform: scale(1) translateY(0);
}
</style>
