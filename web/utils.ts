export function formatDuration(totalMs: number): string {
  const totalSeconds = Math.floor(totalMs / 1000);

  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  const s = totalSeconds % 60;
  const ms = totalMs % 1000;

  return [h, m, s].map((v) => `${v}`.padStart(2, "0")).join(":") + "." + `${ms}`.padStart(3, "0");
}
export function formatFileSize(bytes: number) {
  if (bytes === 0) return "0 Bytes";

  const units = ["Bytes", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / 1024 ** i;

  return `${+value.toFixed(2)} ${units[i]}`;
}

export function useErrM(el: HTMLElement) {
  let errMsg: string | null = null;

  const obj = {
    clear() {
      errMsg = null;
    },
    get() {
      return errMsg;
    },
    report(msg: string) {
      errMsg = msg;
      obj.test();
    },
    set(msg: string) {
      errMsg = msg;
    },
    test() {
      const isInput = (el: HTMLElement) =>
        el instanceof HTMLButtonElement ||
        el instanceof HTMLInputElement ||
        el instanceof HTMLObjectElement ||
        el instanceof HTMLSelectElement ||
        el instanceof HTMLTextAreaElement;

      if (errMsg) {
        if (isInput(el)) {
          el.setCustomValidity(errMsg);
          el.reportValidity();
        }
        el.setAttribute("aria-invalid", "true");
        return false;
      }
      if (isInput(el)) {
        el.setCustomValidity("");
        el.reportValidity();
      }
      el.setAttribute("aria-invalid", "false");
      return true;
    },
  };

  return obj;
}

export function binaryInsert<T>(arr: T[], value: T, compare: (a: T, b: T) => number): number {
  let lo = 0;
  let hi = arr.length;

  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (compare(arr[mid], value) < 0) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }

  arr.splice(lo, 0, value);
  return lo;
}
export function binarySearch<T>(arr: T[], value: T, compare: (a: T, b: T) => number): number {
  let lo = 0;
  let hi = arr.length;

  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (compare(arr[mid], value) < 0) {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }

  return lo < arr.length && compare(arr[lo], value) === 0 ? lo : -1;
}

export type ValueOf<T> = T[keyof T];

export function pxToRem(px: number): number {
  return px / parseFloat(getComputedStyle(document.documentElement).fontSize);
}

export function kebabCaseToSentenceCase<S extends string>(str: S): SentenceCase<S> {
  return (str && str[0].toUpperCase() + str.slice(1).replaceAll("-", " ")) as SentenceCase<S>;
}

// i was bored
type KebabCaseToSentenceCase<S extends string> = S extends `${infer First}-${infer Rest}` ? `${First} ${KebabCaseToSentenceCase<Rest>}` : S;
type SentenceCase<S extends string> = S extends `${infer First}${infer Rest}` ? `${Uppercase<First>}${KebabCaseToSentenceCase<Rest>}` : S;

export const { BASE_URL } = import.meta.env;
export const nullify = <T>(): T | null => null;

export function maybeToEntries<T>(
  v: T,
): T extends readonly unknown[] ? T : T extends object ? [keyof T, T[keyof T]][] : T extends [keyof (infer K), infer V] ? [K, V][] : never {
  if (Array.isArray(v)) return v as any;
  if (typeof v !== "object" || v === null) return v as any;
  return Object.entries(v) as any;
}
