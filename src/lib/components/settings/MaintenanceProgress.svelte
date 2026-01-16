<script lang="ts">
  import { _ } from "svelte-i18n";

  interface Props {
    mode?: 'determinate' | 'indeterminate';
    progress?: number;
    current?: number;
    total?: number;
    label?: string;
    message?: string;
    status?: 'running' | 'success' | 'error';
    error?: string | null;
    showCancel?: boolean;
    onCancel?: () => void;
  }

  let {
    mode = 'indeterminate',
    progress = 0,
    current = 0,
    total = 0,
    label = '',
    message = '',
    status = 'running',
    error = null,
    showCancel = false,
    onCancel
  }: Props = $props();

  // Compute actual progress percentage
  let computedProgress = $derived(
    mode === 'determinate' && total > 0
      ? Math.min(100, Math.max(0, (current / total) * 100))
      : progress
  );

  // Status color classes
  let statusClass = $derived(
    status === 'success' ? 'status-success' :
    status === 'error' ? 'status-error' :
    'status-running'
  );
</script>

<div class="maintenance-progress {statusClass}">
  <div class="progress-header">
    <span class="progress-label">{label}</span>
    {#if showCancel && status === 'running' && onCancel}
      <button
        type="button"
        class="btn-cancel"
        onclick={onCancel}
      >
        {$_("batch.cancel")}
      </button>
    {/if}
  </div>

  <div class="progress-bar-container">
    {#if mode === 'indeterminate'}
      <div class="progress-bar indeterminate">
        <div class="progress-fill-indeterminate"></div>
      </div>
    {:else}
      <div class="progress-bar">
        <div
          class="progress-fill"
          style="width: {computedProgress}%"
        ></div>
      </div>
    {/if}
  </div>

  {#if mode === 'determinate' && total > 0}
    <div class="progress-details">
      <span class="progress-count">
        {current} / {total}
      </span>
      {#if message}
        <span class="progress-message" title={message}>
          {message.length > 40 ? message.slice(0, 40) + "..." : message}
        </span>
      {/if}
    </div>
  {:else if message}
    <div class="progress-message-only">
      {message}
    </div>
  {/if}

  {#if error}
    <div class="progress-error">
      {error}
    </div>
  {/if}
</div>

<style>
  .maintenance-progress {
    padding: 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.375rem;
    border: 1px solid var(--border-default);
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .progress-label {
    font-weight: 500;
    color: var(--accent-primary);
  }

  .status-success .progress-label {
    color: var(--status-success);
  }

  .status-error .progress-label {
    color: var(--status-error);
  }

  .btn-cancel {
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--status-error);
    border-radius: 0.25rem;
    background: none;
    color: var(--status-error);
    font-size: 0.75rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  .btn-cancel:hover {
    background-color: var(--status-error);
    color: var(--text-on-accent);
  }

  .progress-bar-container {
    margin-bottom: 0.5rem;
  }

  .progress-bar {
    height: 8px;
    background-color: var(--bg-surface);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background-color: var(--accent-primary);
    border-radius: 4px;
    transition: width 0.3s ease;
  }

  .status-success .progress-fill {
    background-color: var(--status-success);
  }

  .status-error .progress-fill {
    background-color: var(--status-error);
  }

  /* Indeterminate animation */
  .progress-bar.indeterminate {
    position: relative;
  }

  .progress-fill-indeterminate {
    position: absolute;
    height: 100%;
    width: 30%;
    background-color: var(--accent-primary);
    border-radius: 4px;
    animation: indeterminate 1.5s ease-in-out infinite;
  }

  @keyframes indeterminate {
    0% {
      left: -30%;
    }
    100% {
      left: 100%;
    }
  }

  .progress-details {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .progress-count {
    font-weight: 500;
    color: var(--text-primary);
  }

  .progress-message {
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 60%;
    text-align: right;
  }

  .progress-message-only {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .progress-error {
    margin-top: 0.5rem;
    padding: 0.5rem;
    background-color: rgba(243, 139, 168, 0.1);
    border-radius: 0.25rem;
    color: var(--status-error);
    font-size: 0.75rem;
  }
</style>
