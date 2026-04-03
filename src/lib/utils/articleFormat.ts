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
export function formatRelativeDate(dateStr: string | null, locale: string = "de"): string {
  if (!dateStr) return "";

  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / (1000 * 60));
  const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  const isGerman = locale.startsWith("de");

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
export function formatShortDate(dateStr: string | null, locale: string = "de"): string {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  const isGerman = locale.startsWith("de");
  return date.toLocaleDateString(isGerman ? "de-DE" : "en-US", {
    day: "numeric",
    month: "short",
  });
}

/**
 * Formatiert ein Datum vollständig
 * @example "Montag, 5. Januar 2025, 14:32"
 */
export function formatFullDate(dateStr: string | null, locale: string = "de"): string {
  if (!dateStr) return "";
  const date = new Date(dateStr);
  const isGerman = locale.startsWith("de");
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
  if (!dateStr) return "-";
  const date = new Date(dateStr);
  return date.toLocaleDateString("de-DE", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
}

/**
 * Formatiert ein Datum mit Uhrzeit kurz (TT.MM.JJJJ, HH:MM)
 * @example "05.01.2025, 14:32"
 */
export function formatDateTimeShort(dateStr: string | null, locale: string = "de"): string {
  if (!dateStr) return "-";
  const date = new Date(dateStr);
  const isGerman = locale.startsWith("de");
  return date.toLocaleDateString(isGerman ? "de-DE" : "en-US", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// ============================================================
// Status-Funktionen
// ============================================================

export type ArticleStatus = "concealed" | "illuminated" | "golden_apple";

/**
 * Gibt das Font Awesome Icon für einen Status zurück
 */
export function getStatusIcon(status: string): string {
  switch (status) {
    case "concealed":
      return "fa-solid fa-eye-slash";
    case "illuminated":
      return "fa-solid fa-eye";
    case "golden_apple":
      return "fa-solid fa-apple-whole";
    default:
      return "fa-solid fa-check";
  }
}

/**
 * Gibt die CSS-Klasse für die Status-Farbe zurück
 */
export function getStatusColorClass(status: string): string {
  switch (status) {
    case "concealed":
      return "status-concealed";
    case "illuminated":
      return "status-illuminated";
    case "golden_apple":
      return "status-golden_apple";
    default:
      return "status-illuminated";
  }
}

// ============================================================
// Bias-Funktionen
// ============================================================

/**
 * Gibt die CSS-Klasse oder CSS-Variable für die Bias-Farbe zurück.
 * @param bias - Bias-Wert (-2 bis +2 oder fließend)
 * @param format - 'class' für CSS-Klassen-Suffix, 'variable' für CSS-Variable
 */
export function getBiasColor(
  bias: number | null,
  format: "class" | "variable" = "variable",
): string {
  if (format === "class") {
    if (bias === null) return "neutral";
    if (bias <= -2) return "strong-left";
    if (bias === -1) return "lean-left";
    if (bias === 0) return "center";
    if (bias === 1) return "lean-right";
    return "strong-right";
  }
  // format === 'variable'
  if (bias === null) return "var(--text-muted)";
  if (bias <= -1.5) return "var(--bias-strong-left)";
  if (bias <= -0.5) return "var(--bias-lean-left)";
  if (bias <= 0.5) return "var(--bias-center)";
  if (bias <= 1.5) return "var(--bias-lean-right)";
  return "var(--bias-strong-right)";
}

/**
 * Gibt das Font Awesome Icon für einen Bias-Wert zurück
 */
export function getBiasIcon(bias: number | null): string {
  if (bias === null) return "";
  switch (bias) {
    case -2:
      return "fa-solid fa-angles-left";
    case -1:
      return "fa-solid fa-angle-left";
    case 0:
      return "fa-solid fa-circle";
    case 1:
      return "fa-solid fa-angle-right";
    case 2:
      return "fa-solid fa-angles-right";
    default:
      return "fa-solid fa-circle";
  }
}

/**
 * Gibt das Label für einen ganzzahligen Bias-Wert zurück.
 * Mit optionalem `t`-Parameter für i18n (z.B. svelte-i18n `$_`).
 */
export function getBiasLabel(
  bias: number | null,
  locale: string = "de",
  t?: (key: string) => string,
): string {
  if (bias === null) return "";
  if (t) {
    switch (bias) {
      case -2:
        return t("articleView.biasStrongLeft");
      case -1:
        return t("articleView.biasLeanLeft");
      case 0:
        return t("articleView.biasNeutral");
      case 1:
        return t("articleView.biasLeanRight");
      case 2:
        return t("articleView.biasStrongRight");
      default:
        return "";
    }
  }
  // Fallback ohne i18n (bestehende Callsites)
  const isGerman = locale.startsWith("de");
  switch (bias) {
    case -2:
      return isGerman ? "Stark links" : "Strong left";
    case -1:
      return isGerman ? "Leicht links" : "Lean left";
    case 0:
      return isGerman ? "Neutral" : "Neutral";
    case 1:
      return isGerman ? "Leicht rechts" : "Lean right";
    case 2:
      return isGerman ? "Stark rechts" : "Strong right";
    default:
      return "";
  }
}

/**
 * Gibt das Label für einen Fließkomma-Bias-Durchschnitt zurück (z.B. für Trends).
 * Verwendet Schwellenwerte statt ganzzahliger switch-cases.
 */
export function getBiasRangeLabel(
  bias: number | null,
  t?: (key: string) => string,
): string {
  if (bias === null) return "";
  if (t) {
    if (bias <= -1.5) return t("mindfuck.bias.strongLeft");
    if (bias <= -0.5) return t("mindfuck.bias.left");
    if (bias <= 0.5) return t("mindfuck.bias.neutral");
    if (bias <= 1.5) return t("mindfuck.bias.right");
    return t("mindfuck.bias.strongRight");
  }
  // Fallback ohne i18n
  if (bias <= -1.5) return "Strong left";
  if (bias <= -0.5) return "Lean left";
  if (bias <= 0.5) return "Neutral";
  if (bias <= 1.5) return "Lean right";
  return "Strong right";
}

/**
 * Gibt die CSS-Klasse für die Bias-Richtung zurück
 */
export function getBiasDirectionClass(bias: number | null): string {
  if (bias === null || bias === 0) return "";
  return bias < 0 ? "bias-left" : "bias-right";
}

// ============================================================
// Sachlichkeit-Funktionen
// ============================================================

/**
 * Gibt das Label für einen Sachlichkeits-Wert zurück.
 * Mit optionalem `t`-Parameter für i18n (z.B. svelte-i18n `$_`).
 */
export function getSachlichkeitLabel(
  s: number | null,
  locale: string = "de",
  t?: (key: string) => string,
): string {
  if (s === null) return "";
  if (t) {
    switch (s) {
      case 0:
        return t("articleView.sachHighlyEmotional");
      case 1:
        return t("articleView.sachEmotional");
      case 2:
        return t("articleView.sachMixed");
      case 3:
        return t("articleView.sachMostlyObjective");
      case 4:
        return t("articleView.sachObjective");
      default:
        return "";
    }
  }
  // Fallback ohne i18n (bestehende Callsites)
  const isGerman = locale.startsWith("de");
  switch (s) {
    case 0:
      return isGerman ? "Hoch emotional" : "Highly emotional";
    case 1:
      return isGerman ? "Emotional" : "Emotional";
    case 2:
      return isGerman ? "Gemischt" : "Mixed";
    case 3:
      return isGerman ? "Überwiegend sachlich" : "Mostly objective";
    case 4:
      return isGerman ? "Sachlich" : "Objective";
    default:
      return "";
  }
}

/**
 * Gibt die CSS-Klasse für einen Sachlichkeits-Wert zurück
 */
export function getSachlichkeitColor(s: number | null): string {
  if (s === null) return "neutral";
  if (s <= 1) return "emotional";
  if (s === 2) return "mixed";
  return "objective";
}

/**
 * Gibt das Font Awesome Icon für einen Sachlichkeits-Wert zurück
 */
export function getSachlichkeitIcon(s: number | null): string {
  if (s === null) return "fa-face-meh";
  if (s <= 1) return "fa-heart";
  if (s === 2) return "fa-face-meh";
  return "fa-brain";
}

// ============================================================
// Kategorie-Funktionen
// ============================================================

/**
 * Gibt die Haupt-Kategorie-ID zurück (1-6).
 * Subcategory-IDs (z.B. 101, 205) werden auf ihre Haupt-ID gemappt.
 */
export function getMainCategoryId(id: number | undefined): number {
  if (!id) return 0;
  if (id <= 6) return id;
  return Math.floor(id / 100); // Subcategory IDs are 101, 102, 201, etc.
}

/**
 * Gibt die CSS-Variable für eine Kategorie-Farbe zurück.
 * @param id - Kategorie-ID (1-6 oder Subcategory wie 101, 205)
 * @param fallback - Fallback CSS-Variable wenn keine gültige Kategorie (default: 'var(--accent-primary)')
 */
export function getCategoryColorVar(
  id: number | undefined,
  fallback: string = "var(--accent-primary)",
): string {
  const mainId = getMainCategoryId(id);
  if (mainId >= 1 && mainId <= 6) {
    return `var(--category-${mainId})`;
  }
  return fallback;
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
  return text.slice(0, maxLength) + "...";
}

/**
 * Entfernt HTML-Tags aus einem String.
 * Verwendet DOMParser zur sicheren Text-Extraktion.
 */
export function stripHtml(html: string): string {
  if (typeof DOMParser === "undefined") {
    // SSR-safe: einfache Regex-Lösung
    return html.replace(/<[^>]*>/g, "");
  }
  const doc = new DOMParser().parseFromString(html, "text/html");
  return doc.body.textContent || "";
}
