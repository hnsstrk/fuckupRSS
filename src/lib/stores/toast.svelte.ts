import type { Toast } from "../types";

let toastId = 0;

class ToastStore {
  items = $state<Toast[]>([]);

  add(type: Toast["type"], message: string, duration = 4000): void {
    const id = ++toastId;
    this.items = [...this.items, { id, type, message }];

    if (duration > 0) {
      setTimeout(() => {
        this.remove(id);
      }, duration);
    }
  }

  remove(id: number): void {
    this.items = this.items.filter((t) => t.id !== id);
  }

  success(message: string, duration = 4000): void {
    this.add("success", message, duration);
  }

  error(message: string, duration = 6000): void {
    this.add("error", message, duration);
  }

  info(message: string, duration = 4000): void {
    this.add("info", message, duration);
  }
}

export const toasts = new ToastStore();

export function removeToast(id: number): void {
  toasts.remove(id);
}
