import { ref } from "vue";

export type ThemePreference = "system" | "dark" | "light";

const STORAGE_KEY = "localcraft-theme";
const themes = new Set<ThemePreference>(["system", "dark", "light"]);
const mediaQuery =
    typeof window !== "undefined"
        ? window.matchMedia("(prefers-color-scheme: dark)")
        : null;

const readStoredTheme = (): ThemePreference => {
    if (typeof localStorage === "undefined") return "system";

    const storedTheme = localStorage.getItem(STORAGE_KEY);
    return themes.has(storedTheme as ThemePreference)
        ? (storedTheme as ThemePreference)
        : "system";
};

export const themePreference = ref<ThemePreference>(readStoredTheme());

const resolveTheme = (preference: ThemePreference) => {
    if (preference === "system") {
        return mediaQuery?.matches ? "dark" : "light";
    }

    return preference;
};

export const applyTheme = () => {
    if (typeof document === "undefined") return;

    const resolvedTheme = resolveTheme(themePreference.value);
    const root = document.documentElement;

    root.classList.toggle("dark", resolvedTheme === "dark");
    root.dataset.theme = themePreference.value;
    root.style.colorScheme = resolvedTheme;
};

export const setThemePreference = (preference: ThemePreference) => {
    themePreference.value = preference;

    if (typeof localStorage !== "undefined") {
        localStorage.setItem(STORAGE_KEY, preference);
    }

    applyTheme();
};

export const initTheme = () => {
    applyTheme();

    mediaQuery?.addEventListener("change", () => {
        if (themePreference.value === "system") {
            applyTheme();
        }
    });
};
