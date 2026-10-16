import { SignalPreset } from "@/data";
import { BASE_URL } from "@/utils";
import { gunzipSync, gzipSync } from "fflate";

const DB_VERSION = 2;

const APP_STORAGE_KEY = "app-storage";
const FILE_STORAGE_KEY = "file-storage";
const SIGNAL_PRESETS_STORAGE_KEY = "signal-data-presets";

const PRESET_MAP = {
  "compat-v2.0.77": { url: `${BASE_URL}assets/data/presets/compat-v2.0.77.json` },
  "dlc-v2.0.77": { url: `${BASE_URL}assets/data/presets/space-age-v2.0.77.json` },
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
  console.log("getSignalPresetNames");
  return new Promise((resolve, reject) => {
    const tx = db.transaction(SIGNAL_PRESETS_STORAGE_KEY, "readonly");
    const req = tx.objectStore(SIGNAL_PRESETS_STORAGE_KEY).getAllKeys();
    req.onsuccess = () => resolve(req.result as string[]);
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
export async function getSignalPreset(name: string): Promise<{ preset: SignalPreset; raw: Uint8Array }> {
  if (name in PRESET_MAP) {
    const { url } = PRESET_MAP[name as keyof typeof PRESET_MAP];
    const req = await fetch(url);
    return {
      preset: await req.clone().json(),
      raw: new Uint8Array(await req.arrayBuffer()),
    };
  }

  const db = await fileDBPromise;
  return new Promise((resolve, reject) => {
    try {
      const tx = db.transaction(SIGNAL_PRESETS_STORAGE_KEY, "readonly");
      const req = tx.objectStore(SIGNAL_PRESETS_STORAGE_KEY).get(name);
      req.onsuccess = () => {
        try {
          const raw = gunzipSync(req.result);
          const preset: SignalPreset = JSON.parse(new TextDecoder().decode(raw));
          resolve({ raw, preset });
        } catch (e) {
          reject(e);
        }
      };
      req.onerror = () => reject(req.error);
    } catch (e) {
      reject(e);
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
