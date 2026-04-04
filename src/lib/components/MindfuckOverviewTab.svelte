<script lang="ts">
  import { _ } from "svelte-i18n";
  import type { ReadingProfile } from "../types";
  import { getCategoryColorVar, getBiasColor } from "$lib/utils/articleFormat";

  let {
    readingProfile,
    loadingProfile,
    biasIndicator,
  }: {
    readingProfile: ReadingProfile | null;
    loadingProfile: boolean;
    biasIndicator: string;
  } = $props();

  // Expanded category for subcategory view
  let expandedCategoryId = $state<number | null>(null);

  function toggleCategoryExpand(categoryId: number) {
    if (expandedCategoryId === categoryId) {
      expandedCategoryId = null;
    } else {
      expandedCategoryId = categoryId;
    }
  }
</script>

{#if loadingProfile}
  <div class="loading">{$_("fnordView.loading")}</div>
{:else if !readingProfile || readingProfile.total_read === 0}
  <div class="no-data">
    <p>{$_("mindfuck.profile.noData")}</p>
  </div>
{:else}
  {@const maxRead = Math.max(...readingProfile.by_category.map((c) => c.read_count), 1)}
  <!-- Profile Overview -->
  <div class="section">
    <h3>{$_("mindfuck.profile.title")}</h3>
    <div class="stats-grid">
      <div class="stat-item">
        <span class="stat-value">{readingProfile.total_read}</span>
        <span class="stat-label">{$_("mindfuck.profile.totalRead")}</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{readingProfile.total_articles}</span>
        <span class="stat-label">{$_("mindfuck.profile.totalArticles")}</span>
      </div>
      <div class="stat-item">
        <span class="stat-value">{readingProfile.read_percentage.toFixed(1)}%</span>
        <span class="stat-label">{$_("mindfuck.profile.readPercentage")}</span>
      </div>
      <div class="stat-item">
        <span
          class="stat-value bias-indicator bias-{getBiasColor(readingProfile.avg_political_bias, 'class')}"
        >
          {biasIndicator}
        </span>
        <span class="stat-label">{$_("mindfuck.profile.avgBias")}</span>
      </div>
    </div>
  </div>

  <!-- Category Distribution - Card Layout (like FnordView) -->
  <div class="section">
    <h3>{$_("mindfuck.categories.title")}</h3>
    <div class="category-cards">
      {#each readingProfile.by_category as cat (cat.sephiroth_id)}
        {@const barWidth = (cat.read_count / maxRead) * 100}
        {@const isExpanded = expandedCategoryId === cat.sephiroth_id}
        <button
          class="category-card {isExpanded ? 'expanded' : ''}"
          style="--cat-color: {getCategoryColorVar(cat.sephiroth_id)}"
          data-category-id={cat.sephiroth_id}
          onclick={() => toggleCategoryExpand(cat.sephiroth_id)}
        >
          <div class="card-header">
            <div class="card-icon-wrapper">
              {#if cat.icon}
                <i class={cat.icon}></i>
              {:else}
                <i class="fa-solid fa-folder"></i>
              {/if}
            </div>
            <span class="card-title">{cat.name}</span>
            <i class="fa-solid fa-chevron-down expand-icon {isExpanded ? 'rotated' : ''}"></i>
          </div>
          <div class="card-stats">
            <div class="stat-row">
              <span class="stat-label">{$_("mindfuck.categories.read")}</span>
              <span class="stat-value">{cat.read_count}</span>
            </div>
            <div class="progress-bar">
              <div class="progress-fill" style="width: {barWidth}%"></div>
            </div>
            <div class="stat-row secondary">
              <span class="stat-label">{$_("mindfuck.categories.available")}</span>
              <span class="stat-value">{cat.total_count}</span>
            </div>
          </div>

          <!-- Subcategories (expanded view) -->
          {#if isExpanded && cat.subcategories && cat.subcategories.length > 0}
            <div class="subcategories">
              {#each cat.subcategories as sub (sub.sephiroth_id)}
                <div class="subcategory-item">
                  <div class="subcategory-info">
                    {#if sub.icon}
                      <i class="{sub.icon} subcategory-icon"></i>
                    {/if}
                    <span class="subcategory-name">{sub.name}</span>
                    {#if sub.percentage < 30 && sub.total_count > 5}
                      <span class="warning-badge" title={$_("mindfuck.blindSpots.lowReadRate")}
                        >!</span
                      >
                    {/if}
                  </div>
                  <div class="subcategory-stats">
                    <span class="subcategory-count" title={$_("mindfuck.categories.read")}>
                      {sub.read_count}
                    </span>
                    <span class="subcategory-divider">/</span>
                    <span class="subcategory-count" title={$_("mindfuck.categories.available")}>
                      {sub.total_count}
                    </span>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </button>
      {/each}
    </div>
  </div>

  <!-- Political Bias Distribution - Spectrum View -->
  <div class="section">
    <h3>{$_("mindfuck.bias.title")}</h3>
    <div class="political-spectrum-card">
      <!-- Spectrum Header -->
      <div class="spectrum-header">
        <span class="spectrum-label left">{$_("mindfuck.bias.left")}</span>
        <span class="spectrum-label center">{$_("mindfuck.bias.neutral")}</span>
        <span class="spectrum-label right">{$_("mindfuck.bias.right")}</span>
      </div>

      <!-- Main Spectrum Bar -->
      {#if readingProfile.by_bias.reduce((sum, b) => sum + b.read_count, 0) > 0}
        {@const totalBiasReads = readingProfile.by_bias.reduce((sum, b) => sum + b.read_count, 0)}
        <div class="spectrum-bar-container">
          <div class="spectrum-bar">
            {#each readingProfile.by_bias as bias (bias.bias_value)}
              {@const segmentClass =
                bias.bias_value <= -1.5
                  ? "left-extreme"
                  : bias.bias_value <= -0.5
                    ? "left-leaning"
                    : bias.bias_value <= 0.5
                      ? "neutral"
                      : bias.bias_value <= 1.5
                        ? "right-leaning"
                        : "right-extreme"}
              {#if bias.read_count > 0}
                <div
                  class="spectrum-segment {segmentClass}"
                  style="width: {bias.percentage}%"
                  title="{bias.label}: {bias.read_count} ({bias.percentage.toFixed(1)}%)"
                >
                  {#if bias.percentage > 10}
                    <span class="segment-label">{bias.percentage.toFixed(0)}%</span>
                  {/if}
                </div>
              {/if}
            {/each}
          </div>

          <!-- Position Indicator (based on average bias) -->
          {#if readingProfile.avg_political_bias !== null}
            {@const avgBias = readingProfile.avg_political_bias}
            {@const indicatorPosition = ((avgBias + 2) / 4) * 100}
            <div
              class="position-indicator"
              style="left: {indicatorPosition}%"
              title="{$_('mindfuck.bias.yourPosition')}: {avgBias.toFixed(2)}"
            >
              <i class="fa-solid fa-caret-down"></i>
            </div>
          {/if}
        </div>

        <!-- Scale Markers -->
        <div class="spectrum-scale">
          <span class="scale-mark">-2</span>
          <span class="scale-mark">-1</span>
          <span class="scale-mark">0</span>
          <span class="scale-mark">+1</span>
          <span class="scale-mark">+2</span>
        </div>

        <!-- Detailed Breakdown -->
        <div class="spectrum-details">
          {#each readingProfile.by_bias as bias (bias.bias_value)}
            {@const segmentClass =
              bias.bias_value <= -1.5
                ? "left-extreme"
                : bias.bias_value <= -0.5
                  ? "left-leaning"
                  : bias.bias_value <= 0.5
                    ? "neutral"
                    : bias.bias_value <= 1.5
                      ? "right-leaning"
                      : "right-extreme"}
            <div class="detail-item">
              <span class="detail-dot {segmentClass}"></span>
              <span class="detail-label">{bias.label}</span>
              <span class="detail-count">{bias.read_count}</span>
              <span class="detail-percent">({bias.percentage.toFixed(1)}%)</span>
            </div>
          {/each}
        </div>

        <!-- Summary -->
        {#if readingProfile.avg_political_bias !== null}
          <div class="spectrum-summary">
            <div
              class="summary-indicator bias-{getBiasColor(readingProfile.avg_political_bias, 'class')}"
            >
              <i class="fa-solid fa-compass"></i>
              <span>{biasIndicator}</span>
            </div>
            <div class="summary-stat">
              <span class="summary-value">{totalBiasReads}</span>
              <span class="summary-label">{$_("mindfuck.bias.articlesAnalyzed")}</span>
            </div>
          </div>
        {/if}
      {:else}
        <div class="spectrum-empty">
          <i class="fa-solid fa-scale-balanced"></i>
          <p>{$_("mindfuck.bias.noData")}</p>
        </div>
      {/if}
    </div>
  </div>

  <!-- Sachlichkeit Distribution -->
  <div class="section">
    <h3>{$_("mindfuck.sachlichkeit.title")}</h3>
    <div class="sachlichkeit-spectrum-card">
      <!-- Spectrum Header -->
      <div class="sachlichkeit-header">
        <span class="sachlichkeit-label-header emotional"
          >{$_("mindfuck.sachlichkeit.emotional")}</span
        >
        <span class="sachlichkeit-label-header mixed">{$_("mindfuck.sachlichkeit.mixed")}</span>
        <span class="sachlichkeit-label-header objective"
          >{$_("mindfuck.sachlichkeit.objective")}</span
        >
      </div>

      <!-- Main Spectrum Bar -->
      {#if readingProfile.by_sachlichkeit.reduce((sum, s) => sum + s.read_count, 0) > 0}
        {@const totalSachReads = readingProfile.by_sachlichkeit.reduce(
          (sum, s) => sum + s.read_count,
          0,
        )}
        <div class="sachlichkeit-bar-container">
          <div class="sachlichkeit-spectrum-bar">
            {#each readingProfile.by_sachlichkeit as sach (sach.sachlichkeit_value)}
              {@const segmentClass =
                sach.sachlichkeit_value <= 0.5
                  ? "highly-emotional"
                  : sach.sachlichkeit_value <= 1.5
                    ? "emotional"
                    : sach.sachlichkeit_value <= 2.5
                      ? "mixed"
                      : sach.sachlichkeit_value <= 3.5
                        ? "mostly-objective"
                        : "objective"}
              {#if sach.read_count > 0}
                <div
                  class="sachlichkeit-segment {segmentClass}"
                  style="width: {sach.percentage}%"
                  title="{sach.label}: {sach.read_count} ({sach.percentage.toFixed(1)}%)"
                >
                  {#if sach.percentage > 10}
                    <span class="segment-label">{sach.percentage.toFixed(0)}%</span>
                  {/if}
                </div>
              {/if}
            {/each}
          </div>
        </div>

        <!-- Scale Markers -->
        <div class="sachlichkeit-scale">
          <span class="scale-mark">0</span>
          <span class="scale-mark">1</span>
          <span class="scale-mark">2</span>
          <span class="scale-mark">3</span>
          <span class="scale-mark">4</span>
        </div>

        <!-- Detailed Breakdown -->
        <div class="sachlichkeit-details">
          {#each readingProfile.by_sachlichkeit as sach (sach.sachlichkeit_value)}
            {@const segmentClass =
              sach.sachlichkeit_value <= 0.5
                ? "highly-emotional"
                : sach.sachlichkeit_value <= 1.5
                  ? "emotional"
                  : sach.sachlichkeit_value <= 2.5
                    ? "mixed"
                    : sach.sachlichkeit_value <= 3.5
                      ? "mostly-objective"
                      : "objective"}
            <div class="detail-item">
              <span class="detail-dot {segmentClass}"></span>
              <span class="detail-label">{sach.label}</span>
              <span class="detail-count">{sach.read_count}</span>
              <span class="detail-percent">({sach.percentage.toFixed(1)}%)</span>
            </div>
          {/each}
        </div>

        <!-- Summary -->
        <div class="sachlichkeit-summary">
          <div class="summary-indicator" style="color: var(--sach-objective)">
            <i class="fa-solid fa-gauge-high"></i>
            <span>{$_("mindfuck.sachlichkeit.title")}</span>
          </div>
          <div class="summary-stat">
            <span class="summary-value">{totalSachReads}</span>
            <span class="summary-label">{$_("mindfuck.bias.articlesAnalyzed")}</span>
          </div>
        </div>
      {:else}
        <div class="sachlichkeit-empty">
          <i class="fa-solid fa-gauge-high"></i>
          <p>{$_("mindfuck.sachlichkeit.noData")}</p>
        </div>
      {/if}
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

  /* Stats Grid */
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 1rem;
    padding: 1rem;
    background-color: var(--bg-overlay);
    border-radius: 0.5rem;
    border: 1px solid var(--border-default);
  }

  .stat-item {
    text-align: center;
  }

  .stat-value {
    display: block;
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--accent-primary);
  }

  .stat-value.bias-indicator {
    font-size: 1rem;
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Category Cards (matching FnordView) */
  .category-cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 1rem;
  }

  .category-card {
    background: linear-gradient(
      135deg,
      color-mix(in srgb, var(--cat-color) 25%, var(--bg-base)) 0%,
      color-mix(in srgb, var(--cat-color) 8%, var(--bg-base)) 100%
    );
    border: 1px solid color-mix(in srgb, var(--cat-color) 50%, transparent);
    border-left: 3px solid var(--cat-color);
    border-radius: 0.625rem;
    padding: 1rem;
    transition:
      transform 0.15s ease,
      box-shadow 0.15s ease,
      border-color 0.15s ease;
    cursor: pointer;
    text-align: left;
    width: 100%;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 15%, transparent);
  }

  .category-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 16px color-mix(in srgb, var(--cat-color) 30%, transparent);
    border-color: color-mix(in srgb, var(--cat-color) 70%, transparent);
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.875rem;
  }

  .card-icon-wrapper {
    width: 2.25rem;
    height: 2.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    background: linear-gradient(
      135deg,
      var(--cat-color),
      color-mix(in srgb, var(--cat-color) 70%, black)
    );
    border-radius: 0.5rem;
    color: white;
    font-size: 1rem;
    box-shadow: 0 2px 8px color-mix(in srgb, var(--cat-color) 40%, transparent);
  }

  .card-title {
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .card-stats {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .stat-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .stat-row.secondary {
    margin-top: 0.25rem;
  }

  .stat-row.secondary .stat-label,
  .stat-row.secondary .stat-value {
    font-size: 0.6875rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .progress-bar {
    height: 6px;
    background-color: color-mix(in srgb, var(--cat-color) 20%, transparent);
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(
      90deg,
      var(--cat-color),
      color-mix(in srgb, var(--cat-color) 80%, white)
    );
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .category-card.expanded {
    grid-column: 1 / -1;
  }

  .expand-icon {
    font-size: 0.75rem;
    color: var(--text-muted);
    transition: transform 0.2s ease;
    flex-shrink: 0;
  }

  .expand-icon.rotated {
    transform: rotate(180deg);
  }

  /* Subcategories */
  .subcategories {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid color-mix(in srgb, var(--cat-color) 20%, transparent);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .subcategory-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background-color: color-mix(in srgb, var(--cat-color) 8%, transparent);
    border-radius: 0.375rem;
  }

  .subcategory-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
    flex: 1;
  }

  .subcategory-icon {
    font-size: 0.75rem;
    color: var(--cat-color);
    flex-shrink: 0;
  }

  .subcategory-name {
    font-size: 0.8125rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .warning-badge {
    font-size: 0.625rem;
    width: 1rem;
    height: 1rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background-color: var(--accent-warning);
    color: var(--bg-base);
    border-radius: 50%;
    font-weight: 700;
    flex-shrink: 0;
  }

  .subcategory-stats {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .subcategory-count {
    font-weight: 500;
  }

  .subcategory-divider {
    color: var(--text-faint);
  }

  /* Sachlichkeit Spectrum Card */
  .sachlichkeit-spectrum-card {
    background-color: var(--bg-overlay);
    border-radius: 0.75rem;
    border: 1px solid var(--border-default);
    padding: 1.25rem;
  }

  .sachlichkeit-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }

  .sachlichkeit-label-header {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .sachlichkeit-label-header.emotional {
    color: var(--sach-emotional);
  }

  .sachlichkeit-label-header.mixed {
    color: var(--sach-mixed);
  }

  .sachlichkeit-label-header.objective {
    color: var(--sach-objective);
  }

  .sachlichkeit-bar-container {
    position: relative;
    margin-bottom: 0.5rem;
  }

  .sachlichkeit-spectrum-bar {
    display: flex;
    height: 2rem;
    border-radius: 0.5rem;
    overflow: hidden;
    background: linear-gradient(
      90deg,
      var(--sach-emotional) 0%,
      color-mix(in srgb, var(--sach-emotional) 50%, var(--bg-surface)) 25%,
      var(--sach-mixed) 50%,
      color-mix(in srgb, var(--sach-objective) 50%, var(--bg-surface)) 75%,
      var(--sach-objective) 100%
    );
    opacity: 0.15;
  }

  .sachlichkeit-spectrum-bar:has(.sachlichkeit-segment) {
    background: var(--bg-base);
    opacity: 1;
  }

  .sachlichkeit-segment {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      width 0.3s ease,
      opacity 0.2s ease;
    min-width: 2px;
    position: relative;
  }

  .sachlichkeit-segment:hover {
    filter: brightness(1.1);
  }

  .sachlichkeit-segment.highly-emotional {
    background: var(--sach-emotional);
  }

  .sachlichkeit-segment.emotional {
    background: color-mix(in srgb, var(--sach-emotional) 70%, var(--sach-mixed));
  }

  .sachlichkeit-segment.mixed {
    background: var(--sach-mixed);
  }

  .sachlichkeit-segment.mostly-objective {
    background: color-mix(in srgb, var(--sach-objective) 60%, var(--sach-mixed));
  }

  .sachlichkeit-segment.objective {
    background: var(--sach-objective);
  }

  .sachlichkeit-scale {
    display: flex;
    justify-content: space-between;
    padding: 0 0.25rem;
    margin-bottom: 1rem;
  }

  .sachlichkeit-details {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    margin-bottom: 1rem;
    padding: 0.75rem;
    background-color: var(--bg-base);
    border-radius: 0.5rem;
  }

  .sachlichkeit-details .detail-dot.highly-emotional {
    background: var(--sach-emotional);
  }

  .sachlichkeit-details .detail-dot.emotional {
    background: color-mix(in srgb, var(--sach-emotional) 70%, var(--sach-mixed));
  }

  .sachlichkeit-details .detail-dot.mixed {
    background: var(--sach-mixed);
  }

  .sachlichkeit-details .detail-dot.mostly-objective {
    background: color-mix(in srgb, var(--sach-objective) 60%, var(--sach-mixed));
  }

  .sachlichkeit-details .detail-dot.objective {
    background: var(--sach-objective);
  }

  .sachlichkeit-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-default);
  }

  .sachlichkeit-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    color: var(--text-muted);
    gap: 0.75rem;
  }

  .sachlichkeit-empty i {
    font-size: 2rem;
    opacity: 0.5;
  }

  .sachlichkeit-empty p {
    margin: 0;
    font-size: 0.875rem;
  }

  /* Political Spectrum Card */
  .political-spectrum-card {
    background-color: var(--bg-overlay);
    border-radius: 0.75rem;
    border: 1px solid var(--border-default);
    padding: 1.25rem;
  }

  .spectrum-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.75rem;
  }

  .spectrum-label {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .spectrum-label.left {
    color: var(--category-2);
  }

  .spectrum-label.center {
    color: var(--text-muted);
  }

  .spectrum-label.right {
    color: var(--category-5);
  }

  .spectrum-bar-container {
    position: relative;
    margin-bottom: 0.5rem;
  }

  .spectrum-bar {
    display: flex;
    height: 2rem;
    border-radius: 0.5rem;
    overflow: hidden;
    background: linear-gradient(
      90deg,
      var(--category-2) 0%,
      color-mix(in srgb, var(--category-2) 50%, var(--bg-surface)) 25%,
      var(--text-muted) 50%,
      color-mix(in srgb, var(--category-5) 50%, var(--bg-surface)) 75%,
      var(--category-5) 100%
    );
    opacity: 0.15;
  }

  .spectrum-bar:has(.spectrum-segment) {
    background: var(--bg-base);
    opacity: 1;
  }

  .spectrum-segment {
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition:
      width 0.3s ease,
      opacity 0.2s ease;
    min-width: 2px;
    position: relative;
  }

  .spectrum-segment:hover {
    filter: brightness(1.1);
  }

  .spectrum-segment.left-extreme {
    background: var(--category-2);
  }

  .spectrum-segment.left-leaning {
    background: color-mix(in srgb, var(--category-2) 60%, var(--bg-surface));
  }

  .spectrum-segment.neutral {
    background: var(--text-muted);
  }

  .spectrum-segment.right-leaning {
    background: color-mix(in srgb, var(--category-5) 60%, var(--bg-surface));
  }

  .spectrum-segment.right-extreme {
    background: var(--category-5);
  }

  .segment-label {
    font-size: 0.6875rem;
    font-weight: 600;
    color: white;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
  }

  .position-indicator {
    position: absolute;
    top: -0.25rem;
    transform: translateX(-50%);
    color: var(--accent-primary);
    font-size: 1.25rem;
    filter: drop-shadow(0 1px 2px rgba(0, 0, 0, 0.3));
    z-index: 10;
    transition: left 0.3s ease;
  }

  .spectrum-scale {
    display: flex;
    justify-content: space-between;
    padding: 0 0.25rem;
    margin-bottom: 1rem;
  }

  .scale-mark {
    font-size: 0.625rem;
    color: var(--text-muted);
    font-weight: 500;
  }

  .spectrum-details {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem 1rem;
    margin-bottom: 1rem;
    padding: 0.75rem;
    background-color: var(--bg-base);
    border-radius: 0.5rem;
  }

  .detail-item {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    font-size: 0.8125rem;
  }

  .detail-dot {
    width: 0.625rem;
    height: 0.625rem;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .detail-dot.left-extreme {
    background: var(--category-2);
  }

  .detail-dot.left-leaning {
    background: color-mix(in srgb, var(--category-2) 60%, var(--bg-surface));
  }

  .detail-dot.neutral {
    background: var(--text-muted);
  }

  .detail-dot.right-leaning {
    background: color-mix(in srgb, var(--category-5) 60%, var(--bg-surface));
  }

  .detail-dot.right-extreme {
    background: var(--category-5);
  }

  .detail-label {
    color: var(--text-secondary);
  }

  .detail-count {
    font-weight: 600;
    color: var(--text-primary);
  }

  .detail-percent {
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  .spectrum-summary {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-default);
  }

  .summary-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9375rem;
    font-weight: 600;
  }

  .summary-indicator i {
    font-size: 1rem;
  }

  .summary-stat {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
  }

  .summary-value {
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--accent-primary);
  }

  .summary-label {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .spectrum-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 2rem;
    color: var(--text-muted);
    gap: 0.75rem;
  }

  .spectrum-empty i {
    font-size: 2rem;
    opacity: 0.5;
  }

  .spectrum-empty p {
    margin: 0;
    font-size: 0.875rem;
  }

  /* Loading & No Data */
  .loading,
  .no-data {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3rem;
    color: var(--text-muted);
  }

  .no-data p {
    text-align: center;
    max-width: 400px;
  }

  /* Responsive */
  @media (max-width: 600px) {
    .stats-grid {
      grid-template-columns: repeat(2, 1fr);
    }

    .spectrum-details {
      flex-direction: column;
      gap: 0.5rem;
    }

    .spectrum-summary {
      flex-direction: column;
      gap: 1rem;
      align-items: flex-start;
    }

    .summary-stat {
      align-items: flex-start;
    }

    .political-spectrum-card {
      padding: 1rem;
    }
  }
</style>
