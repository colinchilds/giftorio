export interface FormElementValue {
  disabled?: boolean;
  name: string;
  splash?: string;
  tooltip?: string;
  type: string;
}

export type WasmSharedMemory = { memory: WebAssembly.Memory; ptr: number; len: number };
