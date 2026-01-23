/**
 * Gemeinsame Formatierungs-Funktionen für Artikel-Darstellungen
 */

// ============================================================
// Datum-Formatierung
// ============================================================

/**
 * Formatiert ein Datum relativ zur aktuellen Zeit
 * @example "vor 5 Min", "vor 2 Std", "vor 3 Tagen", "5. Jan"
 */
export function formatRelativeDate(dateStr: string | null, locale: string = 'de'): string {
  if (!dateStr) return "";

  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / (1000 * 60));
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  const isGerman = locale.startsWith('de');

  if (diffMins < 60) {
    return isGerman ? `vor ${diffMins} Min` : `${diffMins} min ago`;
  } else if (diffHours < 24) {
    return isGerman ? `vor ${diffHours} Std` : `${diffHours}h ago`;
  } else if (diffDays < 7) {
    return isGerman ? `vor ${diffDays} Tagen` : `${diffDays}d ago`;
  } else {
    return date.toLocaleDateString(isGerman ? "de-DE" : "en-US", {
      day: "numeric",
      month: "short",
    });
  }
}

/**
 * Formatiert ein Datum kurz (Tag + Monat)
 * @example "5. Jan", "12. Dez"
 */
export function formatShortDate(dateStr: string | null, locale: string = 'de'): string {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  const isGerman = locale.startsWith('de');
  return date.toLocaleDateString(isGerman ? "de-DE" : "en-US", {
    day: "numeric",
    month: "short",
  });
}

/**
 * Formatiert ein Datum vollständig
 * @example "Montag, 5. Januar 2025, 14:32"
 */
export function formatFullDate(dateStr: string | null, locale: string = 'de'): string {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  const isGerman = locale.startsWith('de');
  return date.toLocaleDateString(isGerman ? "de-DE" : "en-US", {
    weekday: "long",
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * Formatiert ein Datum für Änderungs-Anzeige (TT.MM.JJJJ)
 * @example "05.01.2025"
 */
export function formatChangedDate(dateStr: string | null): string {
  if (!dateStr) return '-';
  const date = new Date(dateStr);
  return date.toLocaleDateString('de-DE', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
  });
}

// ============================================================
// Status-Funktionen
// ============================================================

export type ArticleStatus = 'concealed' | 'illuminated' | 'golden_apple';

/**
 * Gibt das Font Awesome Icon für einen Status zurück
 */
export function getStatusIcon(status: string): string {
  switch (status) {
    case "concealed": return "fa-solid fa-eye-slash";
    case "illuminated": return "fa-solid fa-eye";
    case "golden_apple": return "fa-solid fa-apple-whole";
    default: return "fa-solid fa-check";
  }
}

/**
 * Gibt die CSS-Klasse für die Status-Farbe zurück
 */
export function getStatusColorClass(status: string): string {
  switch (status) {
    case "concealed": return "status-concealed";
    case "illuminated": return "status-illuminated";
    case "golden_apple": return "status-golden_apple";
    default: return "status-illuminated";
  }
}

// ============================================================
// Bias-Funktionen
// ============================================================

/**
 * Gibt das Font Awesome Icon für einen Bias-Wert zurück
 */
export function getBiasIcon(bias: number | null): string {
  if (bias === null) return "";
  switch (bias) {
    case -2: return "fa-solid fa-angles-left";
    case -1: return "fa-solid fa-angle-left";
    case 0: return "fa-solid fa-circle";
    case 1: return "fa-solid fa-angle-right";
    case 2: return "fa-solid fa-angles-right";
    default: return "fa-solid fa-circle";
  }
}

/**
 * Gibt das Label für einen Bias-Wert zurück
 */
export function getBiasLabel(bias: number | null, locale: string = 'de'): string {
  if (bias === null) return "";
  const isGerman = locale.startsWith('de');
  switch (bias) {
    case -2: return isGerman ? "Stark links" : "Strong left";
    case -1: return isGerman ? "Leicht links" : "Lean left";
    case 0: return isGerman ? "Neutral" : "Neutral";
    case 1: return isGerman ? "Leicht rechts" : "Lean right";
    case 2: return isGerman ? "Stark rechts" : "Strong right";
    default: return "";
  }
}

/**
 * Gibt die CSS-Klasse für die Bias-Richtung zurück
 */
export function getBiasDirectionClass(bias: number | null): string {
  if (bias === null || bias === 0) return "";
  return bias < 0 ? "bias-left" : "bias-right";
}

// ============================================================
// Similarity / Score
// ============================================================

/**
 * Formatiert einen Similarity-Wert als Prozent
 * @example 0.85 -> "85%"
 */
export function formatSimilarity(similarity: number): string {
  return `${Math.round(similarity * 100)}%`;
}

// ============================================================
// Text-Utilities
// ============================================================

/**
 * Kürzt Text auf eine maximale Länge mit Ellipsis
 */
export function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
}

/**
 * Entfernt HTML-Tags aus einem String.
 * Verwendet textContent zur sicheren Text-Extraktion.
 */
export function stripHtml(html: string): string {
  if (typeof document === 'undefined') {
    // SSR-safe: einfache Regex-Lösung
    return html.replace(/<[^>]*>/g, '');
  }
  const template = document.createElement('template');
  template.innerHTML = html;
  return template.content.textContent || '';
}
