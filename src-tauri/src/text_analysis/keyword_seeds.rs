//! Keyword seed data for pre-populating the database with known entities.
//!
//! This module provides lists of known persons, organizations, locations, and acronyms
//! that can be used to:
//! 1. Pre-populate the immanentize table with correct keyword_type
//! 2. Improve keyword extraction by recognizing known entities
//! 3. Validate and correct keyword types

use rusqlite::{params, Connection};
use log::info;

/// Known persons (politicians, business leaders, celebrities)
pub static KNOWN_PERSONS: &[&str] = &[
    // German Politicians
    "Olaf Scholz", "Friedrich Merz", "Robert Habeck", "Christian Lindner",
    "Annalena Baerbock", "Markus Söder", "Armin Laschet", "Saskia Esken",
    "Lars Klingbeil", "Alice Weidel", "Tino Chrupalla", "Sahra Wagenknecht",
    "Dietmar Bartsch", "Ricarda Lang", "Omid Nouripour", "Wolfgang Kubicki",
    "Karl Lauterbach", "Boris Pistorius", "Nancy Faeser", "Volker Wissing",
    "Cem Özdemir", "Steffi Lemke", "Marco Buschmann", "Hubertus Heil",
    "Klara Geywitz", "Bettina Stark-Watzinger", "Lisa Paus", "Svenja Schulze",
    // Former German Politicians
    "Angela Merkel", "Gerhard Schröder", "Helmut Kohl", "Helmut Schmidt",
    "Willy Brandt", "Konrad Adenauer", "Frank-Walter Steinmeier",
    "Joachim Gauck", "Horst Köhler", "Christian Wulff", "Johannes Rau",
    "Wolfgang Schäuble", "Heiko Maas", "Sigmar Gabriel", "Martin Schulz",
    "Andrea Nahles", "Ursula von der Leyen", "Thomas de Maizière",
    // US Politicians
    "Donald Trump", "Joe Biden", "Kamala Harris", "Barack Obama",
    "Hillary Clinton", "Bill Clinton", "George Bush", "Nancy Pelosi",
    "Mitch McConnell", "Chuck Schumer", "Alexandria Ocasio-Cortez",
    "Bernie Sanders", "Elizabeth Warren", "Ron DeSantis", "Nikki Haley",
    "Mike Pence", "Mike Pompeo", "Antony Blinken", "Janet Yellen",
    "Jerome Powell", "Lloyd Austin", "Jake Sullivan",
    // European Politicians
    "Emmanuel Macron", "Marine Le Pen", "Keir Starmer", "Rishi Sunak",
    "Boris Johnson", "Giorgia Meloni", "Mario Draghi", "Viktor Orbán",
    "Andrzej Duda", "Pedro Sánchez", "Mark Rutte", "Alexander De Croo",
    "Mette Frederiksen", "Ulf Kristersson", "Ursula von der Leyen",
    "Charles Michel", "Josep Borrell", "Christine Lagarde",
    // World Leaders
    "Xi Jinping", "Vladimir Putin", "Wolodymyr Selenskyj", "Narendra Modi",
    "Recep Tayyip Erdogan", "Mohammed bin Salman", "Benjamin Netanjahu",
    "Abdel Fattah al-Sisi", "Cyril Ramaphosa", "Luiz Inácio Lula da Silva",
    "Javier Milei", "Justin Trudeau", "Anthony Albanese",
    // Business Leaders
    "Elon Musk", "Jeff Bezos", "Mark Zuckerberg", "Tim Cook", "Satya Nadella",
    "Sundar Pichai", "Sam Altman", "Jensen Huang", "Warren Buffett",
    "Larry Fink", "Jamie Dimon", "Christine Lagarde", "Christian Sewing",
    "Oliver Bäte", "Herbert Diess", "Ola Källenius", "Oliver Blume",
    "Martin Winterkorn", "Joe Kaeser", "Roland Busch",
    // Other Notable Persons
    "Papst Franziskus", "Dalai Lama", "Greta Thunberg", "Julian Assange",
    "Edward Snowden", "Alexei Nawalny",
];

/// Known organizations (companies, institutions, NGOs)
pub static KNOWN_ORGANIZATIONS: &[&str] = &[
    // German Companies
    "Deutsche Bank", "Commerzbank", "Allianz", "Munich Re", "Siemens",
    "Volkswagen", "BMW", "Mercedes-Benz", "Daimler", "Porsche", "Audi",
    "BASF", "Bayer", "Henkel", "SAP", "Deutsche Telekom", "Deutsche Post",
    "Lufthansa", "Deutsche Bahn", "RWE", "E.ON", "Thyssenkrupp",
    "Bosch", "Continental", "Infineon", "Fresenius", "Merck", "Adidas",
    "Puma", "Zalando", "Delivery Hero", "HelloFresh", "Axel Springer",
    "Bertelsmann", "ProSiebenSat.1",
    // US Tech Companies
    "Google", "Apple", "Microsoft", "Amazon", "Meta", "Tesla", "Nvidia",
    "Netflix", "Alphabet", "Intel", "AMD", "IBM", "Oracle", "Salesforce",
    "Adobe", "Cisco", "Qualcomm", "Broadcom", "Texas Instruments",
    "OpenAI", "Anthropic", "SpaceX", "Twitter", "X Corp",
    // Other International Companies
    "Samsung", "Sony", "Toyota", "Honda", "Huawei", "Alibaba", "Tencent",
    "TSMC", "Shell", "BP", "TotalEnergies", "Nestlé", "Novartis", "Roche",
    "LVMH", "Unilever", "HSBC", "Barclays", "UBS", "Credit Suisse",
    // German Government/Institutions
    "Bundesregierung", "Bundestag", "Bundesrat", "Bundeskanzleramt",
    "Auswärtiges Amt", "Bundesinnenministerium", "Bundesfinanzministerium",
    "Bundesverteidigungsministerium", "Bundesgesundheitsministerium",
    "Bundeswirtschaftsministerium", "Bundesjustizministerium",
    "Bundesumweltministerium", "Bundesarbeitsministerium",
    "Bundesverfassungsgericht", "Bundesgerichtshof", "Bundesbank",
    "Bundesnachrichtendienst", "Bundesamt für Verfassungsschutz",
    "Bundeskriminalamt", "Bundespolizei", "Bundeswehr",
    "Robert Koch-Institut", "Paul-Ehrlich-Institut", "Stiko",
    // EU Institutions
    "Europäische Kommission", "Europäisches Parlament", "Europäischer Rat",
    "Europäische Zentralbank", "Europäischer Gerichtshof",
    "European Commission", "European Parliament", "European Council",
    "European Central Bank", "European Court of Justice",
    // International Organizations
    "Vereinte Nationen", "United Nations", "Weltbank", "World Bank",
    "Internationaler Währungsfonds", "International Monetary Fund", "IMF",
    "Weltgesundheitsorganisation", "World Health Organization", "WHO",
    "Welthandelsorganisation", "World Trade Organization", "WTO",
    "Internationaler Strafgerichtshof", "International Criminal Court", "ICC",
    "Amnesty International", "Greenpeace", "Ärzte ohne Grenzen",
    "Médecins Sans Frontières", "UNHCR", "UNICEF", "UNESCO",
    // German Parties
    "CDU", "CSU", "SPD", "Bündnis 90/Die Grünen", "Die Grünen", "FDP",
    "AfD", "Die Linke", "BSW", "Bündnis Sahra Wagenknecht",
    // Media Organizations
    "ARD", "ZDF", "Deutschlandfunk", "Deutsche Welle", "RTL", "ProSieben",
    "Sat.1", "Spiegel", "Der Spiegel", "BILD", "FAZ", "Frankfurter Allgemeine",
    "Süddeutsche Zeitung", "Die Zeit", "Die Welt", "Handelsblatt",
    "Wirtschaftswoche", "Focus", "Stern", "taz", "dpa",
    "Reuters", "Associated Press", "AFP", "BBC", "CNN", "Fox News",
    "New York Times", "Washington Post", "The Guardian", "Financial Times",
];

/// Known locations (countries, cities, regions)
pub static KNOWN_LOCATIONS: &[&str] = &[
    // German Cities
    "Berlin", "Hamburg", "München", "Köln", "Frankfurt", "Stuttgart",
    "Düsseldorf", "Leipzig", "Dortmund", "Essen", "Bremen", "Dresden",
    "Hannover", "Nürnberg", "Duisburg", "Bochum", "Wuppertal", "Bielefeld",
    "Bonn", "Münster", "Mannheim", "Karlsruhe", "Augsburg", "Wiesbaden",
    "Mönchengladbach", "Gelsenkirchen", "Aachen", "Braunschweig", "Kiel",
    "Chemnitz", "Magdeburg", "Freiburg", "Lübeck", "Erfurt", "Rostock",
    "Mainz", "Kassel", "Halle", "Saarbrücken", "Potsdam",
    // German States
    "Bayern", "Baden-Württemberg", "Nordrhein-Westfalen", "Niedersachsen",
    "Hessen", "Sachsen", "Rheinland-Pfalz", "Berlin", "Schleswig-Holstein",
    "Brandenburg", "Sachsen-Anhalt", "Thüringen", "Hamburg", "Mecklenburg-Vorpommern",
    "Saarland", "Bremen",
    // European Countries
    "Deutschland", "Frankreich", "Großbritannien", "Italien", "Spanien",
    "Polen", "Niederlande", "Belgien", "Österreich", "Schweiz", "Schweden",
    "Norwegen", "Dänemark", "Finnland", "Portugal", "Griechenland",
    "Tschechien", "Rumänien", "Ungarn", "Irland", "Ukraine", "Russland",
    // European Cities
    "Paris", "London", "Rom", "Madrid", "Barcelona", "Amsterdam", "Brüssel",
    "Wien", "Zürich", "Genf", "Bern", "Stockholm", "Oslo", "Kopenhagen",
    "Helsinki", "Warschau", "Prag", "Budapest", "Athen", "Lissabon",
    "Dublin", "Edinburgh", "Kiew", "Moskau", "Sankt Petersburg",
    // World Regions/Countries
    "USA", "Vereinigte Staaten", "United States", "China", "Japan", "Indien",
    "Brasilien", "Kanada", "Australien", "Mexiko", "Südkorea", "Indonesien",
    "Türkei", "Saudi-Arabien", "Iran", "Israel", "Ägypten", "Südafrika",
    "Nigeria", "Argentinien", "Taiwan", "Thailand", "Vietnam", "Philippinen",
    "Pakistan", "Bangladesch", "Afghanistan", "Irak", "Syrien", "Libanon",
    "Jordanien", "Katar", "Vereinigte Arabische Emirate", "Kuwait",
    // World Cities
    "New York", "Los Angeles", "Chicago", "Washington", "San Francisco",
    "Boston", "Miami", "Seattle", "Denver", "Houston", "Dallas", "Atlanta",
    "Peking", "Shanghai", "Hongkong", "Tokio", "Seoul", "Singapur",
    "Mumbai", "Delhi", "Tel Aviv", "Jerusalem", "Dubai", "Kairo",
    "Kapstadt", "Lagos", "Nairobi", "São Paulo", "Buenos Aires",
    "Mexico City", "Toronto", "Sydney", "Melbourne",
    // Regions
    "Europa", "Asien", "Afrika", "Nordamerika", "Südamerika", "Australien",
    "Naher Osten", "Mittlerer Osten", "Osteuropa", "Westeuropa", "Balkan",
    "Skandinavien", "Baltikum", "Kaukasus", "Zentralasien", "Südostasien",
    "Gaza", "Gazastreifen", "Westjordanland", "Krim", "Donbas", "Taiwan-Straße",
];

/// Known acronyms (organizations, technical terms)
pub static KNOWN_ACRONYMS: &[&str] = &[
    // Political/Government
    "EU", "UN", "NATO", "USA", "UK", "BRD", "DDR", "UNO", "G7", "G20",
    "OECD", "OPEC", "ASEAN", "AU", "BRICS", "OSZE", "GUS", "EFTA", "EWR",
    // German Agencies
    "BKA", "BND", "LKA", "BGH", "BVG", "EZB", "KfW", "GIZ", "THW", "DRK",
    "TÜV", "ADAC", "DIHK", "BDI", "DGB", "IG", "GEW", "Verdi", "BAMF",
    "BSI", "BfV", "MAD", "BaFin", "UBA", "DWD", "BfS", "BAFA", "PTB",
    // German Parties
    "CDU", "CSU", "SPD", "FDP", "AfD", "BSW",
    // International Organizations
    "WHO", "WTO", "IMF", "IWF", "UNESCO", "UNICEF", "UNHCR", "ICC", "ICJ",
    "IAEA", "OPCW", "ICRC", "FIFA", "UEFA", "IOC", "COP", "IPCC", "FAO",
    "ILO", "WIPO", "ITU", "UNEP", "WFP", "IFAD", "UNIDO", "CTBTO",
    // Tech/Business Acronyms
    "AI", "KI", "IT", "IoT", "API", "CEO", "CFO", "CTO", "IPO", "M&A",
    "B2B", "B2C", "SaaS", "IaaS", "PaaS", "VPN", "DNS", "HTTP", "HTTPS", "SSL", "TLS",
    "GPU", "CPU", "RAM", "SSD", "USB", "WLAN", "LTE", "5G", "6G",
    "DSGVO", "GDPR", "NIS2", "AGI", "LLM", "NLP", "ML", "DL", "RL",
    "USB-C", "HDMI", "PCIe", "NVMe", "SATA", "RAID",
    "REST", "GraphQL", "gRPC", "OAuth", "JWT", "CORS",
    "AWS", "GCP", "CI", "CD", "DevOps", "MLOps", "GitOps",
    "SQL", "NoSQL", "ORM", "CRUD", "MVC", "MVVM", "DDD",
    "TCP", "UDP", "IP", "FTP", "SSH", "SMTP", "IMAP", "POP3",
    "JSON", "XML", "YAML", "CSV", "HTML", "CSS", "SVG", "PNG", "JPEG",
    // Science/Research
    "CERN", "ESA", "NASA", "DLR", "MPG", "DFG", "DAAD", "HGF", "WGL", "FhG",
    "MIT", "ETH", "EPFL", "RWTH", "TUM", "KIT", "LMU", "FU", "HU", "TU",
    // Media
    "ARD", "ZDF", "RTL", "WDR", "NDR", "SWR", "BR", "HR", "MDR", "RBB",
    "ORF", "SRF", "BBC", "CNN", "AFP", "DPA", "AP", "NPR", "PBS", "ABC",
    "NBC", "CBS", "FOX", "MSNBC", "RT", "Al Jazeera",
    // Finance/Economy
    "CO2", "BIP", "GDP", "BTW", "DAX", "DOW", "MDAX", "SDAX", "TecDAX",
    "S&P", "MSCI", "NASDAQ", "NYSE", "ETF", "ESG", "IPO", "SPO",
    "EPS", "KGV", "EBIT", "EBITDA", "ROI", "ROE", "WACC",
    "EUR", "USD", "GBP", "CHF", "JPY", "CNY", "BTC", "ETH",
    // Transportation
    "ÖPNV", "ICE", "TGV", "AVE", "TGV", "SUV", "EV", "BEV", "PHEV", "HEV",
    "BER", "MUC", "FRA", "DUS", "HAM", "CGN", "STR", "LHR", "CDG", "JFK",
    // Sports
    "DFB", "DFL", "UEFA", "FIFA", "IOC", "DOSB", "NOK", "CAS", "WADA",
    "NBA", "NFL", "NHL", "MLB", "MLS", "F1", "DTM", "WRC", "MotoGP",
    // Medical/Pharma
    "RKI", "PEI", "STIKO", "EMA", "FDA", "CDC", "NHS", "WHO",
    "mRNA", "PCR", "CT", "MRT", "EKG", "EEG", "HIV", "HPV", "RSV",
    // Military/Security
    "MAD", "NSA", "CIA", "FBI", "MI5", "MI6", "FSB", "GRU", "Mossad",
    "ISAF", "KFOR", "UNIFIL", "ABC", "CBRN", "MANPADS", "HIMARS", "ATACMS",
];

/// Known concepts (technology, science, economics, etc.)
pub static KNOWN_CONCEPTS: &[&str] = &[
    // === Technology & IT ===
    "Computer", "Software", "Hardware", "Informatik", "Programmierung",
    "Algorithmus", "Algorithmen", "Datenbank", "Netzwerk", "Server", "Cloud",
    "Internet", "Cybersecurity", "Cyberangriff", "Cyberattacke", "Hacking",
    "Ransomware", "Malware", "Phishing", "Firewall", "Verschlüsselung",
    "Blockchain", "Kryptowährung", "Bitcoin", "Ethereum", "Smart Contract",
    "Machine Learning", "Deep Learning", "Neural Network", "Neuronales Netz",
    "Natural Language Processing", "Computer Vision", "Reinforcement Learning",
    "Generative AI", "Large Language Model", "Transformer", "ChatGPT", "GPT-4",
    "Chatbot", "Sprachmodell", "Künstliche Intelligenz", "Maschinelles Lernen",
    "Autonomes Fahren", "Robotik", "Automation", "Digitalisierung",
    "Quantencomputer", "Quantencomputing", "Supercomputer",
    "Open Source", "Linux", "Windows", "macOS", "Android", "iOS",
    "Betriebssystem", "Browser", "App", "Smartphone", "Tablet",
    "Rechenzentrum", "Serverraum", "Glasfaser", "Breitband", "Mobilfunk",
    "Streaming", "E-Commerce", "Fintech", "Startup", "Unicorn",
    // === Science ===
    "Wissenschaft", "Forschung", "Studie", "Experiment", "Labor",
    "Klimawandel", "Erderwärmung", "Treibhauseffekt", "Klimakrise",
    "Erneuerbare Energien", "Solarenergie", "Windenergie", "Wasserstoff",
    "Kernenergie", "Atomkraft", "Kernfusion", "Kernspaltung",
    "Elektromobilität", "Batterie", "Akkumulator", "Lithium-Ionen",
    "Photovoltaik", "Solarzelle", "Windkraftanlage", "Offshore-Wind",
    "Gentechnik", "CRISPR", "Genomeditierung", "Stammzellen",
    "Evolution", "Mutation", "DNA", "RNA", "Genetik", "Genomik",
    "Astronomie", "Astrophysik", "Schwarzes Loch", "Galaxie", "Universum",
    "Kosmologie", "Raumfahrt", "Satellit", "Weltraummission", "Mars-Mission",
    "Physik", "Chemie", "Biologie", "Mathematik", "Geologie",
    "Teilchenphysik", "Quantenmechanik", "Relativitätstheorie",
    // === Economy & Finance ===
    "Wirtschaft", "Ökonomie", "Konjunktur", "Rezession", "Inflation",
    "Deflation", "Stagflation", "Zinsen", "Leitzins", "Geldpolitik",
    "Fiskalpolitik", "Haushalt", "Staatshaushalt", "Staatsschulden",
    "Bruttoinlandsprodukt", "Wachstum", "Arbeitslosigkeit", "Beschäftigung",
    "Börse", "Aktienmarkt", "Anleihen", "Derivate", "Optionen", "Futures",
    "Hedgefonds", "Private Equity", "Venture Capital", "Investment",
    "Fusion", "Übernahme", "Insolvenz", "Bankrott", "Restrukturierung",
    "Lieferkette", "Supply Chain", "Globalisierung", "Protektionismus",
    "Freihandel", "Zölle", "Handelsabkommen", "Sanktionen", "Embargo",
    "Steuern", "Mehrwertsteuer", "Einkommensteuer", "Körperschaftsteuer",
    "Subvention", "Bürgergeld", "Mindestlohn", "Tarifvertrag",
    // === Politics & Society ===
    "Demokratie", "Autokratie", "Diktatur", "Populismus", "Extremismus",
    "Nationalismus", "Separatismus", "Terrorismus", "Radikalisierung",
    "Menschenrechte", "Grundrechte", "Verfassung", "Grundgesetz",
    "Parlamentarismus", "Gewaltenteilung", "Rechtsstaatlichkeit",
    "Wahlen", "Referendum", "Koalition", "Opposition", "Regierung",
    "Gesetzgebung", "Verordnung", "Richtlinie", "Gesetz", "Reform",
    "Migration", "Flucht", "Asyl", "Integration", "Einwanderung",
    "Sozialpolitik", "Rentenpolitik", "Gesundheitspolitik", "Bildungspolitik",
    "Außenpolitik", "Innenpolitik", "Sicherheitspolitik", "Verteidigungspolitik",
    "Diplomatie", "Multilateralismus", "Unilateralismus",
    // === Law & Justice ===
    "Recht", "Gesetz", "Rechtsprechung", "Justiz", "Gericht",
    "Strafrecht", "Zivilrecht", "Verwaltungsrecht", "Völkerrecht",
    "Urteil", "Prozess", "Anklage", "Berufung", "Revision",
    "Datenschutz", "Privatsphäre", "Urheberrecht", "Patentrecht",
    // === Environment & Energy ===
    "Umwelt", "Umweltschutz", "Naturschutz", "Artenschutz", "Biodiversität",
    "Nachhaltigkeit", "Klimaschutz", "Energiewende", "Kohleausstieg",
    "Atomausstieg", "Dekarbonisierung", "Emissionen", "Treibhausgase",
    "Luftverschmutzung", "Wasserverschmutzung", "Plastikmüll", "Recycling",
    "Kreislaufwirtschaft", "Ressourceneffizienz",
    // === Health & Medicine ===
    "Gesundheit", "Medizin", "Pharma", "Impfung", "Impfstoff", "Vakzin",
    "Pandemie", "Epidemie", "Virus", "Bakterien", "Infektion",
    "Krankheit", "Therapie", "Behandlung", "Diagnose", "Prävention",
    "Krankenhaus", "Klinik", "Arzt", "Pflege", "Pflegekräfte",
    "Gesundheitssystem", "Krankenkasse", "Krankenversicherung",
    "Krebs", "Diabetes", "Herz-Kreislauf", "Demenz", "Alzheimer",
    "Antibiotika", "Antibiotikaresistenz", "Organspende", "Transplantation",
    // === Security & Defense ===
    "Sicherheit", "Verteidigung", "Militär", "Bundeswehr", "Streitkräfte",
    "Rüstung", "Aufrüstung", "Abrüstung", "Waffenlieferungen",
    "Krieg", "Konflikt", "Friedensverhandlungen", "Waffenstillstand",
    "Drohne", "Kampfjet", "Panzer", "Rakete", "Munition",
    "Geheimdienst", "Spionage", "Aufklärung", "Überwachung",
    // === Education & Culture ===
    "Bildung", "Schule", "Universität", "Hochschule", "Forschung",
    "Wissenschaft", "Akademie", "Studium", "Ausbildung", "Weiterbildung",
    "Digitale Bildung", "E-Learning", "Homeschooling",
    "Kultur", "Kunst", "Museum", "Theater", "Oper", "Konzert",
    "Literatur", "Film", "Kino", "Musik", "Architektur",
    // === Infrastructure ===
    "Infrastruktur", "Verkehr", "Mobilität", "Transport", "Logistik",
    "Straße", "Autobahn", "Schiene", "Bahnhof", "Flughafen", "Hafen",
    "Brücke", "Tunnel", "Bauwerk", "Bauprojekt", "Stadtentwicklung",
    "Wohnungsbau", "Immobilien", "Miete", "Mietpreisbremse",
];

/// Known sports teams and clubs
pub static KNOWN_SPORTS: &[&str] = &[
    // German Bundesliga
    "Bayern München", "FC Bayern", "Borussia Dortmund", "BVB",
    "RB Leipzig", "Bayer Leverkusen", "Union Berlin", "SC Freiburg",
    "Eintracht Frankfurt", "VfL Wolfsburg", "1. FSV Mainz 05",
    "Borussia Mönchengladbach", "1. FC Köln", "TSG Hoffenheim",
    "FC Augsburg", "VfB Stuttgart", "Werder Bremen", "VfL Bochum",
    "Hertha BSC", "FC Schalke 04", "Hamburger SV", "HSV",
    "Hannover 96", "Fortuna Düsseldorf", "1. FC Nürnberg",
    "SpVgg Greuther Fürth", "FC St. Pauli", "Karlsruher SC",
    "SV Darmstadt 98", "1. FC Heidenheim", "Holstein Kiel",
    // International Football Clubs
    "Real Madrid", "FC Barcelona", "Atlético Madrid", "Sevilla FC",
    "Manchester United", "Manchester City", "Liverpool", "Chelsea",
    "Arsenal", "Tottenham", "Paris Saint-Germain", "PSG",
    "Juventus Turin", "AC Mailand", "Inter Mailand", "AS Rom",
    "SSC Neapel", "Ajax Amsterdam", "Benfica Lissabon", "Porto",
    // Other Sports
    "Formel 1", "Formula 1", "Tour de France", "Giro d'Italia",
    "Olympische Spiele", "Olympia", "Weltmeisterschaft", "WM",
    "Europameisterschaft", "EM", "Champions League", "Europa League",
    "Super Bowl", "NBA Finals", "Stanley Cup", "World Series",
    "Wimbledon", "French Open", "Australian Open", "US Open",
    "Fußball-WM", "Fußball-EM", "DFB-Pokal", "Bundesliga",
    "Premier League", "La Liga", "Serie A", "Ligue 1",
];

/// Initialize the immanentize table with known keywords if they don't exist
pub fn seed_known_keywords(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let mut inserted = 0;

    // Insert known persons
    for name in KNOWN_PERSONS {
        let result = conn.execute(
            "INSERT OR IGNORE INTO immanentize (name, keyword_type, quality_score) VALUES (?1, 'person', 0.9)",
            params![name],
        );
        if let Ok(changes) = result {
            inserted += changes;
        }
    }

    // Insert known organizations
    for name in KNOWN_ORGANIZATIONS {
        let result = conn.execute(
            "INSERT OR IGNORE INTO immanentize (name, keyword_type, quality_score) VALUES (?1, 'organization', 0.9)",
            params![name],
        );
        if let Ok(changes) = result {
            inserted += changes;
        }
    }

    // Insert known locations
    for name in KNOWN_LOCATIONS {
        let result = conn.execute(
            "INSERT OR IGNORE INTO immanentize (name, keyword_type, quality_score) VALUES (?1, 'location', 0.9)",
            params![name],
        );
        if let Ok(changes) = result {
            inserted += changes;
        }
    }

    // Insert known acronyms
    for name in KNOWN_ACRONYMS {
        let result = conn.execute(
            "INSERT OR IGNORE INTO immanentize (name, keyword_type, quality_score) VALUES (?1, 'acronym', 0.9)",
            params![name],
        );
        if let Ok(changes) = result {
            inserted += changes;
        }
    }

    // Insert known concepts
    for name in KNOWN_CONCEPTS {
        let result = conn.execute(
            "INSERT OR IGNORE INTO immanentize (name, keyword_type, quality_score) VALUES (?1, 'concept', 0.9)",
            params![name],
        );
        if let Ok(changes) = result {
            inserted += changes;
        }
    }

    // Insert known sports (as organizations)
    for name in KNOWN_SPORTS {
        let result = conn.execute(
            "INSERT OR IGNORE INTO immanentize (name, keyword_type, quality_score) VALUES (?1, 'organization', 0.9)",
            params![name],
        );
        if let Ok(changes) = result {
            inserted += changes;
        }
    }

    info!("Seeded {} known keywords into immanentize table", inserted);
    Ok(inserted)
}

/// Update keyword types for existing keywords based on known entities
pub fn update_types_from_seeds(conn: &Connection) -> Result<usize, rusqlite::Error> {
    let mut updated = 0;

    // Update persons
    for name in KNOWN_PERSONS {
        let changes = conn.execute(
            "UPDATE immanentize SET keyword_type = 'person' WHERE name = ?1 AND keyword_type != 'person'",
            params![name],
        )?;
        updated += changes;
    }

    // Update organizations
    for name in KNOWN_ORGANIZATIONS {
        let changes = conn.execute(
            "UPDATE immanentize SET keyword_type = 'organization' WHERE name = ?1 AND keyword_type != 'organization'",
            params![name],
        )?;
        updated += changes;
    }

    // Update locations
    for name in KNOWN_LOCATIONS {
        let changes = conn.execute(
            "UPDATE immanentize SET keyword_type = 'location' WHERE name = ?1 AND keyword_type != 'location'",
            params![name],
        )?;
        updated += changes;
    }

    // Update acronyms
    for name in KNOWN_ACRONYMS {
        let changes = conn.execute(
            "UPDATE immanentize SET keyword_type = 'acronym' WHERE name = ?1 AND keyword_type != 'acronym'",
            params![name],
        )?;
        updated += changes;
    }

    // Update concepts
    for name in KNOWN_CONCEPTS {
        let changes = conn.execute(
            "UPDATE immanentize SET keyword_type = 'concept' WHERE name = ?1 AND keyword_type != 'concept'",
            params![name],
        )?;
        updated += changes;
    }

    // Update sports (as organizations)
    for name in KNOWN_SPORTS {
        let changes = conn.execute(
            "UPDATE immanentize SET keyword_type = 'organization' WHERE name = ?1 AND keyword_type != 'organization'",
            params![name],
        )?;
        updated += changes;
    }

    info!("Updated {} keyword types from seed data", updated);
    Ok(updated)
}

/// Check if a keyword matches a known entity and return its type
pub fn get_known_keyword_type(keyword: &str) -> Option<&'static str> {
    let keyword_lower = keyword.to_lowercase();

    // Check persons (case-insensitive)
    if KNOWN_PERSONS.iter().any(|p| p.to_lowercase() == keyword_lower) {
        return Some("person");
    }

    // Check organizations (includes sports teams)
    if KNOWN_ORGANIZATIONS.iter().any(|o| o.to_lowercase() == keyword_lower) {
        return Some("organization");
    }

    // Check sports teams (also organizations)
    if KNOWN_SPORTS.iter().any(|s| s.to_lowercase() == keyword_lower) {
        return Some("organization");
    }

    // Check locations
    if KNOWN_LOCATIONS.iter().any(|l| l.to_lowercase() == keyword_lower) {
        return Some("location");
    }

    // Check acronyms (case-sensitive for acronyms)
    if KNOWN_ACRONYMS.contains(&keyword) {
        return Some("acronym");
    }

    // Check concepts (case-insensitive)
    if KNOWN_CONCEPTS.iter().any(|c| c.to_lowercase() == keyword_lower) {
        return Some("concept");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_keyword_type() {
        // Persons
        assert_eq!(get_known_keyword_type("Olaf Scholz"), Some("person"));
        assert_eq!(get_known_keyword_type("olaf scholz"), Some("person"));
        // Organizations
        assert_eq!(get_known_keyword_type("Deutsche Bank"), Some("organization"));
        // Sports teams (also organizations)
        assert_eq!(get_known_keyword_type("Bayern München"), Some("organization"));
        assert_eq!(get_known_keyword_type("BVB"), Some("organization"));
        // Locations
        assert_eq!(get_known_keyword_type("Berlin"), Some("location"));
        // Acronyms
        assert_eq!(get_known_keyword_type("NATO"), Some("acronym"));
        assert_eq!(get_known_keyword_type("IT"), Some("acronym"));
        assert_eq!(get_known_keyword_type("AI"), Some("acronym"));
        // Concepts
        assert_eq!(get_known_keyword_type("Computer"), Some("concept"));
        assert_eq!(get_known_keyword_type("Klimawandel"), Some("concept"));
        assert_eq!(get_known_keyword_type("Künstliche Intelligenz"), Some("concept"));
        // Unknown
        assert_eq!(get_known_keyword_type("Unknown Entity"), None);
    }

    #[test]
    fn test_seed_data_counts() {
        assert!(KNOWN_PERSONS.len() > 50, "Should have many known persons");
        assert!(KNOWN_ORGANIZATIONS.len() > 50, "Should have many known organizations");
        assert!(KNOWN_LOCATIONS.len() > 100, "Should have many known locations");
        assert!(KNOWN_ACRONYMS.len() > 100, "Should have many known acronyms");
        assert!(KNOWN_CONCEPTS.len() > 150, "Should have many known concepts");
        assert!(KNOWN_SPORTS.len() > 30, "Should have many known sports teams");
    }
}
