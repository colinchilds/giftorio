export function formatDuration(totalMs: number): string {
  const totalS = Math.floor(totalMs / 1000);

  const h = Math.floor(totalS / 3600).toString();
  const m = Math.floor((totalS % 3600) / 60).toString();
  const s = (totalS % 60).toString().padStart(2, "0");
  const ms = (totalMs % 1000).toString().padStart(3, "0");

  return `${h.padStart(2, "0")}:${m.padStart(2, "0")}:${s.padStart(2, "0")}.${ms.padStart(3, "0")}`;
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
      } else if (isInput(el)) {
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
