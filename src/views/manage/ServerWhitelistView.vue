<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useRoute } from "vue-router";
import { Loader2, RefreshCw, Search, ShieldCheck, Trash2, User, UserPlus, X } from "@lucide/vue";
import Button from "../../components/ui/Button.vue";
import Modal from "../../components/ui/Modal.vue";

interface WhitelistUser {
    uuid: string;
    name: string;
}

const route = useRoute();
const serverId = computed(() => route.params.id as string);
const showAddUserModal = ref(false);
const addUserButtonRef = ref<HTMLElement | null>(null);
const usernameInputRef = ref<HTMLInputElement | null>(null);
const username = ref("");
const searchQuery = ref("");
const users = ref<WhitelistUser[]>([]);
const isLoading = ref(false);
const isAdding = ref(false);
const removingIds = ref<Set<string>>(new Set());
const error = ref<string | null>(null);
const failedSkinIds = ref<Set<string>>(new Set());

const filteredUsers = computed(() => {
    const query = searchQuery.value.trim().toLowerCase();
    if (!query) return users.value;
    return users.value.filter((user) =>
        user.name.toLowerCase().includes(query) ||
        user.uuid.toLowerCase().includes(query),
    );
});

const cleanUuid = (uuid: string) => uuid.replace(/-/g, "");
const cleanName = (name: string) => encodeURIComponent(name.trim());

const skinUrl = (user: WhitelistUser) =>
    `https://mc-heads.net/body/${cleanName(user.name)}/128`;

const avatarUrl = (user: WhitelistUser) =>
    `https://mc-heads.net/avatar/${cleanName(user.name)}/96`;

const crafatarAvatarUrl = (uuid: string) =>
    `https://crafatar.com/avatars/${cleanUuid(uuid)}?overlay&size=96&default=MHF_Steve`;

const handleSkinError = (event: Event, user: WhitelistUser) => {
    const image = event.currentTarget as HTMLImageElement | null;
    if (!image) return;

    if (!image.dataset.fallback) {
        image.dataset.fallback = "mc-avatar";
        image.src = avatarUrl(user);
        return;
    }

    if (image.dataset.fallback === "mc-avatar") {
        image.dataset.fallback = "crafatar-avatar";
        image.src = crafatarAvatarUrl(user.uuid);
        return;
    }

    failedSkinIds.value = new Set(failedSkinIds.value).add(user.uuid);
};

const loadWhitelist = async () => {
    isLoading.value = true;
    error.value = null;

    try {
        const content = await invoke<string>("read_file", {
            serverId: serverId.value,
            path: "whitelist.json",
        });
        const parsed = JSON.parse(content || "[]");
        users.value = Array.isArray(parsed)
            ? parsed.filter((entry): entry is WhitelistUser =>
                typeof entry?.uuid === "string" && typeof entry?.name === "string",
            )
            : [];
        failedSkinIds.value = new Set();
    } catch (err) {
        const message = String(err);
        if (message.toLowerCase().includes("not found") || message.toLowerCase().includes("no such")) {
            users.value = [];
            return;
        }
        error.value = message;
        users.value = [];
    } finally {
        isLoading.value = false;
    }
};

const openAddUserModal = () => {
    showAddUserModal.value = true;
    window.setTimeout(() => usernameInputRef.value?.focus(), 100);
};

const closeAddUserModal = () => {
    username.value = "";
    showAddUserModal.value = false;
};

const addUser = async () => {
    if (!username.value.trim()) return;
    isAdding.value = true;
    error.value = null;

    try {
        await invoke("send_command", {
            id: serverId.value,
            command: `whitelist add ${username.value.trim()}`,
        });
        closeAddUserModal();
        window.setTimeout(() => {
            void loadWhitelist();
        }, 600);
    } catch (err) {
        error.value = String(err);
    } finally {
        isAdding.value = false;
    }
};

const removeUser = async (user: WhitelistUser) => {
    if (removingIds.value.has(user.uuid)) return;
    removingIds.value = new Set(removingIds.value).add(user.uuid);
    error.value = null;

    try {
        await invoke("send_command", {
            id: serverId.value,
            command: `whitelist remove ${user.name}`,
        });
        window.setTimeout(() => {
            void loadWhitelist();
        }, 600);
    } catch (err) {
        error.value = String(err);
    } finally {
        const next = new Set(removingIds.value);
        next.delete(user.uuid);
        removingIds.value = next;
    }
};

onMounted(loadWhitelist);
</script>

<template>
    <div class="flex h-full w-full flex-col overflow-hidden">
        <section
            class="flex shrink-0 flex-row items-center justify-between gap-2 border-b border-[#26382d] bg-[#151815]/70 px-4 py-3"
        >
            <div
                class="group flex h-11 w-full max-w-md items-center overflow-hidden rounded-2xl border-2 border-[#223127] bg-[#0c0f0d] text-white/80 shadow-[0_4px_0_#060806] transition-all focus-within:border-brand/40 focus-within:ring-4 focus-within:ring-brand/10"
            >
                <div
                    class="flex h-full w-12 shrink-0 items-center justify-center border-r border-white/10 bg-white/[0.03] text-white/35 transition-colors group-focus-within:text-brand"
                >
                    <Search :size="18" />
                </div>
                <input
                    v-model="searchQuery"
                    type="search"
                    class="h-full min-w-0 flex-1 bg-transparent px-3 text-sm text-white outline-none placeholder:text-white/30"
                    placeholder="Search user..."
                />
            </div>

            <div class="flex flex-row">
                <Button
                    tooltip="Refresh whitelist"
                    tooltip-position="bottom"
                    class="ml-2 border border-white/10 bg-white/5 px-3! py-2.5! text-white/70 hover:bg-white/10 hover:text-white"
                    :disabled="isLoading"
                    @click="loadWhitelist"
                >
                    <RefreshCw :size="18" :class="{ 'animate-spin': isLoading }" />
                </Button>
            </div>
        </section>

        <section class="min-h-0 flex-1 overflow-y-auto p-4">
            <div
                v-if="error"
                class="mb-4 rounded-2xl border border-red-400/20 bg-red-500/10 px-4 py-3 text-sm font-semibold text-red-300"
            >
                {{ error }}
            </div>

            <div
                v-if="isLoading"
                class="flex h-full items-center justify-center gap-2 text-sm font-black uppercase tracking-widest text-white/35"
            >
                <Loader2 :size="18" class="animate-spin" />
                Loading whitelist
            </div>

            <div
                v-else
                class="grid grid-cols-2 gap-4 md:grid-cols-3 xl:grid-cols-5"
            >
                <div
                    v-if="filteredUsers.length === 0"
                    class="col-span-full flex items-center justify-center gap-3 rounded-[24px] border-2 border-[#26382d] bg-[#101410] p-5 text-white/35 shadow-[0_6px_0_#060806]"
                >
                    <ShieldCheck :size="20" class="text-brand" />
                    <p class="text-sm font-black uppercase tracking-widest">
                        {{ users.length === 0 ? "No whitelisted users" : "No users found" }}
                    </p>
                </div>

                <article
                    v-for="user in filteredUsers"
                    :key="user.uuid"
                    class="group relative flex min-h-52 flex-col items-center justify-end overflow-hidden rounded-[26px] border-2 border-[#26382d] bg-[#101410] p-4 text-center shadow-[0_7px_0_#060806] transition-all hover:-translate-y-0.5 hover:border-brand/30 hover:bg-[#141b15]"
                >
                    <button
                        type="button"
                        class="absolute right-3 top-3 z-10 flex h-9 w-9 items-center justify-center rounded-xl border border-red-400/15 bg-red-500/10 text-red-300/70 opacity-0 shadow-[0_3px_0_#060806] transition-all hover:border-red-400/35 hover:bg-red-500/15 hover:text-red-200 disabled:cursor-not-allowed disabled:opacity-45 group-hover:opacity-100"
                        title="Remove from whitelist"
                        :disabled="removingIds.has(user.uuid)"
                        @click="removeUser(user)"
                    >
                        <Loader2
                            v-if="removingIds.has(user.uuid)"
                            :size="15"
                            class="animate-spin"
                        />
                        <Trash2
                            v-else
                            :size="15"
                        />
                    </button>
                    <div class="relative flex flex-1 items-end justify-center">
                        <img
                            v-if="!failedSkinIds.has(user.uuid)"
                            :src="skinUrl(user)"
                            :alt="`${user.name} skin`"
                            class="h-32 min-w-20 object-contain drop-shadow-[0_10px_12px_rgba(0,0,0,0.45)] transition-transform duration-300 group-hover:scale-105"
                            loading="lazy"
                            referrerpolicy="no-referrer"
                            @error="handleSkinError($event, user)"
                        />
                        <div
                            v-else
                            class="flex h-24 w-24 items-center justify-center rounded-[22px] border-2 border-[#26382d] bg-[#151815] text-white/35 shadow-[0_5px_0_#060806]"
                        >
                            <User :size="36" />
                        </div>
                    </div>
                    <div class="mt-3 w-full rounded-2xl border border-white/10 bg-white/5 px-3 py-2">
                        <p class="truncate text-sm font-black text-white">
                            {{ user.name }}
                        </p>
                        <p class="mt-0.5 truncate font-mono text-[10px] text-white/25">
                            {{ user.uuid }}
                        </p>
                    </div>
                </article>

                <button
                    ref="addUserButtonRef"
                    type="button"
                    class="group flex min-h-52 flex-col items-center justify-center gap-4 rounded-[26px] border-2 border-dashed border-brand/25 bg-brand/5 p-4 text-center text-brand shadow-[0_7px_0_#060806] transition-all hover:-translate-y-0.5 hover:border-brand/45 hover:bg-brand/10 active:translate-y-0.5 active:shadow-[0_2px_0_#060806]"
                    title="Add user"
                    @click="openAddUserModal"
                >
                    <div
                        class="flex h-16 w-16 items-center justify-center rounded-[22px] border-2 border-brand/35 bg-brand/10 shadow-[inset_0_-4px_0_rgba(0,0,0,0.22)] transition-transform group-hover:scale-105"
                    >
                        <UserPlus :size="28" />
                    </div>
                    <div>
                        <p class="text-sm font-black text-white">Add User</p>
                        <p class="mt-1 text-xs font-semibold text-white/35">
                            Add to whitelist
                        </p>
                    </div>
                </button>
            </div>
        </section>

        <Modal
            :is-open="showAddUserModal"
            :anchor-el="(addUserButtonRef as any)?.$el ?? addUserButtonRef"
            layout-id="whitelist-add-user-modal"
            @close="closeAddUserModal"
        >
            <div class="relative flex w-full max-w-[340px] flex-col gap-6 p-6">
                <header class="relative flex flex-col items-center justify-center pt-2">
                    <button
                        class="absolute -left-1 -top-1 rounded-full p-2 transition-colors hover:bg-white/5 group"
                        @click="closeAddUserModal"
                    >
                        <X :size="18" class="text-white/60 group-hover:text-white" />
                    </button>

                    <h2 class="text-xl font-black tracking-tight text-white">
                        Add User
                    </h2>
                </header>

                <div class="flex flex-col gap-6">
                    <div class="flex flex-col gap-2">
                        <label class="ml-1 text-xs font-semibold uppercase tracking-widest text-white/40">
                            Username
                        </label>
                        <div class="relative group">
                            <div
                                class="absolute left-4 top-1/2 -translate-y-1/2 text-white/40 transition-colors group-focus-within:text-brand"
                            >
                                <User :size="18" />
                            </div>
                            <input
                                ref="usernameInputRef"
                                v-model="username"
                                type="text"
                                placeholder="Minecraft username"
                                class="w-full rounded-xl border border-white/10 bg-white/5 py-3.5 pl-12 pr-4 font-mono text-sm text-white outline-none transition-all placeholder-white/20 focus:border-brand/50 focus:ring-4 focus:ring-brand/10"
                                @keyup.enter="addUser"
                            />
                        </div>
                    </div>

                    <Button
                        class="w-full rounded-xl border-2 border-brand/70 bg-brand py-3.5 text-sm font-black text-black shadow-[0_4px_0_#060806] transition-all hover:bg-brand/90 active:translate-y-0.5 active:shadow-[0_1px_0_#060806]"
                        :disabled="isAdding || !username.trim()"
                        @click="addUser"
                    >
                        <span class="inline-flex items-center justify-center gap-2">
                            <Loader2 v-if="isAdding" :size="14" class="animate-spin" />
                            {{ isAdding ? "Adding..." : "Add to Whitelist" }}
                        </span>
                    </Button>
                </div>
            </div>
        </Modal>
    </div>
</template>
