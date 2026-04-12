<script lang="ts">
  import { _ } from "svelte-i18n";

  let {
    days,
    searchQuery,
    generating,
    ongenerate,
    ondayschange,
    onsearchchange,
  }: {
    days: number;
    searchQuery: string;
    generating: boolean;
    ongenerate: () => void;
    ondayschange: (days: number) => void;
    onsearchchange: (query: string) => void;
  } = $props();

  const dayOptions = [
    { value: 1, label: "themeReport.day1" },
    { value: 3, label: "themeReport.day3" },
    { value: 7, label: "themeReport.day7" },
    { value: 14, label: "themeReport.day14" },
  ];
</script>

<div class="tr-header">
  <div class="tr-header-left">
    <h2>
      <i class="fa-solid fa-newspaper"></i>
      {$_("themeReport.title")}
    </h2>
  </div>
  <div class="tr-header-controls">
    <div class="tr-time-range">
      <label for="tr-days">{$_("themeReport.days")}:</label>
      <select
        id="tr-days"
        value={days}
        onchange={(e) => ondayschange(Number((e.target as HTMLSelectElement).value))}
      >
        {#each dayOptions as opt (opt.value)}
          <option value={opt.value}>{$_(opt.label)}</option>
        {/each}
      </select>
    </div>
    <div class="tr-search">
      <i class="fa-solid fa-search tr-search-icon"></i>
      <input
        type="text"
        placeholder={$_("themeReport.search")}
        value={searchQuery}
        oninput={(e) => onsearchchange((e.target as HTMLInputElement).value)}
      />
    </div>
    <button class="btn-primary" onclick={ongenerate} disabled={generating}>
      {#if generating}
        <i class="fa-solid fa-spinner fa-spin"></i>
        {$_("themeReport.generating")}
      {:else}
        <i class="fa-solid fa-wand-magic-sparkles"></i>
        {$_("themeReport.generate")}
      {/if}
    </button>
  </div>
</div>

<style>
  .tr-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-color);
    flex-shrink: 0;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .tr-header-left h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-primary);
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .tr-header-left h2 i {
    color: var(--accent-primary);
  }

  .tr-header-controls {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .tr-time-range {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }

  .tr-time-range label {
    font-size: 0.8rem;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .tr-time-range select {
    padding: 0.3rem 0.5rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.82rem;
    cursor: pointer;
  }

  .tr-search {
    position: relative;
    display: flex;
    align-items: center;
  }

  .tr-search-icon {
    position: absolute;
    left: 0.5rem;
    font-size: 0.72rem;
    color: var(--text-muted);
    pointer-events: none;
  }

  .tr-search input {
    padding: 0.3rem 0.5rem 0.3rem 1.6rem;
    background: var(--bg-primary);
    border: 1px solid var(--border-color);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 0.82rem;
    width: 160px;
  }

  .tr-search input::placeholder {
    color: var(--text-muted);
  }

  .btn-primary {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.8rem;
    background: var(--accent-primary);
    color: var(--accent-text);
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 500;
    transition: opacity 0.15s;
  }

  .btn-primary:hover:not(:disabled) {
    opacity: 0.9;
  }

  .btn-primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
