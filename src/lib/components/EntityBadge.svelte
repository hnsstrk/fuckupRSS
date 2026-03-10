<script lang="ts">
  import { _ } from "svelte-i18n";

  let {
    name,
    entityType,
    mentionCount,
    articleCount,
    onclick,
  }: {
    name: string;
    entityType: string;
    mentionCount?: number;
    articleCount?: number;
    onclick?: () => void;
  } = $props();

  const typeConfig: Record<string, { icon: string; colorClass: string }> = {
    person: { icon: "fa-solid fa-user", colorClass: "entity-person" },
    organization: { icon: "fa-solid fa-building", colorClass: "entity-organization" },
    location: { icon: "fa-solid fa-location-dot", colorClass: "entity-location" },
    event: { icon: "fa-solid fa-calendar", colorClass: "entity-event" },
  };

  let config = $derived(typeConfig[entityType] || typeConfig.person);
  let typeLabel = $derived($_(`entities.${entityType}`) || entityType);
  let tooltipText = $derived(
    `${typeLabel}${mentionCount ? ` (${mentionCount} ${$_("entities.mentions")})` : ""}${articleCount ? ` - ${articleCount} ${$_("entities.articles")}` : ""}`,
  );
</script>

{#if onclick}
  <button class="entity-badge {config.colorClass}" title={tooltipText} {onclick}>
    <i class="{config.icon} badge-icon"></i>
    <span class="badge-text">{name}</span>
    {#if mentionCount && mentionCount > 1}
      <span class="badge-count">{mentionCount}</span>
    {/if}
  </button>
{:else}
  <span class="entity-badge {config.colorClass}" title={tooltipText}>
    <i class="{config.icon} badge-icon"></i>
    <span class="badge-text">{name}</span>
    {#if mentionCount && mentionCount > 1}
      <span class="badge-count">{mentionCount}</span>
    {/if}
  </span>
{/if}

<style>
  .entity-badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.15rem 0.5rem;
    border-radius: 9999px;
    font-size: 0.7rem;
    font-weight: 500;
    border: 1px solid transparent;
    transition: all 0.2s;
    white-space: nowrap;
  }

  button.entity-badge {
    cursor: pointer;
    background: none;
  }

  button.entity-badge:hover {
    filter: brightness(1.2);
    transform: translateY(-1px);
  }

  .badge-icon {
    font-size: 0.65rem;
  }

  .badge-text {
    max-width: 12rem;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge-count {
    font-size: 0.6rem;
    opacity: 0.7;
    margin-left: 0.125rem;
  }

  /* Entity type colors */
  .entity-person {
    background-color: rgba(59, 130, 246, 0.15);
    color: rgb(96, 165, 250);
    border-color: rgba(59, 130, 246, 0.3);
  }

  .entity-organization {
    background-color: rgba(34, 197, 94, 0.15);
    color: rgb(74, 222, 128);
    border-color: rgba(34, 197, 94, 0.3);
  }

  .entity-location {
    background-color: rgba(239, 68, 68, 0.15);
    color: rgb(248, 113, 113);
    border-color: rgba(239, 68, 68, 0.3);
  }

  .entity-event {
    background-color: rgba(168, 85, 247, 0.15);
    color: rgb(192, 132, 252);
    border-color: rgba(168, 85, 247, 0.3);
  }

  @media print {
    .entity-badge {
      display: none !important;
    }
  }
</style>
