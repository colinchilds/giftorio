import { SignalData, SignalPreset } from "@/data";
import { gunzipSync, gzipSync } from "fflate";

const DB_VERSION = 2;

const APP_STORAGE_KEY = "app-storage";
const FILE_STORAGE_KEY = "file-storage";
const SIGNAL_PRESETS_STORAGE_KEY = "signal-data-presets";

const PRESET_MAP = {
  "compat-v2.0.77": {
    import: () => import("@/assets/data/presets/compat-v2.0.77.json") satisfies Promise<SignalData>,
    description: "Signals from the base game (v2.0.77). Excludes 'satellite' signals for upwards compatibility with Space Age.",
  },
  "dlc-v2.0.77": {
    import: () => import("@/assets/data/presets/space-age-v2.0.77.json") satisfies Promise<SignalData>,
    description: "Signals from the Space Age DLC (v2.0.77).",
  },
} as const;
export const DEFAULT_SIGNAL_PRESET_KEYS = Object.keys(PRESET_MAP);

const fileDBPromise: Promise<IDBDatabase> = new Promise((resolve, reject) => {
  const req = indexedDB.open(APP_STORAGE_KEY, DB_VERSION);
  req.onupgradeneeded = () => {
    const { result } = req;
    result.createObjectStore(FILE_STORAGE_KEY);
    result.createObjectStore(SIGNAL_PRESETS_STORAGE_KEY);
  };
  req.onsuccess = () => resolve(req.result);
  req.onerror = () => reject(req.error);
});

export async function saveFileDB(file: File | null, key: string): Promise<void> {
  const db = await fileDBPromise;
  return new Promise((resolve, reject) => {
    const tx = db.transaction(FILE_STORAGE_KEY, "readwrite");
    tx.objectStore(FILE_STORAGE_KEY).put(file, key);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
  });
}

export async function loadFileDB(key: string): Promise<File | null> {
  const db = await fileDBPromise;
  return new Promise((resolve, reject) => {
    try {
      const req = db.transaction(FILE_STORAGE_KEY, "readonly").objectStore(FILE_STORAGE_KEY).get(key);
      req.onsuccess = () => resolve(req.result ?? null);
      req.onerror = () => reject(req.error);
    } catch (e) {
      resolve(null);
    }
  });
}

export async function getSignalPresetNames(): Promise<string[]> {
  const db = await fileDBPromise;
  return new Promise((resolve, reject) => {
    const tx = db.transaction(SIGNAL_PRESETS_STORAGE_KEY, "readonly");
    const req = tx.objectStore(SIGNAL_PRESETS_STORAGE_KEY).getAllKeys();
    req.onsuccess = () => resolve((console.log(req.result), req.result as string[]));
    req.onerror = () => reject(tx.error);
  });
}
export async function addSignalPreset(name: string, preset: SignalPreset): Promise<void> {
  const db = await fileDBPromise;
  return new Promise((resolve, reject) => {
    const tx = db.transaction(SIGNAL_PRESETS_STORAGE_KEY, "readwrite");
    const compressed = gzipSync(new TextEncoder().encode(JSON.stringify(preset)), { level: 9 });
    const req = tx.objectStore(SIGNAL_PRESETS_STORAGE_KEY).put(compressed, name);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(req.error);
  });
}
export async function getSignalPreset(name: string): Promise<SignalPreset | null> {
  if (name in PRESET_MAP) {
    const { description, import: get } = PRESET_MAP[name as keyof typeof PRESET_MAP];
    return { description, ...(await get()) } satisfies SignalPreset;
  }

  const db = await fileDBPromise;
  return new Promise((resolve, reject) => {
    try {
      const tx = db.transaction(SIGNAL_PRESETS_STORAGE_KEY, "readonly");
      const req = tx.objectStore(SIGNAL_PRESETS_STORAGE_KEY).get(name);
      req.onsuccess = () => {
        try {
          const result = JSON.parse(new TextDecoder().decode(gunzipSync(req.result)));
          resolve(result);
        } catch (e) {
          throw e;
          reject(e);
        }
      };
      req.onerror = () => reject(req.error);
    } catch (e) {
      resolve(null);
    }
  });
}
export async function removeSignalPreset(name: string): Promise<void> {
  const db = await fileDBPromise;
  return new Promise((resolve, reject) => {
    const tx = db.transaction(SIGNAL_PRESETS_STORAGE_KEY, "readwrite");
    const req = tx.objectStore(SIGNAL_PRESETS_STORAGE_KEY).delete(name);
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(req.error);
  });
}
