#!/usr/bin/env python3
"""Qualitative Gegenüberstellung: bestehende Kategorie vs. Anker-Top1."""
import json, sqlite3, struct, urllib.request, random

DB = "/Users/hnsstrk/Repositories/fuckupRSS/src-tauri/data/fuckup.db"
OLLAMA = "http://localhost:11435/api/embed"
MODEL = "snowflake-arctic-embed2:latest"
CATEGORIES = {
    101: "Technik", 102: "Wissenschaft", 201: "Politik", 202: "Gesellschaft",
    203: "Recht", 301: "Wirtschaft", 302: "Energie", 401: "Umwelt",
    402: "Gesundheit", 501: "Sicherheit", 502: "Verteidigung",
    601: "Kultur", 602: "Sport",
}
DESCS = {
    101: "Technik: Technologie, Software, Internet, Computer, KI, Gadgets und digitale Entwicklungen",
    102: "Wissenschaft: Wissenschaftliche Forschung, Studien, Raumfahrt, Physik, Biologie und Entdeckungen",
    201: "Politik: Politik, Regierungen, Wahlen, Parteien, Diplomatie und internationale Beziehungen",
    202: "Gesellschaft: Gesellschaftliche Themen, Soziales, Bildung, Migration, Demografie und Alltagsleben",
    203: "Recht: Justiz, Gerichte, Urteile, Gesetze, Kriminalität und Strafverfolgung",
    301: "Wirtschaft: Wirtschaft, Unternehmen, Finanzen, Märkte, Handel, Inflation und Arbeitsmarkt",
    302: "Energie: Energieversorgung, Strom, Gas, Öl, erneuerbare Energien und Energiepolitik",
    401: "Umwelt: Umwelt, Klimawandel, Naturkatastrophen, Artenschutz und Nachhaltigkeit",
    402: "Gesundheit: Gesundheit, Medizin, Krankheiten, Pandemien, Therapien und Gesundheitspolitik",
    501: "Sicherheit: Innere Sicherheit, Polizei, Terrorismus, Geheimdienste und Cybersicherheit",
    502: "Verteidigung: Militär, Krieg, Streitkräfte, Waffen, Rüstung und bewaffnete Konflikte",
    601: "Kultur: Kultur, Kunst, Musik, Film, Literatur, Medien und Unterhaltung",
    602: "Sport: Sport, Fußball, Olympia, Wettkämpfe, Vereine und Athleten",
}

def embed(texts):
    req = urllib.request.Request(OLLAMA, data=json.dumps({"model": MODEL, "input": texts}).encode(),
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:
        return json.load(r)["embeddings"]

def cosine(a, b):
    dot = sum(x*y for x, y in zip(a, b))
    na = sum(x*x for x in a) ** 0.5
    nb = sum(x*x for x in b) ** 0.5
    return dot/(na*nb) if na and nb else 0.0

conn = sqlite3.connect(f"file:{DB}?mode=ro", uri=True)
rows = conn.execute("""
    SELECT f.id, f.title, f.embedding,
           (SELECT fs.sephiroth_id FROM fnord_sephiroth fs
            WHERE fs.fnord_id=f.id ORDER BY (fs.source = 'ai') DESC, fs.confidence DESC LIMIT 1)
    FROM fnords f WHERE f.embedding IS NOT NULL""").fetchall()
rows = [r for r in rows if r[3] in CATEGORIES]
random.seed(42)
sample = random.sample(rows, 30)

anchors = embed(list(DESCS.values()))
cat_ids = list(DESCS.keys())

print(f"{'Titel':<62} | {'DB-Kat':<13} | Anker-Top1 (Score)")
print("-" * 110)
for _fid, title, blob, gt in sample:
    n = len(blob)//4
    vec = struct.unpack(f"<{n}f", blob)
    sims = sorted(((cosine(vec, av), cid) for av, cid in zip(anchors, cat_ids)), reverse=True)
    s, pred = sims[0]
    mark = "=" if pred == gt else " "
    print(f"{(title or '')[:60]:<62} | {CATEGORIES[gt]:<13} |{mark}{CATEGORIES[pred]} ({s:.2f})")
