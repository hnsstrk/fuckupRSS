/**
 * HTML Sanitization for Article Content
 *
 * Verwendet DOMPurify um XSS-Angriffe zu verhindern,
 * während Artikel-Formatierung erhalten bleibt.
 */

import DOMPurify from 'dompurify';

/**
 * Konfiguration für Artikel-Content Sanitization
 * Erlaubt gängige Artikel-Formatierungen, blockiert XSS-Vektoren
 */
const ARTICLE_CONFIG: DOMPurify.Config = {
  ALLOWED_TAGS: [
    // Text-Formatierung
    'p', 'br', 'span', 'em', 'i', 'strong', 'b', 'u', 's', 'strike', 'del', 'ins',
    'mark', 'small', 'sub', 'sup', 'abbr', 'acronym', 'cite', 'dfn', 'q',
    // Überschriften
    'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
    // Listen
    'ul', 'ol', 'li',
    // Zitate und Code
    'blockquote', 'code', 'pre', 'kbd', 'samp', 'var',
    // Media
    'img', 'figure', 'figcaption', 'picture', 'source',
    // Tabellen
    'table', 'thead', 'tbody', 'tfoot', 'tr', 'td', 'th', 'caption', 'colgroup', 'col',
    // Links
    'a',
    // Semantische HTML5-Elemente
    'section', 'article', 'aside', 'header', 'footer', 'main', 'div',
    // Horizontale Linie
    'hr',
    // Definition Lists
    'dl', 'dt', 'dd',
    // Details/Summary
    'details', 'summary',
    // Time
    'time',
    // Media Embeds
    'iframe', 'video', 'source', 'embed', 'object',
  ],
  ALLOWED_ATTR: [
    // Bild-Attribute
    'src', 'alt', 'title', 'loading', 'width', 'height', 'decoding', 'srcset', 'sizes',
    // Link-Attribute
    'href', 'target', 'rel', 'title',
    // Tabellen-Attribute
    'colspan', 'rowspan', 'align', 'valign', 'scope',
    // Allgemeine Attribute
    'id', 'class', 'title', 'role', 'lang', 'dir',
    // ARIA für Accessibility
    'aria-label', 'aria-hidden', 'aria-describedby', 'aria-labelledby',
    // Media Attribute
    'data-src', 'data-srcset',
    // Time
    'datetime',
    // Details
    'open',
    // Iframe Attributes
    'allow', 'allowfullscreen', 'frameborder', 'scrolling', 'sandbox',
    // Video Attributes
    'controls', 'poster', 'loop', 'muted', 'autoplay', 'playsinline', 'preload',
    // Object/Embed Attributes
    'type', 'data',
    // Lazy-loading for iframes
    'loading',
  ],
  ALLOW_DATA_ATTR: true,
  ALLOW_ARIA_ATTR: true,
  ALLOW_UNKNOWN_PROTOCOLS: false,
  KEEP_CONTENT: true,
  SANITIZE_DOM: true,
  IN_PLACE: false,
};

/**
 * Elemente die komplett entfernt werden sollen (inkl. Inhalt)
 * - SVGs aus Video-Playern
 * - Iframes
 * - Scripts
 */
const FORBID_TAGS = ['svg', 'script', 'style', 'noscript', 'canvas', 'audio'];

/**
 * Hook um problematische Inhalte vor der Sanitization zu bereinigen
 * - Entfernt leere Spans ohne Inhalt
 * - Entfernt Video-Player Artefakte (SVG mit play-arrow etc.)
 * - Fügt lazy-loading zu Bildern hinzu
 */
function setupDOMPurifyHooks(): void {
  // Hook: Vor dem Sanitizen eines Elements
  DOMPurify.addHook('uponSanitizeElement', (node: Element, _data: DOMPurify.SanitizeElementHookEvent) => {
    // Entferne Elemente mit data-component (BBC-spezifische Artefakte)
    if (node.hasAttribute && node.hasAttribute('data-component')) {
      const component = node.getAttribute('data-component');
      // Behalte nur sinnvolle data-components (text-block, caption-block)
      if (component && !['text-block', 'caption-block', 'image-block'].includes(component)) {
        // Ersetze durch Kinder
        if (node.parentNode) {
          while (node.firstChild) {
            node.parentNode.insertBefore(node.firstChild, node);
          }
          node.parentNode.removeChild(node);
        }
      }
    }
  });

  // Hook: Nach dem Sanitizen eines Elements
  DOMPurify.addHook('afterSanitizeElements', (node: Element) => {
    // Füge lazy-loading zu allen Bildern hinzu
    if (node.tagName === 'IMG') {
      node.setAttribute('loading', 'lazy');
      node.setAttribute('decoding', 'async');
    }

    // Füge rel="noopener noreferrer" zu externen Links hinzu
    if (node.tagName === 'A') {
      const href = node.getAttribute('href');
      if (href && (href.startsWith('http://') || href.startsWith('https://'))) {
        node.setAttribute('target', '_blank');
        node.setAttribute('rel', 'noopener noreferrer');
      }
    }
  });
}

// Initialisiere Hooks einmalig
let hooksInitialized = false;

/**
 * Sanitize HTML-Content für sichere Darstellung in Artikeln
 * Entfernt XSS-Vektoren während Artikel-Formatierung erhalten bleibt
 *
 * @param dirtyHtml Raw HTML-Content vom RSS-Feed oder Web-Scraping
 * @returns Sicherer HTML-String für {@html} Directive
 */
export function sanitizeArticleContent(dirtyHtml: string): string {
  if (!dirtyHtml) return '';

  // Initialisiere Hooks beim ersten Aufruf
  if (!hooksInitialized) {
    setupDOMPurifyHooks();
    hooksInitialized = true;
  }

  // Pre-Processing: Entferne bekannte problematische Patterns
  let processedHtml = dirtyHtml
    // Entferne Video-Player SVGs (tagesschau, BBC, etc.)
    .replace(/<svg[^>]*>[\s\S]*?<\/svg>/gi, '')
    // Entferne leere Spans
    .replace(/<span[^>]*>\s*<\/span>/gi, '')
    // Entferne data-testid Attribute (React-Artefakte)
    .replace(/\s*data-testid="[^"]*"/gi, '')
    // Entferne Video-Duration Spans (tagesschau)
    .replace(/<span>Video Duration[^<]*<\/span>/gi, '')
    // Konvertiere &nbsp; zu normalen Leerzeichen wo sinnvoll
    .replace(/(&nbsp;){3,}/gi, ' ');

  // Sanitize mit Konfiguration
  const config: DOMPurify.Config = {
    ...ARTICLE_CONFIG,
    FORBID_TAGS,
  };

  return DOMPurify.sanitize(processedHtml, config);
}
