import { reactive } from "vue";
import { listen } from "@tauri-apps/api/event";

export interface LogPayload {
    id: string;
    line: string;
}

export const globalLogsCache = reactive<Record<string, string[]>>({});
let isListening = false;

export const startGlobalListener = async () => {
    if (isListening) return;
    isListening = true;

    await listen<LogPayload>('server-log', (event) => {
        const { id, line } = event.payload;

        if (!globalLogsCache[id]) {
            globalLogsCache[id] = [];
        }

        globalLogsCache[id].push(line);

        if (globalLogsCache[id].length > 500) {
            globalLogsCache[id].shift();
        }
    });
};