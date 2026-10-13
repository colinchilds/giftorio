import { JSX } from "solid-js/jsx-runtime";

export interface DialogDeleteApi {
  show(): void;
  hide(): void;
}

export interface DialogDeleteProps {
  onCancel?(): void;
  onConfirm?(): void;
  ref(api: DialogDeleteApi): void;

  autoclose?: boolean;
  cancelTitle?: string;
  deleteTitle?: string;

  children: JSX.Element;
}

export default function DialogDelete(props: DialogDeleteProps) {
  let refDialog: HTMLDialogElement = null!;

  function show() {
    refDialog.showModal();
  }
  function hide() {
    refDialog.close();
  }

  props.ref?.({ show, hide });

  return (
    <dialog class="panel fixed max-w-100 left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 backdrop:bg-black/60" ref={refDialog}>
      <div class="flex flex-col items-center justify-center">
        {props.children}
        <hr class="my-5" />
        <div class="flex justify-between w-full">
          <button
            class="button"
            type="button"
            onClick={() => {
              props.onCancel?.();
              (props.autoclose ?? true) && hide();
            }}
          >
            {props.cancelTitle ?? "Cancel"}
          </button>
          <button class="button button-red min-w-[128px]" type="button" onClick={() => props.onConfirm?.()}>
            {props.deleteTitle ?? "Delete"}
          </button>
        </div>
      </div>
    </dialog>
  );
}
