// Law of Fives - Discordianisches Dashboard-Modul
//
// Das "Gesetz der Fuenf" ist ein zentrales Element des Discordianismus:
// "Alle Dinge geschehen in Fuenfern, oder sind durch Fuenf teilbar, oder
// sind irgendwie direkt oder indirekt mit 5 verbunden."
//
// Dieses Modul liefert Statistiken in Fuenfer-Gruppen fuer das Dashboard.

use crate::AppState;
use serde::Serialize;
use tauri::State;

/// Keyword mit Trend-Information
#[derive(Debug, Serialize, Clone)]
pub struct TopKeyword {
    pub id: i64,
    pub name: String,
    pub keyword_type: Option<String>,
    pub article_count: i64,
    pub trend_direction: TrendDirection,
    pub trend_percent: f64,
}

/// Trend-Richtung
#[derive(Debug, Serialize, Clone)]
pub enum TrendDirection {
    Rising,
    Stable,
    Falling,
}

/// Top Feed mit Aktivitaets-Info
#[derive(Debug, Serialize, Clone)]
pub struct TopFeed {
    pub id: i64,
    pub title: String,
    pub article_count: i64,
    pub unread_count: i64,
    pub articles_today: i64,
    pub articles_week: i64,
}

/// Top Kategorie
#[derive(Debug, Serialize, Clone)]
pub struct TopCategory {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub article_count: i64,
    pub trend_direction: TrendDirection,
}

/// 5-Tage-Trend Datenpunkt
#[derive(Debug, Serialize, Clone)]
pub struct TrendPoint {
    pub date: String,
    pub article_count: i64,
    pub keyword_count: i64,
}

/// Fnord-Index (5-stufige Bewertung)
#[derive(Debug, Serialize, Clone)]
pub struct FnordIndex {
    /// Stufe 1-5 (1=niedrig, 5=hoch)
    pub level: u8,
    /// Beschreibung der Stufe
    pub description: String,
    /// Komponenten des Index
    pub components: FnordIndexComponents,
}

/// Komponenten des Fnord-Index
#[derive(Debug, Serialize, Clone)]
pub struct FnordIndexComponents {
    /// Anteil geaenderter Artikel (0-1)
    pub change_rate: f64,
    /// Durchschnittliche Bias-Staerke (0-1)
    pub bias_intensity: f64,
    /// Artikel-Aktivitaet (0-1)
    pub activity_rate: f64,
    /// Keyword-Diversitaet (0-1)
    pub keyword_diversity: f64,
    /// Leseabdeckung (0-1)
    pub reading_coverage: f64,
}

/// Komplett-Response fuer Law of Fives Dashboard
#[derive(Debug, Serialize)]
pub struct LawOfFivesStats {
    pub top_5_keywords: Vec<TopKeyword>,
    pub top_5_feeds: Vec<TopFeed>,
    pub top_5_categories: Vec<TopCategory>,
    pub five_day_trend: Vec<TrendPoint>,
    pub fnord_index: FnordIndex,
    /// Zeitstempel der Berechnung
    pub calculated_at: String,
}

/// Haupt-Command: Law of Fives Statistiken
#[tauri::command]
pub fn get_law_of_fives_stats(state: State<AppState>) -> Result<LawOfFivesStats, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let conn = db.conn();

    // Top 5 Keywords mit Trend
    let top_5_keywords = get_top_keywords(conn)?;

    // Top 5 Feeds
    let top_5_feeds = get_top_feeds(conn)?;

    // Top 5 Kategorien
    let top_5_categories = get_top_categories(conn)?;

    // 5-Tage-Trend
    let five_day_trend = get_five_day_trend(conn)?;

    // Fnord-Index
    let fnord_index = calculate_fnord_index(conn)?;

    Ok(LawOfFivesStats {
        top_5_keywords,
        top_5_feeds,
        top_5_categories,
        five_day_trend,
        fnord_index,
        calculated_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn get_top_keywords(conn: &rusqlite::Connection) -> Result<Vec<TopKeyword>, String> {
    // Top 5 Keywords nach Artikelanzahl mit Trend-Berechnung
    // Trend: Vergleich letzte 7 Tage vs. vorherige 7 Tage
    let mut stmt = conn
        .prepare(
            r#"
            WITH keyword_recent AS (
                SELECT fi.keyword_id, COUNT(*) as recent_count
                FROM fnord_immanentize fi
                JOIN fnords f ON f.id = fi.fnord_id
                WHERE f.published_at >= date('now', '-7 days')
                GROUP BY fi.keyword_id
            ),
            keyword_previous AS (
                SELECT fi.keyword_id, COUNT(*) as previous_count
                FROM fnord_immanentize fi
                JOIN fnords f ON f.id = fi.fnord_id
                WHERE f.published_at >= date('now', '-14 days')
                  AND f.published_at < date('now', '-7 days')
                GROUP BY fi.keyword_id
            )
            SELECT
                i.id,
                i.name,
                i.keyword_type,
                COUNT(fi.fnord_id) as article_count,
                COALESCE(kr.recent_count, 0) as recent,
                COALESCE(kp.previous_count, 0) as previous
            FROM immanentize i
            JOIN fnord_immanentize fi ON fi.keyword_id = i.id
            LEFT JOIN keyword_recent kr ON kr.keyword_id = i.id
            LEFT JOIN keyword_previous kp ON kp.keyword_id = i.id
            GROUP BY i.id
            ORDER BY article_count DESC
            LIMIT 5
            "#,
        )
        .map_err(|e| e.to_string())?;

    let keywords = stmt
        .query_map([], |row| {
            let recent: i64 = row.get(4)?;
            let previous: i64 = row.get(5)?;

            let (trend_direction, trend_percent) = calculate_trend(recent, previous);

            Ok(TopKeyword {
                id: row.get(0)?,
                name: row.get(1)?,
                keyword_type: row.get(2)?,
                article_count: row.get(3)?,
                trend_direction,
                trend_percent,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(keywords)
}

fn get_top_feeds(conn: &rusqlite::Connection) -> Result<Vec<TopFeed>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                p.id,
                p.title,
                COUNT(f.id) as article_count,
                SUM(CASE WHEN f.status = 'concealed' THEN 1 ELSE 0 END) as unread_count,
                SUM(CASE WHEN date(f.published_at) = date('now') THEN 1 ELSE 0 END) as articles_today,
                SUM(CASE WHEN f.published_at >= date('now', '-7 days') THEN 1 ELSE 0 END) as articles_week
            FROM pentacles p
            LEFT JOIN fnords f ON f.pentacle_id = p.id
            GROUP BY p.id
            ORDER BY articles_week DESC, article_count DESC
            LIMIT 5
            "#,
        )
        .map_err(|e| e.to_string())?;

    let feeds = stmt
        .query_map([], |row| {
            Ok(TopFeed {
                id: row.get(0)?,
                title: row.get::<_, Option<String>>(1)?.unwrap_or_else(|| "Unknown".to_string()),
                article_count: row.get(2)?,
                unread_count: row.get(3)?,
                articles_today: row.get(4)?,
                articles_week: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(feeds)
}

fn get_top_categories(conn: &rusqlite::Connection) -> Result<Vec<TopCategory>, String> {
    // Top 5 Hauptkategorien nach Artikelanzahl (level=0)
    let mut stmt = conn
        .prepare(
            r#"
            WITH category_recent AS (
                SELECT s.parent_id, COUNT(*) as recent_count
                FROM fnord_sephiroth fs
                JOIN sephiroth s ON s.id = fs.sephiroth_id
                JOIN fnords f ON f.id = fs.fnord_id
                WHERE s.level = 1
                  AND f.published_at >= date('now', '-7 days')
                GROUP BY s.parent_id
            ),
            category_previous AS (
                SELECT s.parent_id, COUNT(*) as previous_count
                FROM fnord_sephiroth fs
                JOIN sephiroth s ON s.id = fs.sephiroth_id
                JOIN fnords f ON f.id = fs.fnord_id
                WHERE s.level = 1
                  AND f.published_at >= date('now', '-14 days')
                  AND f.published_at < date('now', '-7 days')
                GROUP BY s.parent_id
            )
            SELECT
                m.id,
                m.name,
                m.icon,
                m.color,
                COUNT(DISTINCT fs.fnord_id) as article_count,
                COALESCE(cr.recent_count, 0) as recent,
                COALESCE(cp.previous_count, 0) as previous
            FROM sephiroth m
            LEFT JOIN sephiroth s ON s.parent_id = m.id AND s.level = 1
            LEFT JOIN fnord_sephiroth fs ON fs.sephiroth_id = s.id
            LEFT JOIN category_recent cr ON cr.parent_id = m.id
            LEFT JOIN category_previous cp ON cp.parent_id = m.id
            WHERE m.level = 0
            GROUP BY m.id
            ORDER BY article_count DESC
            LIMIT 5
            "#,
        )
        .map_err(|e| e.to_string())?;

    let categories = stmt
        .query_map([], |row| {
            let recent: i64 = row.get(5)?;
            let previous: i64 = row.get(6)?;

            let (trend_direction, _) = calculate_trend(recent, previous);

            Ok(TopCategory {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                color: row.get(3)?,
                article_count: row.get(4)?,
                trend_direction,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(categories)
}

fn get_five_day_trend(conn: &rusqlite::Connection) -> Result<Vec<TrendPoint>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            WITH RECURSIVE dates(date) AS (
                SELECT date('now', '-4 days')
                UNION ALL
                SELECT date(date, '+1 day')
                FROM dates
                WHERE date < date('now')
            )
            SELECT
                d.date,
                COALESCE(article_counts.count, 0) as article_count,
                COALESCE(keyword_counts.count, 0) as keyword_count
            FROM dates d
            LEFT JOIN (
                SELECT date(published_at) as pub_date, COUNT(*) as count
                FROM fnords
                WHERE published_at >= date('now', '-4 days')
                GROUP BY pub_date
            ) article_counts ON article_counts.pub_date = d.date
            LEFT JOIN (
                SELECT date(id.created_at) as create_date, COUNT(*) as count
                FROM immanentize_daily id
                WHERE id.date >= date('now', '-4 days')
                GROUP BY create_date
            ) keyword_counts ON keyword_counts.create_date = d.date
            ORDER BY d.date ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let trend = stmt
        .query_map([], |row| {
            Ok(TrendPoint {
                date: row.get(0)?,
                article_count: row.get(1)?,
                keyword_count: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(trend)
}

fn calculate_fnord_index(conn: &rusqlite::Connection) -> Result<FnordIndex, String> {
    // Komponenten sammeln

    // 1. Change Rate: Anteil Artikel mit Revisionen
    let (total_articles, changed_articles): (i64, i64) = conn
        .query_row(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN has_changes = 1 OR revision_count > 0 THEN 1 ELSE 0 END) as changed
            FROM fnords
            "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or((0, 0));

    let change_rate = if total_articles > 0 {
        changed_articles as f64 / total_articles as f64
    } else {
        0.0
    };

    // 2. Bias Intensity: Durchschnittliche Abweichung vom Neutral
    let bias_intensity: f64 = conn
        .query_row(
            r#"
            SELECT COALESCE(AVG(ABS(COALESCE(political_bias, 0))), 0) / 2.0
            FROM fnords
            WHERE political_bias IS NOT NULL
            "#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // 3. Activity Rate: Artikel der letzten 7 Tage vs. Durchschnitt
    let (recent_articles, avg_weekly): (i64, f64) = conn
        .query_row(
            r#"
            SELECT
                (SELECT COUNT(*) FROM fnords WHERE published_at >= date('now', '-7 days')),
                COALESCE(
                    (SELECT COUNT(*) * 7.0 / MAX(1, julianday('now') - julianday(MIN(published_at)))
                     FROM fnords
                     WHERE published_at IS NOT NULL),
                    0
                )
            "#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap_or((0, 0.0));

    let activity_rate = if avg_weekly > 0.0 {
        (recent_articles as f64 / avg_weekly).min(1.0)
    } else {
        0.0
    };

    // 4. Keyword Diversity: Eindeutige Keywords pro Artikel
    let keyword_diversity: f64 = conn
        .query_row(
            r#"
            SELECT COALESCE(
                CAST(COUNT(DISTINCT keyword_id) AS REAL) / NULLIF(COUNT(DISTINCT fnord_id), 0),
                0
            ) / 10.0
            FROM fnord_immanentize
            "#,
            [],
            |row| row.get(0),
        )
        .map(|v: f64| v.min(1.0))
        .unwrap_or(0.0);

    // 5. Reading Coverage: Anteil gelesener Artikel
    let reading_coverage: f64 = conn
        .query_row(
            r#"
            SELECT COALESCE(
                CAST(SUM(CASE WHEN status IN ('illuminated', 'golden_apple') THEN 1 ELSE 0 END) AS REAL)
                / NULLIF(COUNT(*), 0),
                0
            )
            FROM fnords
            "#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0.0);

    // Gesamtscore berechnen (gewichteter Durchschnitt)
    let score = (change_rate * 0.2
        + bias_intensity * 0.2
        + activity_rate * 0.2
        + keyword_diversity * 0.2
        + reading_coverage * 0.2)
        .min(1.0);

    // In 5er-Stufe umwandeln
    let level = match score {
        s if s < 0.2 => 1,
        s if s < 0.4 => 2,
        s if s < 0.6 => 3,
        s if s < 0.8 => 4,
        _ => 5,
    };

    let description = match level {
        1 => "Novize - Beginne die Fnords zu sehen".to_string(),
        2 => "Initiat - Die Muster werden sichtbar".to_string(),
        3 => "Adept - Chaos und Ordnung im Gleichgewicht".to_string(),
        4 => "Erleuchteter - Die goldenen Aepfel nahen".to_string(),
        5 => "Papst/Paepstin - Hail Eris!".to_string(),
        _ => "Unbekannt".to_string(),
    };

    Ok(FnordIndex {
        level,
        description,
        components: FnordIndexComponents {
            change_rate,
            bias_intensity,
            activity_rate,
            keyword_diversity,
            reading_coverage,
        },
    })
}

fn calculate_trend(recent: i64, previous: i64) -> (TrendDirection, f64) {
    if previous == 0 {
        if recent > 0 {
            (TrendDirection::Rising, 100.0)
        } else {
            (TrendDirection::Stable, 0.0)
        }
    } else {
        let change = ((recent - previous) as f64 / previous as f64) * 100.0;
        let direction = if change > 10.0 {
            TrendDirection::Rising
        } else if change < -10.0 {
            TrendDirection::Falling
        } else {
            TrendDirection::Stable
        };
        (direction, change.abs())
    }
}
