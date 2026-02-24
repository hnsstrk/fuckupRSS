<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    variant?: "primary" | "danger" | "default" | "ghost";
    size?: "sm" | "md" | "lg";
    icon?: string;
    disabled?: boolean;
    loading?: boolean;
    onclick?: (e: MouseEvent) => void;
    title?: string;
    type?: "button" | "submit" | "reset";
    children?: Snippet;
  }

  let {
    variant = "default",
    size = "md",
    icon,
    disabled = false,
    loading = false,
    onclick,
    title,
    type = "button",
    children,
  }: Props = $props();

  const sizeClass = $derived(`action-button--${size}`);
  const variantClass = $derived(`action-button--${variant}`);
</script>

<button
  {type}
  class="action-button {variantClass} {sizeClass}"
  class:action-button--loading={loading}
  {disabled}
  {onclick}
  {title}
>
  {#if loading}
    <i class="fa-solid fa-rotate fa-spin action-button__icon" aria-hidden="true"></i>
  {:else if icon}
    <i class="{icon} action-button__icon" aria-hidden="true"></i>
  {/if}
  {#if children}
    <span class="action-button__label">
      {@render children()}
    </span>
  {/if}
</button>

<style>
  .action-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    border: 1px solid var(--border-default);
    border-radius: 6px;
    cursor: pointer;
    font-weight: 500;
    transition:
      background-color 0.15s ease,
      border-color 0.15s ease,
      opacity 0.15s ease;
    white-space: nowrap;
    font-family: inherit;
    line-height: 1;
  }

  .action-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .action-button--loading {
    pointer-events: none;
    opacity: 0.7;
  }

  /* Sizes */
  .action-button--sm {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
  }

  .action-button--md {
    padding: 0.4rem 0.75rem;
    font-size: 0.8rem;
  }

  .action-button--lg {
    padding: 0.5rem 1rem;
    font-size: 0.9rem;
  }

  /* Variants */
  .action-button--default {
    background: var(--bg-overlay);
    color: var(--text-primary);
    border-color: var(--border-default);
  }

  .action-button--default:hover:not(:disabled) {
    background: var(--bg-muted);
    border-color: var(--text-faint);
  }

  .action-button--primary {
    background: var(--accent-primary);
    color: var(--text-on-accent);
    border-color: var(--accent-primary);
  }

  .action-button--primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .action-button--danger {
    background: transparent;
    color: var(--accent-error);
    border-color: var(--accent-error);
  }

  .action-button--danger:hover:not(:disabled) {
    background: var(--accent-error);
    color: var(--text-on-accent);
  }

  .action-button--ghost {
    background: transparent;
    color: var(--text-secondary);
    border-color: transparent;
  }

  .action-button--ghost:hover:not(:disabled) {
    background: var(--bg-overlay);
    color: var(--text-primary);
  }

  .action-button__icon {
    font-size: 0.85em;
  }
</style>
