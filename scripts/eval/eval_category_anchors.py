#!/usr/bin/env python3
"""Task #74: Evaluation Kategorie-Anker-Embeddings.

Frage: Können Artikel-Kategorien per Cosine-Similarity zwischen dem
vorhandenen Artikel-Embedding (snowflake-arctic-embed2) und vorberechneten
Kategorie-Anker-Embeddings bestimmt werden — statt per LLM?

Ground Truth: vorhandene LLM-Kategorisierung (fnord_sephiroth) der
488 Artikel mit Embeddings in src-tauri/data/fuckup.db.
Ollama: via SSH-Tunnel auf Ganymed (localhost:11435), identisches Modell.
"""

import json
import sqlite3
import struct
import urllib.request
from collections import Counter, defaultdict

DB = "/Users/hnsstrk/Repositories/fuckupRSS/src-tauri/data/fuckup.db"
OLLAMA = "http://localhost:11435/api/embed"
MODEL = "snowflake-arctic-embed2:latest"

# Anker-Beschreibungen (Variante b/c). Deutsch, da Summaries deutsch sind;
# arctic-embed2 ist multilingual.
CATEGORIES = {
    101: ("Technik", "Technologie, Software, Internet, Computer, KI, Gadgets und digitale Entwicklungen"),
    102: ("Wissenschaft", "Wissenschaftliche Forschung, Studien, Raumfahrt, Physik, Biologie und Entdeckungen"),
    201: ("Politik", "Politik, Regierungen, Wahlen, Parteien, Diplomatie und internationale Beziehungen"),
    202: ("Gesellschaft", "Gesellschaftliche Themen, Soziales, Bildung, Migration, Demografie und Alltagsleben"),
    203: ("Recht", "Justiz, Gerichte, Urteile, Gesetze, Kriminalität und Strafverfolgung"),
    301: ("Wirtschaft", "Wirtschaft, Unternehmen, Finanzen, Märkte, Handel, Inflation und Arbeitsmarkt"),
    302: ("Energie", "Energieversorgung, Strom, Gas, Öl, erneuerbare Energien und Energiepolitik"),
    401: ("Umwelt", "Umwelt, Klimawandel, Naturkatastrophen, Artenschutz und Nachhaltigkeit"),
    402: ("Gesundheit", "Gesundheit, Medizin, Krankheiten, Pandemien, Therapien und Gesundheitspolitik"),
    501: ("Sicherheit", "Innere Sicherheit, Polizei, Terrorismus, Geheimdienste und Cybersicherheit"),
    502: ("Verteidigung", "Militär, Krieg, Streitkräfte, Waffen, Rüstung und bewaffnete Konflikte"),
    601: ("Kultur", "Kultur, Kunst, Musik, Film, Literatur, Medien und Unterhaltung"),
    602: ("Sport", "Sport, Fußball, Olympia, Wettkämpfe, Vereine und Athleten"),
}


def embed(texts):
    req = urllib.request.Request(
        OLLAMA,
        data=json.dumps({"model": MODEL, "input": texts}).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.load(r)["embeddings"]


def blob_to_vec(blob):
    n = len(blob) // 4
    return struct.unpack(f"<{n}f", blob)


def cosine(a, b):
    dot = sum(x * y for x, y in zip(a, b))
    na = sum(x * x for x in a) ** 0.5
    nb = sum(x * x for x in b) ** 0.5
    return dot / (na * nb) if na and nb else 0.0


def evaluate(anchor_texts, articles, label):
    anchors = embed(anchor_texts)
    cat_ids = list(CATEGORIES.keys())
    top1 = top3 = any_match_top1 = 0
    per_cat = defaultdict(lambda: [0, 0])  # gt -> [correct, total]
    confusion = Counter()
    for art_vec, primary_gt, all_gt in articles:
        sims = sorted(
            ((cosine(art_vec, av), cid) for av, cid in zip(anchors, cat_ids)),
            reverse=True,
        )
        pred1 = sims[0][1]
        pred3 = {c for _, c in sims[:3]}
        if pred1 == primary_gt:
            top1 += 1
            per_cat[primary_gt][0] += 1
        else:
            confusion[(primary_gt, pred1)] += 1
        per_cat[primary_gt][1] += 1
        if primary_gt in pred3:
            top3 += 1
        if pred1 in all_gt:
            any_match_top1 += 1
    n = len(articles)
    print(f"\n=== {label} ===")
    print(f"Top-1 (primäre Kategorie):   {top1}/{n} = {top1/n:.1%}")
    print(f"Top-3 (primäre Kategorie):   {top3}/{n} = {top3/n:.1%}")
    print(f"Top-1 in irgendeiner GT-Kat: {any_match_top1}/{n} = {any_match_top1/n:.1%}")
    print("Pro Kategorie (Top-1):")
    for cid, (c, t) in sorted(per_cat.items()):
        print(f"  {CATEGORIES[cid][0]:<14} {c:>3}/{t:<3} = {c/t:.0%}")
    print("Häufigste Verwechslungen (GT -> Prediction):")
    for (gt, pred), cnt in confusion.most_common(5):
        print(f"  {CATEGORIES[gt][0]} -> {CATEGORIES[pred][0]}: {cnt}")
    return top1 / n, top3 / n, any_match_top1 / n


def main():
    conn = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
    rows = conn.execute(
        """SELECT f.id, f.embedding,
                  (SELECT fs.sephiroth_id FROM fnord_sephiroth fs
                   WHERE fs.fnord_id = f.id ORDER BY fs.confidence DESC LIMIT 1),
                  GROUP_CONCAT(fs2.sephiroth_id)
           FROM fnords f
           JOIN fnord_sephiroth fs2 ON fs2.fnord_id = f.id
           WHERE f.embedding IS NOT NULL
           GROUP BY f.id"""
    ).fetchall()

    articles = []
    for _fid, blob, primary, all_cats in rows:
        if primary not in CATEGORIES:
            continue  # 999 Unkategorisiert etc.
        all_gt = {int(c) for c in all_cats.split(",") if int(c) in CATEGORIES}
        articles.append((blob_to_vec(blob), primary, all_gt))

    print(f"Artikel in der Evaluation: {len(articles)}")
    print("GT-Verteilung:", dict(Counter(a[1] for a in articles).most_common()))

    names = [CATEGORIES[c][0] for c in CATEGORIES]
    descs = [f"{CATEGORIES[c][0]}: {CATEGORIES[c][1]}" for c in CATEGORIES]
    query_descs = [f"query: {d}" for d in descs]

    r_a = evaluate(names, articles, "Variante A: nur Kategoriename")
    r_b = evaluate(descs, articles, "Variante B: Name + Beschreibung")
    r_c = evaluate(query_descs, articles, "Variante C: 'query: ' + Name + Beschreibung")

    print("\n=== Zusammenfassung (Top-1 / Top-3 / Any-GT) ===")
    for label, r in [("A Name", r_a), ("B Name+Desc", r_b), ("C query-Präfix", r_c)]:
        print(f"  {label:<16} {r[0]:.1%} / {r[1]:.1%} / {r[2]:.1%}")


if __name__ == "__main__":
    main()
