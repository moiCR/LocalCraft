import { reactive } from "vue";
import { listen } from "@tauri-apps/api/event";

export interface LogPayload {
    id: string;
    lines: string[];
}

const MAX_LOG_LINES = 500;

export const globalLogsCache = reactive<Record<string, string[]>>({});
let isListening = false;

const trimLogBuffer = (logs: string[]) => {
    if (logs.length > MAX_LOG_LINES) {
        logs.splice(0, logs.length - MAX_LOG_LINES);
    }
};

const appendLogBatch = (id: string, lines: string[]) => {
    const nextLogs = (globalLogsCache[id] || []).concat(lines);
    trimLogBuffer(nextLogs);
    globalLogsCache[id] = nextLogs;
};

export const startGlobalListener = async () => {
    if (isListening) return;
    isListening = true;

    await listen<LogPayload>('server-log', (event) => {
        const { id, lines } = event.payload;
        appendLogBatch(id, lines);
    });
};
