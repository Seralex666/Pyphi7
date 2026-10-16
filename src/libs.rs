//! Zusatz-Bibliotheken (pip): Auswahlliste, Eingabepruefung, Auswertung von `pip list`.

pub struct Lib {
    pub pkg: &'static str,
    pub info: &'static str,
}

pub const CURATED: &[Lib] = &[
    Lib { pkg: "pyinstaller", info: "EXE-Export (Projekt > Als EXE exportieren)" },
    Lib { pkg: "PySide6", info: "Qt-Toolkit für die erzeugten Qt-Programme" },
    Lib { pkg: "Pillow", info: "Bilder (JPG u. a.) in Tkinter-Programmen" },
    Lib { pkg: "requests", info: "Webseiten und Web-APIs abfragen" },
    Lib { pkg: "numpy", info: "Zahlen und Matrizen" },
    Lib { pkg: "pandas", info: "Tabellen und Datenanalyse" },
    Lib { pkg: "matplotlib", info: "Diagramme zeichnen" },
    Lib { pkg: "openpyxl", info: "Excel-Dateien lesen und schreiben" },
    Lib { pkg: "pyserial", info: "Serielle Schnittstellen (COM-Ports)" },
    Lib { pkg: "psutil", info: "System- und Prozessinformationen" },
];

/// Schon in Python enthalten (nichts zu installieren).
pub const BUILTIN: &[Lib] = &[
    Lib { pkg: "sqlite3", info: "Datenbank (SQL) – Komponenten Database und Grid im Reiter „Daten“" },
    Lib { pkg: "tkinter", info: "Oberfläche der Tkinter-Programme" },
    Lib { pkg: "json, csv", info: "Dateien lesen und schreiben" },
    Lib { pkg: "decimal, datetime", info: "Geldbeträge und Datum/Uhrzeit" },
];

/// pip-Namen sind unabhaengig von Gross-/Kleinschreibung und `_`/`-`/`.`.
pub fn norm(name: &str) -> String {
    name.trim().to_lowercase().replace(['_', '.'], "-")
}

/// Prueft eine Eingabe wie "requests beautifulsoup4==4.12" und liefert die einzelnen Angaben.
/// Optionen (beginnend mit '-'), Pfade und URLs werden abgelehnt.
pub fn parse_specs(input: &str) -> Result<Vec<String>, String> {
    let specs: Vec<String> = input.split_whitespace().map(|s| s.to_string()).collect();
    if specs.is_empty() {
        return Err("Bitte einen Paketnamen eingeben, z. B. requests".into());
    }
    for s in &specs {
        let first_ok = s.chars().next().map(|c| c.is_ascii_alphanumeric()).unwrap_or(false);
        let all_ok = s.chars().all(|c| c.is_ascii_alphanumeric() || "._-=<>!~[],".contains(c));
        if !first_ok || !all_ok {
            return Err(format!("Ungültiger Paketname: {}", s));
        }
    }
    Ok(specs)
}

/// Name ohne Versionsangabe/Extras ("requests>=2" -> "requests").
pub fn base_name(spec: &str) -> String {
    spec.split(|c| "=<>!~[,".contains(c)).next().unwrap_or(spec).to_string()
}

/// Wertet die Ausgabe von `pip list --format=json` aus: (normierter Name, Anzeigename, Version).
pub fn parse_pip_list(json: &str) -> Result<Vec<(String, String, String)>, String> {
    let v: serde_json::Value = serde_json::from_str(json.trim()).map_err(|e| e.to_string())?;
    let arr = v.as_array().ok_or("unerwartetes Format")?;
    let mut out: Vec<(String, String, String)> = arr
        .iter()
        .filter_map(|e| {
            let n = e.get("name")?.as_str()?;
            let ver = e.get("version")?.as_str()?;
            Some((norm(n), n.to_string(), ver.to_string()))
        })
        .collect();
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs() {
        assert_eq!(parse_specs("requests  numpy>=1.2").unwrap(), vec!["requests", "numpy>=1.2"]);
        assert!(parse_specs("").is_err());
        assert!(parse_specs("--index-url").is_err());
        assert!(parse_specs("x; rm -rf").is_err());
        assert!(parse_specs("http://evil/x.zip").is_err());
        assert!(parse_specs("./lokal").is_err());
        assert_eq!(base_name("numpy>=1.2"), "numpy");
        assert_eq!(base_name("uvicorn[standard]"), "uvicorn");
    }

    #[test]
    fn pip_list() {
        let l = parse_pip_list(r#"[{"name":"PyInstaller","version":"6.22.3"},{"name":"typing_extensions","version":"4"}]"#).unwrap();
        assert_eq!(l[0].0, "pyinstaller");
        assert!(l.iter().any(|e| e.0 == "typing-extensions"));
        assert!(parse_pip_list("kaputt").is_err());
        assert_eq!(norm("Py_Side.6"), "py-side-6");
    }
}
