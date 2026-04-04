<script setup lang="ts">
import { PlayIcon, Rotate3D, SquareIcon, ChevronRight } from '@lucide/vue';
import Button from '../../components/ui/Button.vue';


</script>

<template>
    <div class="flex flex-col gap-2 w-full h-full">
        <section class="flex flex-row p-4 border-b border-white/10 gap-2">
            <Button :tooltip="'Start server'" :tooltip-position="'bottom'" :class="'bg-transparent dark:bg-green-500/10 hover:bg-green-500/10 dark:hover:bg-green-500/10'">
                <PlayIcon :class="'text-green-500'" :size="16" />
            </Button>
            <Button :tooltip="'Stop server'" :tooltip-position="'bottom'" :class="'bg-transparent dark:bg-red-500/10 hover:bg-red-500/10 dark:hover:bg-red-500/10'">
                <SquareIcon :class="'text-red-500'" :size="16" />
            </Button>
            <Button :tooltip="'Restart server'" :tooltip-position="'bottom'" :class="'bg-transparent dark:bg-yellow-500/10 hover:bg-yellow-500/10 dark:hover:bg-yellow-500/10'">
                <Rotate3D :class="'text-yellow-500'" :size="16" />
            </Button>
        </section>

        <section class="flex flex-col gap-4 w-full h-full p-4 overflow-hidden">
            <!-- Log console -->
            <div class="flex-1 bg-[#0d0d0d] border border-white/5 rounded-xl p-6 overflow-y-auto flex flex-col gap-1.5 font-mono text-xs text-white/70 custom-scrollbar">

            </div>
            
            <!-- Command console -->
            <div class="relative group">
                <div class="absolute left-4 top-1/2 -translate-y-1/2 text-white/20 font-mono text-sm group-focus-within:text-brand/40 transition-colors">
                    <ChevronRight :size="16" />
                </div>
                <input
                    type="text"
                    placeholder="Type a command..."
                    class="w-full bg-[#0d0d0d] border border-white/5 rounded-xl py-4 pl-10 pr-4 font-mono text-white/90 text-sm outline-none focus:border-white/10 focus:ring-1 focus:ring-white/5 transition-all disabled:opacity-50"
                />
            </div>
        </section>
    </div>
</template>