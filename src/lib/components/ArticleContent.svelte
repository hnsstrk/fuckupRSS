<script lang="ts">
  import { _ } from "svelte-i18n";
  import { sanitizeArticleContent } from "$lib/utils/sanitizer";

  let {
    contentFull,
    contentRaw,
  }: {
    contentFull: string | null;
    contentRaw: string | null;
  } = $props();
</script>

<div class="content-section">
  <div class="section-content">
    <!-- eslint-disable svelte/no-at-html-tags -->
    <article class="article-body">
      {#if contentFull}
        {@html sanitizeArticleContent(contentFull)}
      {:else if contentRaw}
        {@html sanitizeArticleContent(contentRaw)}
      {:else}
        <p class="no-content">
          {$_("articleView.noContent")}
        </p>
      {/if}
    </article>
  </div>
</div>

<style>
  /* ===========================================
     Content Section - Mobile First
     =========================================== */
  .content-section {
    padding: 1rem;
  }

  @media (min-width: 640px) {
    .content-section {
      padding: 1.5rem;
    }
  }

  .section-content {
    max-width: 48rem;
    margin: 0 auto;
  }

  /* ===========================================
     Article Body - Base Typography
     =========================================== */
  .article-body {
    color: var(--text-primary);
    line-height: 1.75;
    font-size: 1rem;
    word-wrap: break-word;
    overflow-wrap: break-word;
    hyphens: auto;
  }

  @media (min-width: 640px) {
    .article-body {
      font-size: 1.0625rem;
      line-height: 1.8;
    }
  }

  /* First element should not have top margin */
  .article-body :global(> *:first-child) {
    margin-top: 0;
  }

  /* ===========================================
     Headings - h1 to h6
     =========================================== */
  .article-body :global(h1) {
    font-size: 1.75rem;
    font-weight: 700;
    color: var(--text-primary);
    margin-top: 2rem;
    margin-bottom: 1rem;
    line-height: 1.3;
    border-bottom: 1px solid var(--border-muted);
    padding-bottom: 0.5rem;
  }

  .article-body :global(h2) {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-top: 1.75rem;
    margin-bottom: 0.875rem;
    line-height: 1.35;
  }

  .article-body :global(h3) {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
    line-height: 1.4;
  }

  .article-body :global(h4) {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-top: 1.25rem;
    margin-bottom: 0.625rem;
    line-height: 1.4;
  }

  .article-body :global(h5) {
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-secondary);
    margin-top: 1rem;
    margin-bottom: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.025em;
  }

  .article-body :global(h6) {
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-muted);
    margin-top: 1rem;
    margin-bottom: 0.5rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  /* ===========================================
     Text Formatting - Bold, Italic, etc.
     =========================================== */
  .article-body :global(strong),
  .article-body :global(b) {
    font-weight: 600;
    color: var(--text-primary);
  }

  .article-body :global(em),
  .article-body :global(i) {
    font-style: italic;
  }

  .article-body :global(u) {
    text-decoration: underline;
    text-decoration-color: var(--accent-primary);
    text-underline-offset: 2px;
  }

  .article-body :global(s),
  .article-body :global(del),
  .article-body :global(strike) {
    text-decoration: line-through;
    color: var(--text-muted);
  }

  .article-body :global(mark) {
    background-color: var(--golden-apple-color);
    color: var(--text-on-accent);
    padding: 0.0625rem 0.25rem;
    border-radius: 2px;
  }

  .article-body :global(small) {
    font-size: 0.875em;
    color: var(--text-secondary);
  }

  .article-body :global(sub),
  .article-body :global(sup) {
    font-size: 0.75em;
    line-height: 0;
    position: relative;
    vertical-align: baseline;
  }

  .article-body :global(sup) {
    top: -0.5em;
  }

  .article-body :global(sub) {
    bottom: -0.25em;
  }

  .article-body :global(cite) {
    font-style: italic;
    color: var(--text-secondary);
  }

  /* ===========================================
     Paragraphs
     =========================================== */
  .article-body :global(p) {
    margin: 0 0 1.25rem 0;
  }

  .article-body :global(p:last-child) {
    margin-bottom: 0;
  }

  /* ===========================================
     Links
     =========================================== */
  .article-body :global(a) {
    color: var(--accent-info);
    text-decoration: none;
    transition: color 0.15s ease;
  }

  .article-body :global(a:hover) {
    color: var(--accent-primary);
    text-decoration: underline;
  }

  .article-body :global(a:visited) {
    color: var(--accent-secondary);
  }

  /* External links indicator */
  .article-body :global(a[target="_blank"])::after {
    content: " \2197";
    font-size: 0.75em;
    color: var(--text-muted);
  }

  /* ===========================================
     Lists - Unordered, Ordered
     =========================================== */
  .article-body :global(ul),
  .article-body :global(ol) {
    margin: 1rem 0 1.25rem 0;
    padding-left: 1.5rem;
  }

  .article-body :global(ul) {
    list-style-type: disc;
  }

  .article-body :global(ol) {
    list-style-type: decimal;
  }

  .article-body :global(li) {
    margin: 0.375rem 0;
    padding-left: 0.25rem;
  }

  .article-body :global(li > p) {
    margin-bottom: 0.5rem;
  }

  /* Nested lists */
  .article-body :global(ul ul),
  .article-body :global(ol ul) {
    list-style-type: circle;
    margin: 0.375rem 0;
  }

  .article-body :global(ul ul ul),
  .article-body :global(ol ul ul) {
    list-style-type: square;
  }

  .article-body :global(ol ol),
  .article-body :global(ul ol) {
    list-style-type: lower-alpha;
    margin: 0.375rem 0;
  }

  /* ===========================================
     Blockquotes
     =========================================== */
  .article-body :global(blockquote) {
    margin: 1.5rem 0;
    padding: 0.75rem 1rem 0.75rem 1.25rem;
    border-left: 4px solid var(--accent-primary);
    background-color: var(--bg-surface);
    border-radius: 0 0.375rem 0.375rem 0;
    color: var(--text-secondary);
    font-style: italic;
  }

  .article-body :global(blockquote p) {
    margin-bottom: 0.75rem;
  }

  .article-body :global(blockquote p:last-child) {
    margin-bottom: 0;
  }

  /* Nested blockquotes */
  .article-body :global(blockquote blockquote) {
    margin: 0.75rem 0;
    border-left-color: var(--accent-secondary);
  }

  .article-body :global(blockquote cite) {
    display: block;
    margin-top: 0.75rem;
    font-size: 0.875rem;
    color: var(--text-muted);
    font-style: normal;
  }

  .article-body :global(blockquote cite::before) {
    content: "\2014 ";
  }

  /* ===========================================
     Images and Figures
     =========================================== */
  .article-body :global(img) {
    max-width: 100%;
    height: auto;
    border-radius: 0.5rem;
    display: block;
    margin: 1.25rem auto;
    background-color: var(--bg-surface);
  }

  .article-body :global(figure) {
    margin: 1.5rem 0;
    padding: 0;
    text-align: center;
  }

  .article-body :global(figure img) {
    margin: 0 auto;
  }

  .article-body :global(figcaption) {
    margin-top: 0.75rem;
    font-size: 0.875rem;
    color: var(--text-muted);
    font-style: italic;
    line-height: 1.5;
    text-align: center;
    padding: 0 1rem;
  }

  /* ===========================================
     Horizontal Rules
     =========================================== */
  .article-body :global(hr) {
    margin: 2rem 0;
    border: none;
    border-top: 1px solid var(--border-default);
  }

  /* ===========================================
     Time element
     =========================================== */
  .article-body :global(time) {
    color: var(--text-muted);
    font-size: 0.875em;
  }

  /* ===========================================
     No Content State
     =========================================== */
  .no-content {
    color: var(--text-muted);
    font-style: italic;
    margin: 0;
    padding: 2rem;
    text-align: center;
  }

  /* ===========================================
     Print Styles
     =========================================== */
  @media print {
    .content-section {
      padding: 1rem 0;
    }

    .section-content {
      max-width: 100%;
    }

    .article-body {
      font-size: 11pt;
      line-height: 1.6;
      color: black;
    }

    .article-body :global(a) {
      color: black;
      text-decoration: underline;
    }

    .article-body :global(a[target="_blank"])::after {
      content: " (" attr(href) ")";
      font-size: 9pt;
      color: #666;
    }

    .article-body :global(img) {
      max-width: 100%;
      page-break-inside: avoid;
    }

    .article-body :global(blockquote) {
      page-break-inside: avoid;
      border-color: #ccc;
    }

    .article-body :global(h1),
    .article-body :global(h2),
    .article-body :global(h3),
    .article-body :global(h4) {
      page-break-after: avoid;
      color: black;
    }
  }
</style>
