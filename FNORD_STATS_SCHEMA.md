# FNORD_STATS_SCHEMA.md

## Datenfelder für Fnord-Statistiken

### Primäre Tabellen

#### fnords (Artikel)
| Feld | Typ | Statistik-Relevanz |
|------|-----|-------------------|
| `id` | INTEGER PK | Identifikation |
| `pentacle_id` | INTEGER FK | **Dimension: Quelle** |
| `status` | TEXT | **Dimension: Status** (concealed/illuminated/golden_apple) |
| `has_changes` | BOOLEAN | Filter: Nur Fnords |
| `revision_count` | INTEGER | **Metrik: Revisions pro Artikel** |
| `political_bias` | INTEGER | **Dimension: Bias** (-2 bis +2) |
| `sachlichkeit` | INTEGER | **Metrik: Qualität** (0-4) |
| `published_at` | DATETIME | Filter: Zeitraum |
| `changed_at` | DATETIME | Filter: Letzte Änderung |

#### fnord_revisions (Änderungshistorie)
| Feld | Typ | Statistik-Relevanz |
|------|-----|-------------------|
| `id` | INTEGER PK | Zählbar |
| `fnord_id` | INTEGER FK | Join zu Artikel |
| `revision_at` | DATETIME | **Dimension: Zeit** |

#### pentacles (Quellen)
| Feld | Typ | Statistik-Relevanz |
|------|-----|-------------------|
| `id` | INTEGER PK | Identifikation |
| `title` | TEXT | Anzeigename |
| `article_count` | INTEGER | Gesamtartikel (denormalisiert) |

#### sephiroth (Kategorien)
| Feld | Typ | Statistik-Relevanz |
|------|-----|-------------------|
| `id` | INTEGER PK | Identifikation |
| `name` | TEXT | Anzeigename |
| `parent_id` | INTEGER FK | Hierarchie |
| `level` | INTEGER | 0=Haupt, 1=Unter |
| `icon` | TEXT | UI Icon |
| `color` | TEXT | UI Farbe |

#### fnord_sephiroth (Artikel ↔ Kategorie)
| Feld | Typ | Statistik-Relevanz |
|------|-----|-------------------|
| `fnord_id` | INTEGER FK | Join |
| `sephiroth_id` | INTEGER FK | **Dimension: Kategorie** |

---

## Aggregationslogik

### 1. Nach Quelle (Gesamt) - WIEDERHERZUSTELLEN

**SQL:**
```sql
SELECT
    p.id AS pentacle_id,
    p.title,
    COUNT(DISTINCT r.id) AS revision_count,
    COUNT(DISTINCT r.fnord_id) AS article_count
FROM pentacles p
LEFT JOIN fnords f ON f.pentacle_id = p.id
LEFT JOIN fnord_revisions r ON r.fnord_id = f.id
GROUP BY p.id
ORDER BY revision_count DESC
```

**Ergebnis-Typ:**
```rust
pub struct SourceRevisionStats {
    pub pentacle_id: i64,
    pub title: String,
    pub revision_count: i64,
    pub article_count: i64,
}
```

**Existiert bereits:** `get_fnord_stats()` in `fnords.rs:534-567`

### 2. Nach Kategorie (Haupt)

**SQL:**
```sql
SELECT
    m.id AS sephiroth_id,
    m.name,
    m.icon,
    m.color,
    COUNT(DISTINCT r.id) AS revision_count,
    COUNT(DISTINCT r.fnord_id) AS article_count
FROM sephiroth m
LEFT JOIN sephiroth s ON s.parent_id = m.id
LEFT JOIN fnord_sephiroth fs ON fs.sephiroth_id = s.id
LEFT JOIN fnord_revisions r ON r.fnord_id = fs.fnord_id
WHERE m.level = 0
GROUP BY m.id
ORDER BY revision_count DESC
```

**Existiert bereits:** `get_fnord_stats()` liefert `by_category`

### 3. Status-Übersicht

**SQL:**
```sql
SELECT
    status,
    COUNT(*) AS count
FROM fnords
GROUP BY status
```

**Existiert:** Implizit in Sidebar über `pentacles` Query

### 4. Greyface Index

**SQL (Bias-Verteilung):**
```sql
SELECT
    political_bias,
    COUNT(*) AS count
FROM fnords
WHERE political_bias IS NOT NULL
GROUP BY political_bias
```

**Existiert bereits:** `get_greyface_index()` in `fnords.rs`

---

## Backend-Struktur (bereits vorhanden)

### FnordStats (Haupt-Response)
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct FnordStats {
    pub total_revisions: i64,
    pub articles_with_changes: i64,
    pub by_category: Vec<CategoryRevisionStats>,
    pub by_source: Vec<SourceRevisionStats>,  // <-- NICHT GENUTZT IM FRONTEND
}
```

### CategoryRevisionStats
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryRevisionStats {
    pub sephiroth_id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub revision_count: i64,
    pub article_count: i64,
}
```

### SourceRevisionStats
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SourceRevisionStats {
    pub pentacle_id: i64,
    pub title: String,
    pub revision_count: i64,
    pub article_count: i64,
}
```

---

## Frontend-Types (state.svelte.ts)

### Existierend
```typescript
export interface FnordStats {
  total_revisions: number;
  articles_with_changes: number;
  by_category: CategoryRevisionStats[];
  by_source: SourceRevisionStats[];  // <-- VORHANDEN ABER UNGENUTZT
}

export interface CategoryRevisionStats {
  sephiroth_id: number;
  name: string;
  icon?: string;
  color?: string;
  revision_count: number;
  article_count: number;
}

export interface SourceRevisionStats {
  pentacle_id: number;
  title: string;
  revision_count: number;
  article_count: number;
}
```

---

## Fazit

**Keine Backend-Änderungen nötig!**

Das Backend liefert bereits alle benötigten Daten:
- `get_fnord_stats()` gibt `by_source` zurück
- Types sind definiert und exportiert
- SQL-Queries sind optimiert

**Nur Frontend-Änderung erforderlich:**
- Render-Code für `stats.by_source` in `FnordView.svelte` hinzufügen
- Ähnlich zu `stats.by_category` Sektion
- Mit Fortschrittsbalken wie vor dem 16. Januar
