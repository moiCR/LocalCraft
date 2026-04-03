<script setup lang="ts">
    import LogoIcon from '../assets/logo.svg?component';
</script>

<template>
    <div class="flex flex-row justify-between h-full w-full p-16 gap-16 animate-in fade-in duration-800">
        <section class="flex flex-col gap-12 w-1/2 max-w-xl">
            <!-- Header Section -->
            <section class="flex flex-col gap-6">
                <h1 class="text-6xl font-black tracking-tightest text-brand">About</h1>
                <div class="flex flex-col gap-4">
                    <p class="text-white/60 leading-relaxed text-lg">
                        LocalCraft is a desktop application designed to manage Minecraft servers locally. 
                        Inspired by professional panels like Pterodactyl, it brings high-performance management 
                        right to your machine.
                    </p>
                    <p class="text-white/30 text-sm italic">
                        This project is currently under active development. You can contribute to the source code on GitHub.
                    </p>
                </div>
            </section>

            <!-- Simple Feature Cards -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div class="group p-5 rounded-3xl bg-white/3 border border-white/5 hover:border-brand/20 transition-all duration-300">
                    <h3 class="text-brand text-xs font-bold uppercase tracking-widest mb-1">Core Engine</h3>
                    <p class="text-white/60 text-xs leading-snug">Powered by Rust & Tauri for zero-overhead execution.</p>
                </div>
                <div class="group p-5 rounded-3xl bg-white/3 border border-white/5 hover:border-brand/20 transition-all duration-300">
                    <h3 class="text-brand text-xs font-bold uppercase tracking-widest mb-1">Frontend</h3>
                    <p class="text-white/60 text-xs leading-snug">Built with Vue 3 and Vite for a seamless experience.</p>
                </div>
                <div class="group p-5 rounded-3xl bg-white/3 border border-white/5 hover:border-brand/20 transition-all duration-300">
                    <h3 class="text-brand text-xs font-bold uppercase tracking-widest mb-1">Process Mgmt</h3>
                    <p class="text-white/60 text-xs leading-snug">Isolated and reliable management inspired by Pterodactyl.</p>
                </div>
                <div class="group p-5 rounded-3xl bg-white/3 border border-white/5 hover:border-brand/20 transition-all duration-300">
                    <h3 class="text-brand text-xs font-bold uppercase tracking-widest mb-1">Java Env</h3>
                    <p class="text-white/60 text-xs leading-snug">Automated Java versioning for every server instance.</p>
                </div>
            </div>

            <!-- Detailed Info Section (Now aligned to the SAME margin) -->
            <div class="flex flex-col gap-6 pt-10 border-t border-white/5">
                <p class="text-white/50 text-sm leading-relaxed italic">This project was born from a Minecraft player and developer who started out creating plugins and eventually built this tool to manage his own local servers easily.</p>
                <div class="flex flex-col gap-4">
                    <p class="text-white/40 text-xs leading-relaxed">The main purpose of the app is to save me the trouble of installing various Java versions, server JARs, and other components so I can test plugins or mods locally before deploying them to a server.</p>
                    <p class="text-white/40 text-xs leading-relaxed">LocalCraft is under active development, so expect new features and constant improvements in the near future.</p>
                </div>
            </div>
        </section>

        <section class="max-w-[40%] flex justify-center items-center">
            <LogoIcon class="w-full h-auto opacity-10 drop-shadow-2xl hover:opacity-100 transition-all duration-300 transform hover:scale-105" />
        </section>
    </div>
</template>

<style scoped>
.tracking-tightest {
    letter-spacing: -0.05em;
}
</style>
