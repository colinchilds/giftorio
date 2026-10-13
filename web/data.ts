import FORM_ELEMENTS from "@/formElements.json";
import _INITIAL_VALUES from "@/initialValues.json";
import { Ajv, JSONSchemaType } from "ajv";

export { FORM_ELEMENTS };

// Constants
export const DEFAULT_SIGNAL_PRESET = "base-2.0.77";
export const LAST_FILE_KEY = "last-file-user-uploaded";
export const FORM_DATA_KEY = "giftorio-form-data";
export const SHOW_ADVANCED_KEY = "giftorio-form-data-show-advanced";
export const CURR_SIGNAL_PRESET_KEY = "giftorio-curr-signal-preset-key";

export interface SignalData {
  signals: { type: string; name: string }[];
  qualities: string[];
}
export interface SignalPreset extends SignalData {
  description: string;
}
const SIGNAL_DATA_SCHEMA = {
  type: "object",
  properties: {
    signals: {
      type: "array",
      items: {
        type: "object",
        properties: {
          type: { type: "string" },
          name: { type: "string" },
        },
        required: ["type", "name"],
      },
    },
    qualities: {
      type: "array",
      items: { type: "string" },
    },
  },
  required: ["signals", "qualities"],
  additionalProperties: false,
} satisfies JSONSchemaType<SignalData>;
const ajv = new Ajv();

export function setInitialValues(values: object) {
  localStorage.setItem(FORM_DATA_KEY, JSON.stringify(values));
}
export function getInitialValues(): Omit<typeof _INITIAL_VALUES, "substationQuality"> & { substationQuality: string | null } {
  const prev = localStorage.getItem(FORM_DATA_KEY);

  if (prev) {
    const cached = JSON.parse(prev);
    let changed = false;

    for (const k of Object.keys(cached)) {
      if (!(k in _INITIAL_VALUES)) {
        delete cached[k];
        changed = true;
      }
    }
    for (const [k, v] of Object.entries(_INITIAL_VALUES)) {
      if (!(k in cached)) {
        cached[k] = v;
        changed = true;
      }
    }

    if (changed) {
      setInitialValues(cached);
    }
    return cached;
  }

  setInitialValues(_INITIAL_VALUES);
  return { ..._INITIAL_VALUES };
}

export const validateSignalData = ajv.compile(SIGNAL_DATA_SCHEMA);
