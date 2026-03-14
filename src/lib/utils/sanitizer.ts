/**
 * HTML Sanitization for Article Content & Markdown Rendering
 *
 * Verwendet DOMPurify um XSS-Angriffe zu verhindern,
 * während Artikel-Formatierung erhalten bleibt.
 *
 * renderMarkdown() ist die zentrale Funktion fuer alle LLM-generierten
 * Texte (Briefings, Perspektiven, Summaries) — Markdown → HTML → Sanitize.
 */

import DOMPurify from "dompurify";
import type { Config, UponSanitizeElementHookEvent } from "dompurify";
import { marked } from "marked";

// marked: GFM aktivieren, Zeilenumbrueche beibehalten
marked.setOptions({
  breaks: true,
  gfm: true,
});

/**
 * Konfiguration für Artikel-Content Sanitization
 * Erlaubt gängige Artikel-Formatierungen, blockiert XSS-Vektoren
 */
const ARTICLE_CONFIG: Config = {
  ALLOWED_TAGS: [
    // Text-Formatierung
    "p",
    "br",
    "span",
    "em",
    "i",
    "strong",
    "b",
    "u",
    "s",
    "strike",
    "del",
    "ins",
    "mark",
    "small",
    "sub",
    "sup",
    "abbr",
    "acronym",
    "cite",
    "dfn",
    "q",
    // Überschriften
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    // Listen
    "ul",
    "ol",
    "li",
    // Zitate und Code
    "blockquote",
    "code",
    "pre",
    "kbd",
    "samp",
    "var",
    // Media
    "img",
    "figure",
    "figcaption",
    "picture",
    "source",
    // Tabellen
    "table",
    "thead",
    "tbody",
    "tfoot",
    "tr",
    "td",
    "th",
    "caption",
    "colgroup",
    "col",
    // Links
    "a",
    // Semantische HTML5-Elemente
    "section",
    "article",
    "aside",
    "header",
    "footer",
    "main",
    "div",
    // Horizontale Linie
    "hr",
    // Definition Lists
    "dl",
    "dt",
    "dd",
    // Details/Summary
    "details",
    "summary",
    // Time
    "time",
    // Media Embeds: iframe, video, embed, object entfernt (Security Hardening)
    // - 0 Vorkommen in 10.162 Artikeln, aber klassische XSS-Vektoren
    // - iframe: kann beliebige externe Inhalte laden
    // - embed/object: koennen Plugins/beliebige Inhalte ausfuehren
    // - video: in FORBID_TAGS verschoben (audio war bereits dort)
  ],
  ALLOWED_ATTR: [
    // Bild-Attribute
    "src",
    "alt",
    "title",
    "loading",
    "width",
    "height",
    "decoding",
    "srcset",
    "sizes",
    // Link-Attribute
    "href",
    "target",
    "rel",
    "title",
    // Tabellen-Attribute
    "colspan",
    "rowspan",
    "align",
    "valign",
    "scope",
    // Allgemeine Attribute
    "id",
    "class",
    "title",
    "role",
    "lang",
    "dir",
    // ARIA für Accessibility
    "aria-label",
    "aria-hidden",
    "aria-describedby",
    "aria-labelledby",
    // Media Attribute
    "data-src",
    "data-srcset",
    // Time
    "datetime",
    // Details
    "open",
    // Iframe/Video/Object Attribute entfernt (Security Hardening):
    // allow, allowfullscreen, frameborder, scrolling, sandbox (iframe)
    // controls, poster, loop, muted, autoplay, playsinline, preload (video)
    // data (object) — nicht mehr noetig, da Tags in FORBID_TAGS
    // type beibehalten (fuer <source> in <picture>)
    "type",
  ],
  // data-* Attribute deaktiviert (Security Hardening):
  // Koennen mit JS-Frameworks (Alpine.js, HTMX) als XSS-Vektor missbraucht werden.
  // data-src und data-srcset sind explizit in ALLOWED_ATTR gelistet.
  ALLOW_DATA_ATTR: false,
  ALLOW_ARIA_ATTR: true,
  ALLOW_UNKNOWN_PROTOCOLS: false,
  KEEP_CONTENT: true,
  SANITIZE_DOM: true,
  IN_PLACE: false,
};

/**
 * Elemente die komplett entfernt werden sollen (inkl. Inhalt)
 * - script, style, noscript: Code-Ausfuehrung
 * - svg, canvas: koennen Event-Handler enthalten
 * - audio, video: Media-Elemente (nicht in RSS-Artikeln verwendet)
 * - iframe, embed, object: klassische XSS-Vektoren (0 Vorkommen in 10.162 Artikeln)
 */
const FORBID_TAGS = [
  "svg",
  "script",
  "style",
  "noscript",
  "canvas",
  "audio",
  "video",
  "iframe",
  "embed",
  "object",
];

/**
 * Hook um problematische Inhalte vor der Sanitization zu bereinigen
 * - Entfernt leere Spans ohne Inhalt
 * - Entfernt Video-Player Artefakte (SVG mit play-arrow etc.)
 * - Fügt lazy-loading zu Bildern hinzu
 */
function setupDOMPurifyHooks(): void {
  // Hook: Vor dem Sanitizen eines Elements
  DOMPurify.addHook(
    "uponSanitizeElement",
    (currentNode: Node, _data: UponSanitizeElementHookEvent) => {
      // Entferne Elemente mit data-component (BBC-spezifische Artefakte)
      const el = currentNode as Element;
      if (el.hasAttribute && el.hasAttribute("data-component")) {
        const component = el.getAttribute("data-component");
        // Behalte nur sinnvolle data-components (text-block, caption-block)
        if (component && !["text-block", "caption-block", "image-block"].includes(component)) {
          // Ersetze durch Kinder
          if (currentNode.parentNode) {
            while (currentNode.firstChild) {
              currentNode.parentNode.insertBefore(currentNode.firstChild, currentNode);
            }
            currentNode.parentNode.removeChild(currentNode);
          }
        }
      }
    },
  );

  // Hook: Nach dem Sanitizen eines Elements
  DOMPurify.addHook("afterSanitizeElements", (currentNode: Node) => {
    const el = currentNode as Element;
    // Füge lazy-loading zu allen Bildern hinzu
    if (el.tagName === "IMG") {
      el.setAttribute("loading", "lazy");
      el.setAttribute("decoding", "async");
    }

    // Füge rel="noopener noreferrer" zu externen Links hinzu
    if (el.tagName === "A") {
      const href = el.getAttribute("href");
      if (href && (href.startsWith("http://") || href.startsWith("https://"))) {
        el.setAttribute("target", "_blank");
        el.setAttribute("rel", "noopener noreferrer");
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
  if (!dirtyHtml) return "";

  // Initialisiere Hooks beim ersten Aufruf
  if (!hooksInitialized) {
    setupDOMPurifyHooks();
    hooksInitialized = true;
  }

  // Pre-Processing: Entferne bekannte problematische Patterns
  const processedHtml = dirtyHtml
    // Entferne Video-Player SVGs (tagesschau, BBC, etc.)
    .replace(/<svg[^>]*>[\s\S]*?<\/svg>/gi, "")
    // Entferne leere Spans
    .replace(/<span[^>]*>\s*<\/span>/gi, "")
    // Entferne data-testid Attribute (React-Artefakte)
    .replace(/\s*data-testid="[^"]*"/gi, "")
    // Entferne Video-Duration Spans (tagesschau)
    .replace(/<span>Video Duration[^<]*<\/span>/gi, "")
    // Konvertiere &nbsp; zu normalen Leerzeichen wo sinnvoll
    .replace(/(&nbsp;){3,}/gi, " ");

  // Sanitize mit Konfiguration
  const config: Config = {
    ...ARTICLE_CONFIG,
    FORBID_TAGS,
  };

  return DOMPurify.sanitize(processedHtml, config) as string;
}

/**
 * Zentrale Markdown-Rendering-Funktion fuer alle LLM-generierten Texte.
 * Konvertiert Markdown zu HTML und sanitized das Ergebnis mit DOMPurify.
 *
 * Verwendung in Svelte-Komponenten:
 *   {@html renderMarkdown(text)}
 *
 * @param markdown Rohtext mit Markdown-Formatierung (vom LLM)
 * @returns Sicherer HTML-String fuer {@html} Directive
 */
export function renderMarkdown(markdown: string): string {
  if (!markdown) return "";
  const html = marked.parse(markdown) as string;
  return sanitizeArticleContent(html);
}

/**
 * Rendert Inline-Markdown (bold, italic, code, links) ohne Block-Elemente.
 * Fuer Ueberschriften und andere Inline-Kontexte wo kein <p>/<ol>/<ul> erwuenscht ist.
 */
export function renderMarkdownInline(markdown: string): string {
  if (!markdown) return "";
  const html = marked.parseInline(markdown) as string;
  return sanitizeArticleContent(html);
}
