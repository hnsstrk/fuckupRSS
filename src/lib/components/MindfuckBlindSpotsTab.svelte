<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { BlindSpot } from "../types";

  let {
    blindSpots,
    loadingBlindSpots,
  }: {
    blindSpots: BlindSpot[];
    loadingBlindSpots: boolean;
  } = $props();

  function getSeverityColor(severity: string): string {
    switch (severity) {
      case "high":
        return "var(--status-error)";
      case "medium":
        return "var(--status-warning)";
      case "low":
        return "var(--ctp-yellow)";
      default:
        return "var(--text-muted)";
    }
  }
</script>

{#if loadingBlindSpots}
  <div class="loading">{$_("fnordView.loading")}</div>
{:else if blindSpots.length === 0}
  <!-- No blind spots - positive message -->
  <div class="no-blind-spots-container">
    <div class="no-blind-spots-card">
      <div class="success-icon-wrapper">
        <i class="fa-solid fa-check-circle"></i>
      </div>
      <h3>{$_("mindfuck.blindSpots.noBlindSpots")}</h3>
      <p>{$_("mindfuck.blindSpots.noBlindSpotsSubtitle")}</p>
      <div class="balance-indicator">
        <div class="balance-bar">
          <div class="balance-fill"></div>
        </div>
        <span class="balance-label">100%</span>
      </div>
    </div>
  </div>
{:else}
  <div class="section">
    <h3>{$_("mindfuck.blindSpots.title")}</h3>
    <p class="section-description">{$_("mindfuck.blindSpots.description")}</p>

    <!-- Blind Spots Cards Grid -->
    <div class="blind-spots-grid">
      {#each blindSpots as spot (spot.name)}
        {@const readPercentage =
          spot.available_count > 0 ? Math.round((spot.read_count / spot.available_count) * 100) : 0}
        {@const severityColor = getSeverityColor(spot.severity)}
        {@const categoryColor = spot.main_category_color || severityColor}
        <div
          class="blind-spot-card severity-{spot.severity}"
          style="--severity-color: {severityColor}; --category-color: {categoryColor}"
        >
          <!-- Severity indicator bar at top -->
          <div class="severity-bar"></div>

          <!-- Card Header with Icon and Category -->
          <div class="blind-spot-card-header">
            <div class="blind-spot-icon-wrapper">
              {#if spot.icon}
                <i class={spot.icon}></i>
              {:else}
                <i class="fa-solid fa-eye-slash"></i>
              {/if}
            </div>
            <div class="blind-spot-badge" style="background-color: {severityColor}">
              {$_(`mindfuck.blindSpots.severity.${spot.severity}`)}
            </div>
          </div>

          <!-- Title and Category -->
          <div class="blind-spot-title-section">
            <h4 class="blind-spot-card-title">{spot.name}</h4>
            {#if spot.main_category}
              <span
                class="blind-spot-category-tag"
                style="color: {categoryColor}; border-color: {categoryColor}"
              >
                <i class="fa-solid fa-folder-tree"></i>
                {spot.main_category}
              </span>
            {/if}
          </div>

          <!-- Progress visualization -->
          <div class="blind-spot-progress-section">
            <div class="blind-spot-progress-header">
              <span class="blind-spot-progress-label"
                >{$_("mindfuck.blindSpots.readPercentage", {
                  values: { percent: readPercentage },
                })}</span
              >
              <span class="blind-spot-progress-ratio"
                >{spot.read_count} / {spot.available_count}</span
              >
            </div>
            <div class="blind-spot-progress-bar">
              <div
                class="blind-spot-progress-fill"
                style="width: {readPercentage}%; background-color: {severityColor}"
              ></div>
              <div
                class="blind-spot-progress-remaining"
                style="width: {100 - readPercentage}%"
              ></div>
            </div>
          </div>

          <!-- Severity explanation -->
          <div class="blind-spot-explanation">
            <i class="fa-solid fa-circle-info"></i>
            <span>{$_(`mindfuck.blindSpots.severityDescription.${spot.severity}`)}</span>
          </div>

          <!-- Action button -->
          <button
            class="blind-spot-action-btn"
            onclick={() => {
              const event = new CustomEvent("filter-by-category", {
                detail: { categoryName: spot.name },
              });
              window.dispatchEvent(event);
            }}
            type="button"
          >
            <i class="fa-solid fa-arrow-right"></i>
            {$_("mindfuck.blindSpots.exploreCategory")}
          </button>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  /* Sections */
  .section {
    margin-bottom: 2rem;
  }

  .section h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
  }

  .section-description {
    margin: -0.5rem 0 1rem 0;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  /* Loading */
  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    color: var(--text-muted);
  }

  /* Blind Spots - No Blind Spots State */
  .no-blind-spots-container {
    display: flex;
    justify-content: center;
    align-items: center;
    padding: 3rem 1rem;
  }

  .no-blind-spots-card {
    text-align: center;
    max-width: 400px;
    padding: 2.5rem 2rem;
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--status-success) 15%, var(--bg-overlay)) 0%,
      color-mix(in srgb, var(--status-success) 5%, var(--bg-overlay)) 100%
    );
    border: 1px solid color-mix(in srgb, var(--status-success) 30%, var(--border-default));
    border-radius: 1rem;
    box-shadow: 0 4px 24px color-mix(in srgb, var(--status-success) 10%, transparent);
  }

  .success-icon-wrapper {
    width: 4.5rem;
    height: 4.5rem;
    margin: 0 auto 1.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(
      135deg,
      var(--status-success),
      color-mix(in srgb, var(--status-success) 70%, var(--ctp-green))
    );
    border-radius: 50%;
    box-shadow: 0 8px 24px color-mix(in srgb, var(--status-success) 35%, transparent);
  }

  .success-icon-wrapper i {
    font-size: 2.25rem;
    color: white;
  }

  .no-blind-spots-card h3 {
    margin: 0 0 0.75rem;
    font-size: 1.25rem;
    color: var(--text-primary);
    font-weight: 600;
  }

  .no-blind-spots-card p {
    margin: 0 0 1.5rem;
    font-size: 0.9375rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }

  .balance-indicator {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    justify-content: center;
  }

  .balance-bar {
    width: 120px;
    height: 8px;
    background: var(--border-subtle);
    border-radius: 4px;
    overflow: hidden;
  }

  .balance-fill {
    width: 100%;
    height: 100%;
    background: linear-gradient(
      90deg,
      var(--status-success),
      color-mix(in srgb, var(--status-success) 80%, var(--ctp-green))
    );
    border-radius: 4px;
  }

  .balance-label {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--status-success);
  }

  /* Blind Spots - Cards Grid */
  .blind-spots-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1.25rem;
  }

  .blind-spot-card {
    position: relative;
    background: var(--bg-overlay);
    border: 1px solid var(--border-default);
    border-radius: 0.75rem;
    padding: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    transition:
      transform 0.2s ease,
      box-shadow 0.2s ease,
      border-color 0.2s ease;
    overflow: hidden;
  }

  .blind-spot-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px color-mix(in srgb, var(--severity-color) 20%, transparent);
    border-color: color-mix(in srgb, var(--severity-color) 50%, var(--border-default));
  }

  .severity-bar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 4px;
    background: var(--severity-color);
  }

  .blind-spot-card.severity-high .severity-bar {
    background: linear-gradient(
      90deg,
      var(--status-error),
      color-mix(in srgb, var(--status-error) 70%, var(--ctp-red))
    );
  }

  .blind-spot-card.severity-medium .severity-bar {
    background: linear-gradient(
      90deg,
      var(--status-warning),
      color-mix(in srgb, var(--status-warning) 70%, var(--ctp-peach))
    );
  }

  .blind-spot-card.severity-low .severity-bar {
    background: linear-gradient(
      90deg,
      var(--ctp-yellow),
      color-mix(in srgb, var(--ctp-yellow) 70%, var(--ctp-rosewater))
    );
  }

  .blind-spot-card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding-top: 0.25rem;
  }

  .blind-spot-icon-wrapper {
    width: 2.5rem;
    height: 2.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--severity-color) 25%, var(--bg-surface)),
      color-mix(in srgb, var(--severity-color) 10%, var(--bg-surface))
    );
    border: 1px solid color-mix(in srgb, var(--severity-color) 30%, transparent);
    border-radius: 0.5rem;
    color: var(--severity-color);
    font-size: 1.125rem;
  }

  .blind-spot-badge {
    padding: 0.25rem 0.625rem;
    border-radius: 1rem;
    font-size: 0.6875rem;
    font-weight: 600;
    color: white;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }

  .blind-spot-title-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .blind-spot-card-title {
    margin: 0;
    font-size: 1.0625rem;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.3;
  }

  .blind-spot-category-tag {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid;
    border-radius: 0.25rem;
    font-size: 0.6875rem;
    font-weight: 500;
    width: fit-content;
  }

  .blind-spot-category-tag i {
    font-size: 0.625rem;
  }

  .blind-spot-progress-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .blind-spot-progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .blind-spot-progress-label {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .blind-spot-progress-ratio {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .blind-spot-progress-bar {
    display: flex;
    height: 8px;
    border-radius: 4px;
    overflow: hidden;
    background: var(--border-subtle);
  }

  .blind-spot-progress-fill {
    height: 100%;
    border-radius: 4px 0 0 4px;
    transition: width 0.3s ease;
  }

  .blind-spot-progress-remaining {
    height: 100%;
    background: repeating-linear-gradient(
      -45deg,
      color-mix(in srgb, var(--severity-color) 8%, transparent),
      color-mix(in srgb, var(--severity-color) 8%, transparent) 4px,
      color-mix(in srgb, var(--severity-color) 15%, transparent) 4px,
      color-mix(in srgb, var(--severity-color) 15%, transparent) 8px
    );
  }

  .blind-spot-explanation {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.625rem;
    background: color-mix(in srgb, var(--severity-color) 8%, transparent);
    border-radius: 0.375rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .blind-spot-explanation i {
    color: var(--severity-color);
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .blind-spot-action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.625rem 1rem;
    background: transparent;
    border: 1px solid var(--border-default);
    border-radius: 0.5rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.2s ease;
    margin-top: auto;
  }

  .blind-spot-action-btn:hover {
    background: color-mix(in srgb, var(--severity-color) 10%, transparent);
    border-color: var(--severity-color);
    color: var(--severity-color);
  }

  .blind-spot-action-btn i {
    font-size: 0.75rem;
    transition: transform 0.2s ease;
  }

  .blind-spot-action-btn:hover i {
    transform: translateX(3px);
  }

  /* Responsive */
  @media (max-width: 600px) {
    .blind-spots-grid {
      grid-template-columns: 1fr;
    }

    .no-blind-spots-container {
      padding: 2rem 0.5rem;
    }

    .no-blind-spots-card {
      padding: 2rem 1.5rem;
    }

    .success-icon-wrapper {
      width: 3.5rem;
      height: 3.5rem;
    }

    .success-icon-wrapper i {
      font-size: 1.75rem;
    }

    .no-blind-spots-card h3 {
      font-size: 1.125rem;
    }

    .blind-spot-card {
      padding: 1rem;
    }

    .blind-spot-icon-wrapper {
      width: 2rem;
      height: 2rem;
      font-size: 1rem;
    }

    .blind-spot-badge {
      padding: 0.2rem 0.5rem;
      font-size: 0.625rem;
    }

    .blind-spot-card-title {
      font-size: 1rem;
    }
  }
</style>
