# RECS_UI_SPEC.md — Frontend UI Spezifikation

**Erstellt:** 2026-01-18
**Status:** Phase 2 — Technical Design

---

## 1. Übersicht

### Neue/Geänderte Komponenten

| Komponente | Typ | Beschreibung |
|------------|-----|--------------|
| `RecommendationCard.svelte` | Neu | Erweiterte Artikel-Karte mit Feedback |
| `RecommendationList.svelte` | Neu | Liste mit Empty State |
| `MindfuckView.svelte` | Änderung | Neuer Tab + Integration |
| `SavedArticles.svelte` | Neu | Gemerkte Artikel anzeigen |

---

## 2. RecommendationCard.svelte

### 2.1 Design

```
┌─────────────────────────────────────────────────────────────┐
│ ┌──────┐                                                    │
│ │ 🖼️  │  Artikel-Titel der hier steht und vielleicht      │
│ │ Image│  auch mal über zwei Zeilen geht                   │
│ └──────┘                                                    │
│                                                             │
│ [Quelle-Icon] Quelle · vor 2 Stunden                        │
│                                                             │
│ Zusammenfassung des Artikels die hier steht und einen      │
│ kurzen Überblick über den Inhalt gibt...                   │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ 💡 Basierend auf: Trump, NATO, Außenpolitik                │
│                                                             │
│ [Kategorie-Badge] [Kategorie-Badge]                        │
│                                                             │
│                              [Verstecken]  [💾 Merken]     │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Props

```typescript
interface RecommendationCardProps {
  recommendation: Recommendation;
  onSave: (fnordId: number) => void;
  onUnsave: (fnordId: number) => void;
  onHide: (fnordId: number) => void;
  onClick: (fnordId: number) => void;
  showBias?: boolean;
  compact?: boolean;
}
```

### 2.3 Implementation

```svelte
<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { createEventDispatcher } from 'svelte';
  import type { Recommendation } from '$lib/types';
  import { formatRelativeTime } from '$lib/utils/time';
  import { getCategoryColorVar } from '$lib/utils/categories';
  import BiasIndicator from './BiasIndicator.svelte';
  import Tooltip from './Tooltip.svelte';

  export let recommendation: Recommendation;
  export let showBias = true;
  export let compact = false;

  const dispatch = createEventDispatcher<{
    save: number;
    unsave: number;
    hide: number;
    click: number;
  }>();

  let isSaving = false;
  let isHiding = false;

  async function handleSave() {
    if (isSaving) return;
    isSaving = true;

    if (recommendation.is_saved) {
      dispatch('unsave', recommendation.fnord_id);
    } else {
      dispatch('save', recommendation.fnord_id);
    }

    isSaving = false;
  }

  async function handleHide() {
    if (isHiding) return;
    isHiding = true;
    dispatch('hide', recommendation.fnord_id);
  }

  function handleClick() {
    dispatch('click', recommendation.fnord_id);
  }
</script>

<article
  class="recommendation-card"
  class:compact
  class:saved={recommendation.is_saved}
  role="article"
  aria-label={recommendation.title}
>
  <!-- Header mit Bild -->
  <button class="card-content" on:click={handleClick}>
    {#if recommendation.image_url && !compact}
      <div class="card-image">
        <img
          src={recommendation.image_url}
          alt=""
          loading="lazy"
        />
      </div>
    {/if}

    <div class="card-body">
      <h3 class="card-title">{recommendation.title}</h3>

      <div class="card-meta">
        {#if recommendation.pentacle_icon}
          <img
            src={recommendation.pentacle_icon}
            alt=""
            class="source-icon"
          />
        {/if}
        <span class="source-name">{recommendation.pentacle_title}</span>
        <span class="separator">·</span>
        <time datetime={recommendation.published_at}>
          {formatRelativeTime(recommendation.published_at)}
        </time>
      </div>

      {#if recommendation.summary && !compact}
        <p class="card-summary">{recommendation.summary}</p>
      {/if}
    </div>
  </button>

  <!-- Explanation -->
  <div class="card-explanation">
    <i class="fa-solid fa-lightbulb"></i>
    <span>{recommendation.explanation}</span>
  </div>

  <!-- Footer mit Kategorien und Actions -->
  <footer class="card-footer">
    <div class="card-categories">
      {#each recommendation.categories.slice(0, 2) as category}
        <span
          class="category-badge"
          style="--category-color: var({getCategoryColorVar(category.id)})"
        >
          {#if category.icon}
            <i class={category.icon}></i>
          {/if}
          {category.name}
        </span>
      {/each}

      {#if showBias && recommendation.political_bias !== null}
        <BiasIndicator bias={recommendation.political_bias} compact />
      {/if}
    </div>

    <div class="card-actions">
      <Tooltip text={$_('recommendations.hide')}>
        <button
          class="action-btn hide-btn"
          on:click|stopPropagation={handleHide}
          disabled={isHiding}
          aria-label={$_('recommendations.hide')}
        >
          <i class="fa-regular fa-eye-slash"></i>
        </button>
      </Tooltip>

      <Tooltip text={recommendation.is_saved ? $_('recommendations.unsave') : $_('recommendations.save')}>
        <button
          class="action-btn save-btn"
          class:saved={recommendation.is_saved}
          on:click|stopPropagation={handleSave}
          disabled={isSaving}
          aria-label={recommendation.is_saved ? $_('recommendations.unsave') : $_('recommendations.save')}
        >
          <i class={recommendation.is_saved ? 'fa-solid fa-bookmark' : 'fa-regular fa-bookmark'}></i>
          {#if !compact}
            <span>{recommendation.is_saved ? $_('recommendations.saved') : $_('recommendations.save')}</span>
          {/if}
        </button>
      </Tooltip>
    </div>
  </footer>
</article>

<style>
  .recommendation-card {
    background: var(--surface0);
    border: 1px solid var(--surface1);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: all 0.2s ease;
  }

  .recommendation-card:hover {
    border-color: var(--surface2);
    box-shadow: 0 4px 12px var(--shadow-color);
  }

  .recommendation-card.saved {
    border-color: var(--accent);
    border-left-width: 3px;
  }

  .card-content {
    display: block;
    width: 100%;
    padding: 0;
    background: none;
    border: none;
    text-align: left;
    cursor: pointer;
  }

  .card-image {
    width: 100%;
    height: 160px;
    overflow: hidden;
  }

  .card-image img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .card-body {
    padding: var(--spacing-md);
  }

  .card-title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    color: var(--text);
    margin: 0 0 var(--spacing-xs);
    line-height: 1.3;

    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    font-size: var(--font-size-sm);
    color: var(--subtext0);
  }

  .source-icon {
    width: 16px;
    height: 16px;
    border-radius: var(--radius-sm);
  }

  .separator {
    color: var(--surface2);
  }

  .card-summary {
    margin: var(--spacing-sm) 0 0;
    font-size: var(--font-size-sm);
    color: var(--subtext1);
    line-height: 1.5;

    display: -webkit-box;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .card-explanation {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--mantle);
    font-size: var(--font-size-sm);
    color: var(--subtext0);
  }

  .card-explanation i {
    color: var(--yellow);
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--spacing-sm) var(--spacing-md);
    border-top: 1px solid var(--surface1);
  }

  .card-categories {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
    flex-wrap: wrap;
  }

  .category-badge {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: 2px var(--spacing-sm);
    background: color-mix(in srgb, var(--category-color) 20%, transparent);
    border: 1px solid color-mix(in srgb, var(--category-color) 40%, transparent);
    border-radius: var(--radius-full);
    font-size: var(--font-size-xs);
    color: var(--category-color);
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: var(--spacing-xs);
  }

  .action-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--spacing-xs);
    padding: var(--spacing-xs) var(--spacing-sm);
    background: transparent;
    border: 1px solid var(--surface2);
    border-radius: var(--radius-md);
    font-size: var(--font-size-sm);
    color: var(--subtext0);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .action-btn:hover:not(:disabled) {
    background: var(--surface1);
    color: var(--text);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .save-btn.saved {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--base);
  }

  .hide-btn:hover {
    border-color: var(--red);
    color: var(--red);
  }

  /* Compact Mode */
  .compact .card-body {
    padding: var(--spacing-sm);
  }

  .compact .card-title {
    font-size: var(--font-size-base);
  }

  .compact .card-explanation {
    padding: var(--spacing-xs) var(--spacing-sm);
  }

  .compact .card-footer {
    padding: var(--spacing-xs) var(--spacing-sm);
  }
</style>
```

---

## 3. RecommendationList.svelte

### 3.1 Implementation

```svelte
<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import type { Recommendation } from '$lib/types';
  import {
    getRecommendations,
    saveArticle,
    unsaveArticle,
    hideRecommendation,
  } from '$lib/api/recommendations';
  import RecommendationCard from './RecommendationCard.svelte';
  import LoadingSpinner from './LoadingSpinner.svelte';
  import EmptyState from './EmptyState.svelte';

  export let onArticleClick: (fnordId: number) => void;

  let recommendations: Recommendation[] = [];
  let isLoading = true;
  let error: string | null = null;

  onMount(async () => {
    await loadRecommendations();
  });

  async function loadRecommendations() {
    isLoading = true;
    error = null;

    try {
      recommendations = await getRecommendations(10);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Unbekannter Fehler';
    } finally {
      isLoading = false;
    }
  }

  async function handleSave(event: CustomEvent<number>) {
    const fnordId = event.detail;

    try {
      await saveArticle(fnordId);
      recommendations = recommendations.map(r =>
        r.fnord_id === fnordId ? { ...r, is_saved: true } : r
      );
    } catch (e) {
      console.error('Failed to save article:', e);
    }
  }

  async function handleUnsave(event: CustomEvent<number>) {
    const fnordId = event.detail;

    try {
      await unsaveArticle(fnordId);
      recommendations = recommendations.map(r =>
        r.fnord_id === fnordId ? { ...r, is_saved: false } : r
      );
    } catch (e) {
      console.error('Failed to unsave article:', e);
    }
  }

  async function handleHide(event: CustomEvent<number>) {
    const fnordId = event.detail;

    try {
      await hideRecommendation(fnordId);
      // Optimistic update: Remove from list
      recommendations = recommendations.filter(r => r.fnord_id !== fnordId);

      // Lade neue Empfehlung nach
      if (recommendations.length < 5) {
        const newRecs = await getRecommendations(5);
        const existingIds = new Set(recommendations.map(r => r.fnord_id));
        const toAdd = newRecs.filter(r => !existingIds.has(r.fnord_id));
        recommendations = [...recommendations, ...toAdd.slice(0, 5 - recommendations.length)];
      }
    } catch (e) {
      console.error('Failed to hide recommendation:', e);
    }
  }

  function handleClick(event: CustomEvent<number>) {
    onArticleClick(event.detail);
  }
</script>

<div class="recommendation-list">
  {#if isLoading}
    <div class="loading-container">
      <LoadingSpinner />
      <p>{$_('recommendations.loading')}</p>
    </div>

  {:else if error}
    <div class="error-container">
      <i class="fa-solid fa-exclamation-triangle"></i>
      <p>{error}</p>
      <button on:click={loadRecommendations}>
        {$_('common.retry')}
      </button>
    </div>

  {:else if recommendations.length === 0}
    <EmptyState
      icon="fa-solid fa-wand-magic-sparkles"
      title={$_('recommendations.empty.title')}
      description={$_('recommendations.empty.description')}
    >
      <div class="empty-tips">
        <h4>{$_('recommendations.empty.tips_title')}</h4>
        <ul>
          <li>
            <i class="fa-solid fa-book-open"></i>
            {$_('recommendations.empty.tip_read')}
          </li>
          <li>
            <i class="fa-solid fa-rss"></i>
            {$_('recommendations.empty.tip_feeds')}
          </li>
          <li>
            <i class="fa-solid fa-robot"></i>
            {$_('recommendations.empty.tip_ollama')}
          </li>
        </ul>
      </div>
    </EmptyState>

  {:else}
    <div class="recommendations-grid">
      {#each recommendations as recommendation (recommendation.fnord_id)}
        <RecommendationCard
          {recommendation}
          on:save={handleSave}
          on:unsave={handleUnsave}
          on:hide={handleHide}
          on:click={handleClick}
        />
      {/each}
    </div>

    <button class="refresh-btn" on:click={loadRecommendations}>
      <i class="fa-solid fa-arrows-rotate"></i>
      {$_('recommendations.refresh')}
    </button>
  {/if}
</div>

<style>
  .recommendation-list {
    padding: var(--spacing-md);
  }

  .recommendations-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: var(--spacing-md);
  }

  .loading-container,
  .error-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-xl);
    text-align: center;
    color: var(--subtext0);
  }

  .error-container i {
    font-size: 2rem;
    color: var(--red);
    margin-bottom: var(--spacing-md);
  }

  .empty-tips {
    margin-top: var(--spacing-lg);
    text-align: left;
  }

  .empty-tips h4 {
    font-size: var(--font-size-sm);
    color: var(--subtext0);
    margin-bottom: var(--spacing-sm);
  }

  .empty-tips ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  .empty-tips li {
    display: flex;
    align-items: center;
    gap: var(--spacing-sm);
    padding: var(--spacing-xs) 0;
    font-size: var(--font-size-sm);
    color: var(--subtext1);
  }

  .empty-tips li i {
    width: 20px;
    color: var(--subtext0);
  }

  .refresh-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--spacing-sm);
    width: 100%;
    margin-top: var(--spacing-lg);
    padding: var(--spacing-sm) var(--spacing-md);
    background: var(--surface0);
    border: 1px solid var(--surface1);
    border-radius: var(--radius-md);
    color: var(--subtext0);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .refresh-btn:hover {
    background: var(--surface1);
    color: var(--text);
  }
</style>
```

---

## 4. i18n Strings

### 4.1 Deutsch (de.json)

```json
{
  "recommendations": {
    "title": "Empfehlungen",
    "subtitle": "Basierend auf deinen Interessen",
    "loading": "Empfehlungen werden geladen...",
    "refresh": "Neue Empfehlungen laden",
    "save": "Merken",
    "saved": "Gemerkt",
    "unsave": "Nicht mehr merken",
    "hide": "Nicht mehr anzeigen",

    "empty": {
      "title": "Noch keine Empfehlungen",
      "description": "Wir brauchen mehr Informationen über deine Interessen.",
      "tips_title": "So bekommst du Empfehlungen:",
      "tip_read": "Lies mindestens 5 Artikel",
      "tip_feeds": "Füge mehr Feeds hinzu",
      "tip_ollama": "Stelle sicher, dass Ollama läuft"
    },

    "explanations": {
      "keywords": "Basierend auf: {keywords}",
      "category": "Aus deinem Interessenbereich: {category}",
      "similar": "Ähnlich zu Artikeln die du gelesen hast",
      "popular": "Beliebt diese Woche",
      "explore": "Erweitere deinen Horizont"
    },

    "stats": {
      "title": "Dein Profil",
      "articles_read": "Artikel gelesen",
      "articles_saved": "Artikel gemerkt",
      "top_interests": "Top Interessen"
    }
  }
}
```

### 4.2 English (en.json)

```json
{
  "recommendations": {
    "title": "Recommendations",
    "subtitle": "Based on your interests",
    "loading": "Loading recommendations...",
    "refresh": "Load new recommendations",
    "save": "Save",
    "saved": "Saved",
    "unsave": "Unsave",
    "hide": "Hide",

    "empty": {
      "title": "No recommendations yet",
      "description": "We need more information about your interests.",
      "tips_title": "How to get recommendations:",
      "tip_read": "Read at least 5 articles",
      "tip_feeds": "Add more feeds",
      "tip_ollama": "Make sure Ollama is running"
    },

    "explanations": {
      "keywords": "Based on: {keywords}",
      "category": "From your interest area: {category}",
      "similar": "Similar to articles you've read",
      "popular": "Popular this week",
      "explore": "Expand your horizons"
    },

    "stats": {
      "title": "Your Profile",
      "articles_read": "Articles read",
      "articles_saved": "Articles saved",
      "top_interests": "Top interests"
    }
  }
}
```

---

## 5. Theme Integration

### 5.1 CSS Variables verwendet

```css
/* Alle verwendeten Theme-Variablen */
--base           /* Hintergrund */
--mantle         /* Sekundärer Hintergrund */
--surface0       /* Card Background */
--surface1       /* Border, Divider */
--surface2       /* Hover States */
--text           /* Primärer Text */
--subtext0       /* Sekundärer Text */
--subtext1       /* Tertiärer Text */
--accent         /* Akzentfarbe (Save) */
--red            /* Fehler, Hide */
--yellow         /* Erklärungen */
--shadow-color   /* Schatten */

/* Spacing */
--spacing-xs, --spacing-sm, --spacing-md, --spacing-lg, --spacing-xl

/* Border Radius */
--radius-sm, --radius-md, --radius-lg, --radius-full

/* Font Sizes */
--font-size-xs, --font-size-sm, --font-size-base, --font-size-lg

/* Category Colors (dynamisch) */
--category-1 bis --category-6
```

### 5.2 Dark/Light Mode

Alle Komponenten nutzen CSS-Variablen und sind automatisch Theme-kompatibel.

---

## 6. Integration in MindfuckView

### 6.1 Tab-Struktur (geändert)

```svelte
<script>
  // Neuer Tab für Empfehlungen
  const tabs = [
    { id: 'overview', label: 'Übersicht', icon: 'fa-chart-pie' },
    { id: 'recommendations', label: 'Empfehlungen', icon: 'fa-wand-magic-sparkles' },
    { id: 'blindSpots', label: 'Blinde Flecken', icon: 'fa-eye-slash' },
    { id: 'counterPerspectives', label: 'Gegenperspektiven', icon: 'fa-scale-balanced' },
    { id: 'trends', label: 'Trends', icon: 'fa-chart-line' },
  ];
</script>

<!-- Tab Content -->
{#if activeTab === 'recommendations'}
  <RecommendationList onArticleClick={handleArticleClick} />
{:else if activeTab === 'counterPerspectives'}
  <!-- Bestehende Counter-Perspectives Logik -->
{/if}
```

---

## 7. Accessibility

### 7.1 Anforderungen

- [ ] Alle interaktiven Elemente per Tastatur erreichbar
- [ ] Sinnvolle `aria-label` für Buttons ohne Text
- [ ] Kontrastverhältnis ≥ 4.5:1 für Text
- [ ] Focus-Indikatoren sichtbar
- [ ] Screen-Reader Unterstützung

### 7.2 Keyboard Navigation

| Taste | Aktion |
|-------|--------|
| Tab | Nächstes Element |
| Shift+Tab | Vorheriges Element |
| Enter | Aktivieren (Card = Öffnen, Button = Klick) |
| Space | Button aktivieren |

---

## 8. Responsive Design

### 8.1 Breakpoints

```css
/* Mobile */
@media (max-width: 640px) {
  .recommendations-grid {
    grid-template-columns: 1fr;
  }

  .recommendation-card {
    /* Compact Mode */
  }
}

/* Tablet */
@media (min-width: 641px) and (max-width: 1024px) {
  .recommendations-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}

/* Desktop */
@media (min-width: 1025px) {
  .recommendations-grid {
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  }
}
```

---

## 9. Animations

### 9.1 Card Interactions

```css
/* Hover */
.recommendation-card {
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.recommendation-card:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 24px var(--shadow-color);
}

/* Save Animation */
.save-btn.saving {
  animation: pulse 0.3s ease;
}

@keyframes pulse {
  0% { transform: scale(1); }
  50% { transform: scale(1.1); }
  100% { transform: scale(1); }
}

/* Hide Animation */
.recommendation-card.hiding {
  animation: fadeOut 0.3s ease forwards;
}

@keyframes fadeOut {
  to {
    opacity: 0;
    transform: translateX(-20px);
  }
}
```

---

## 10. Testing Checklist

### 10.1 Funktional

- [ ] Empfehlungen werden geladen
- [ ] Save Button speichert Artikel
- [ ] Hide Button entfernt Artikel aus Liste
- [ ] Klick auf Card öffnet Artikel
- [ ] Refresh lädt neue Empfehlungen
- [ ] Empty State wird korrekt angezeigt
- [ ] Error State wird bei Fehler angezeigt

### 10.2 Visual

- [ ] Card-Design entspricht Spec
- [ ] Theme-Variablen werden korrekt angewendet
- [ ] Responsive auf allen Breakpoints
- [ ] Animationen sind flüssig

### 10.3 Accessibility

- [ ] Keyboard-Navigation funktioniert
- [ ] Screen-Reader liest Inhalte korrekt
- [ ] Kontrast ausreichend
