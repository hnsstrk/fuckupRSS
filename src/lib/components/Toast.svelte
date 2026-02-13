<script lang="ts">
  import { _ } from "svelte-i18n";
  import { toasts, removeToast } from "../stores/state.svelte";
  import type { Toast } from "../types";

  function getIconClass(type: Toast["type"]): string {
    switch (type) {
      case "success":
        return "fa-solid fa-check";
      case "error":
        return "fa-solid fa-xmark";
      case "info":
        return "fa-solid fa-circle-info";
      default:
        return "";
    }
  }
</script>

{#if toasts.items.length > 0}
  <div class="toast-container">
    {#each toasts.items as toast (toast.id)}
      <div class="toast toast-{toast.type}" role="alert">
        <i class="toast-icon {getIconClass(toast.type)}"></i>
        <span class="toast-message">{toast.message}</span>
        <button
          class="toast-close"
          onclick={() => removeToast(toast.id)}
          aria-label={$_("actions.close")}
        >
          <i class="fa-solid fa-xmark"></i>
        </button>
      </div>
    {/each}
  </div>
{/if}

<style>
  .toast-container {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-width: 400px;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    border-radius: 0.5rem;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    animation: slideIn 0.3s ease-out;
  }

  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  .toast-success {
    border-left: 4px solid var(--green);
  }

  .toast-success .toast-icon {
    color: var(--green);
  }

  .toast-error {
    border-left: 4px solid var(--red);
  }

  .toast-error .toast-icon {
    color: var(--red);
  }

  .toast-info {
    border-left: 4px solid var(--blue);
  }

  .toast-info .toast-icon {
    color: var(--blue);
  }

  .toast-icon {
    font-size: 1rem;
    flex-shrink: 0;
    width: 1.25rem;
    text-align: center;
  }

  .toast-message {
    flex: 1;
    color: var(--text-primary);
    font-size: 0.875rem;
    line-height: 1.4;
  }

  .toast-close {
    flex-shrink: 0;
    width: 1.5rem;
    height: 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: 0.25rem;
    font-size: 0.875rem;
    line-height: 1;
    transition:
      background-color 0.15s,
      color 0.15s;
  }

  .toast-close:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
