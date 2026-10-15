//! Editor-Verhalten wie in einer IDE (ohne GUI, testbar). Alle Positionen sind
//! Zeichenindizes. Jede Funktion liefert eine Aenderung: der Bereich `from..to`
//! des Originals wird durch `insert` ersetzt, danach gilt die Auswahl `sel`.

pub const INDENT: &str = "    ";

#[derive(Debug, PartialEq, Eq)]
pub struct Edit {
    pub from: usize,
    pub to: usize,
    pub insert: String,
    pub sel: (usize, usize),
}

fn chars_of(text: &str) -> Vec<char> {
    text.chars().collect()
}

fn line_start(c: &[char], pos: usize) -> usize {
    let mut i = pos.min(c.len());
    while i > 0 && c[i - 1] != '\n' {
        i -= 1;
    }
    i
}

fn line_end(c: &[char], pos: usize) -> usize {
    let mut i = pos.min(c.len());
    while i < c.len() && c[i] != '\n' {
        i += 1;
    }
    i
}

fn s(c: &[char]) -> String {
    c.iter().collect()
}

fn closer(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

/// Enter: Einrueckung der Zeile uebernehmen, nach ':' vertiefen, nach return/pass/...
/// zurueck, zwischen Klammern "()" die Klammer in eine eigene Zeile setzen.
pub fn enter(text: &str, a: usize, b: usize) -> Edit {
    let c = chars_of(text);
    let (a, b) = (a.min(b).min(c.len()), a.max(b).min(c.len()));
    let ls = line_start(&c, a);
    let before = s(&c[ls..a]);
    let mut indent: String = before.chars().take_while(|ch| *ch == ' ' || *ch == '\t').collect();
    let code = before.split('#').next().unwrap_or("").trim_end().to_string();
    let first = code.trim_start().split(|ch: char| !ch.is_alphanumeric() && ch != '_').next().unwrap_or("").to_string();
    let prev = c.get(a.wrapping_sub(1)).copied().filter(|_| a > 0);
    let next = c.get(b).copied();
    if code.ends_with(':') {
        indent.push_str(INDENT);
    } else if matches!(first.as_str(), "return" | "pass" | "break" | "continue" | "raise") && indent.len() >= INDENT.len() {
        // nach return/pass/... eine Ebene zurueck (nur wenn die Zeile vollstaendig ist)
        let opens = code.matches(|ch| "([{".contains(ch)).count();
        let closes = code.matches(|ch| ")]}".contains(ch)).count();
        if opens <= closes {
            let keep = indent.len() - INDENT.len();
            indent.truncate(keep);
        }
    } else if let Some(p) = prev {
        if closer(p).is_some() {
            indent.push_str(INDENT);
        }
    }
    // Klammerpaar "(|)" -> zwei Zeilen, Cursor eingerueckt dazwischen
    if let (Some(p), Some(n)) = (prev, next) {
        if closer(p) == Some(n) {
            let base: String = before.chars().take_while(|ch| *ch == ' ' || *ch == '\t').collect();
            let ins = format!("\n{}{}\n{}", base, INDENT, base);
            let pos = a + 1 + base.chars().count() + INDENT.len();
            return Edit { from: a, to: b, insert: ins, sel: (pos, pos) };
        }
    }
    let ins = format!("\n{}", indent);
    let pos = a + ins.chars().count();
    Edit { from: a, to: b, insert: ins, sel: (pos, pos) }
}

/// Tab / Umschalt+Tab. Ohne Auswahl: bis zum naechsten 4er-Tabstopp einruecken
/// (Shift: Zeile ausruecken). Mit mehrzeiliger Auswahl: alle Zeilen ein-/ausruecken.
pub fn tab(text: &str, a: usize, b: usize, shift: bool) -> Edit {
    let c = chars_of(text);
    let (a, b) = (a.min(b).min(c.len()), a.max(b).min(c.len()));
    let multi = c[a..b].contains(&'\n');
    if !shift && !multi {
        let col = a - line_start(&c, a);
        let n = INDENT.len() - col % INDENT.len();
        let ins = " ".repeat(n);
        return Edit { from: a, to: b, insert: ins, sel: (a + n, a + n) };
    }
    // betroffene Zeilen (eine Zeile, die nur am Anfang beruehrt wird, zaehlt nicht mit)
    let first = line_start(&c, a);
    let last_pos = if b > a && b > 0 && c[b - 1] == '\n' { b - 1 } else { b };
    let last_end = line_end(&c, last_pos);
    let block = s(&c[first..last_end]);
    let mut out = Vec::new();
    let (mut da, mut db) = (0isize, 0isize);
    let mut offset = first;
    for line in block.split('\n') {
        let len = line.chars().count();
        let new_line = if shift {
            let mut k = 0;
            let lc: Vec<char> = line.chars().collect();
            while k < INDENT.len() && k < lc.len() && lc[k] == ' ' {
                k += 1;
            }
            if k == 0 && lc.first() == Some(&'\t') {
                k = 1;
            }
            let removed = k as isize;
            if a >= offset {
                da -= removed.min((a - offset) as isize);
            }
            if b >= offset {
                db -= removed.min((b - offset) as isize);
            }
            lc[k..].iter().collect::<String>()
        } else {
            if !line.trim().is_empty() {
                if a > offset {
                    da += INDENT.len() as isize;
                }
                if b > offset {
                    db += INDENT.len() as isize;
                }
                format!("{}{}", INDENT, line)
            } else {
                line.to_string()
            }
        };
        out.push(new_line);
        offset += len + 1;
    }
    let new_a = (a as isize + da).max(first as isize) as usize;
    let new_b = (b as isize + db).max(first as isize) as usize;
    Edit { from: first, to: last_end, insert: out.join("\n"), sel: (new_a, new_b) }
}

/// Backspace: steht der Cursor in fuehrendem Leerraum, ein ganzes Einrueckungsstueck loeschen;
/// zwischen einem leeren Klammer-/Anfuehrungszeichenpaar beide Zeichen entfernen.
pub fn backspace(text: &str, a: usize, b: usize) -> Option<Edit> {
    let c = chars_of(text);
    let (a, b) = (a.min(b).min(c.len()), a.max(b).min(c.len()));
    if a != b || a == 0 {
        return None;
    }
    let ls = line_start(&c, a);
    let col = a - ls;
    if col > 0 && c[ls..a].iter().all(|ch| *ch == ' ') {
        let n = match col % INDENT.len() {
            0 => INDENT.len(),
            r => r,
        };
        return Some(Edit { from: a - n, to: a, insert: String::new(), sel: (a - n, a - n) });
    }
    let (p, n) = (c[a - 1], c.get(a).copied());
    if let Some(n) = n {
        if closer(p) == Some(n) || ((p == '"' || p == '\'') && p == n) {
            return Some(Edit { from: a - 1, to: a + 1, insert: String::new(), sel: (a - 1, a - 1) });
        }
    }
    None
}

/// Strg+/ : Zeilen ein-/auskommentieren.
pub fn toggle_comment(text: &str, a: usize, b: usize) -> Edit {
    let c = chars_of(text);
    let (a, b) = (a.min(b).min(c.len()), a.max(b).min(c.len()));
    let first = line_start(&c, a);
    let last_pos = if b > a && c[b - 1] == '\n' { b - 1 } else { b };
    let last_end = line_end(&c, last_pos);
    let block = s(&c[first..last_end]);
    let lines: Vec<&str> = block.split('\n').collect();
    let all_commented = lines.iter().filter(|l| !l.trim().is_empty()).all(|l| l.trim_start().starts_with('#'));
    let min_indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().take_while(|ch| *ch == ' ').count())
        .min()
        .unwrap_or(0);
    let mut out = Vec::new();
    for l in &lines {
        if l.trim().is_empty() {
            out.push(l.to_string());
        } else if all_commented {
            let lc: Vec<char> = l.chars().collect();
            let k = lc.iter().position(|ch| *ch == '#').unwrap_or(0);
            let mut rest: Vec<char> = lc[k + 1..].to_vec();
            if rest.first() == Some(&' ') {
                rest.remove(0);
            }
            out.push(format!("{}{}", lc[..k].iter().collect::<String>(), rest.iter().collect::<String>()));
        } else {
            let lc: Vec<char> = l.chars().collect();
            let k = min_indent.min(lc.len());
            out.push(format!("{}# {}", lc[..k].iter().collect::<String>(), lc[k..].iter().collect::<String>()));
        }
    }
    let ins = out.join("\n");
    let end = first + ins.chars().count();
    let sel = if a == b {
        // Cursor an gleicher Textstelle halten
        let delta = ins.chars().count() as isize - block.chars().count() as isize;
        let p = (a as isize + delta).max(first as isize) as usize;
        (p.min(end), p.min(end))
    } else {
        (first, end)
    };
    Edit { from: first, to: last_end, insert: ins, sel }
}

/// Anfuehrungszeichen/Klammern: Zeichen eintippen -> ggf. Paar einfuegen, schliessendes
/// Zeichen ueberspringen, Auswahl umschliessen. `None` = normales Tippen.
pub fn typed(text: &str, a: usize, b: usize, ch: char) -> Option<Edit> {
    let c = chars_of(text);
    let (a, b) = (a.min(b).min(c.len()), a.max(b).min(c.len()));
    let next = c.get(b).copied();
    let prev = if a > 0 { Some(c[a - 1]) } else { None };
    let word_before = prev.map(|p| p.is_alphanumeric() || p == '_').unwrap_or(false);
    if a != b {
        let close = closer(ch).or(if ch == '"' || ch == '\'' { Some(ch) } else { None })?;
        let inner = s(&c[a..b]);
        let ins = format!("{}{}{}", ch, inner, close);
        return Some(Edit { from: a, to: b, insert: ins, sel: (a + 1, a + 1 + inner.chars().count()) });
    }
    if matches!(ch, ')' | ']' | '}' | '"' | '\'') && next == Some(ch) {
        return Some(Edit { from: a, to: a, insert: String::new(), sel: (a + 1, a + 1) });
    }
    if let Some(close) = closer(ch) {
        let next_ok = next.map(|n| n.is_whitespace() || ")]},:;".contains(n)).unwrap_or(true);
        if next_ok {
            return Some(Edit { from: a, to: a, insert: format!("{}{}", ch, close), sel: (a + 1, a + 1) });
        }
        return None;
    }
    if ch == '"' || ch == '\'' {
        let next_ok = next.map(|n| n.is_whitespace() || ")]},:;".contains(n)).unwrap_or(true);
        if next_ok && !word_before && prev != Some(ch) {
            return Some(Edit { from: a, to: a, insert: format!("{}{}", ch, ch), sel: (a + 1, a + 1) });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(text: &str, e: &Edit) -> (String, usize, usize) {
        let c: Vec<char> = text.chars().collect();
        let mut out: String = c[..e.from].iter().collect();
        out.push_str(&e.insert);
        out.extend(c[e.to..].iter());
        (out, e.sel.0, e.sel.1)
    }

    #[test]
    fn enter_indents_and_dedents() {
        let t = "def f():";
        let (r, p, _) = apply(t, &enter(t, 8, 8));
        assert_eq!(r, "def f():\n    ");
        assert_eq!(p, r.chars().count());
        let t = "    x = 1";
        assert_eq!(apply(t, &enter(t, 9, 9)).0, "    x = 1\n    ");
        let t = "    return 5";
        assert_eq!(apply(t, &enter(t, 12, 12)).0, "    return 5\n");
        let t = "        pass";
        assert_eq!(apply(t, &enter(t, 12, 12)).0, "        pass\n    ");
        let t = "if x:  # kommentar:";
        assert!(apply(t, &enter(t, t.chars().count(), t.chars().count())).0.ends_with(":  # kommentar:\n    "));
        let t = "x = foo(";
        assert_eq!(apply(t, &enter(t, 8, 8)).0, "x = foo(\n    ");
    }

    #[test]
    fn enter_between_brackets() {
        let t = "    f()";
        let (r, p, _) = apply(t, &enter(t, 6, 6));
        assert_eq!(r, "    f(\n        \n    )");
        assert_eq!(p, 6 + 1 + 8);
    }

    #[test]
    fn enter_in_middle_of_line_keeps_indent() {
        let t = "    a = 1 + 2";
        let (r, _, _) = apply(t, &enter(t, 9, 9));
        assert_eq!(r, "    a = 1\n     + 2");
    }

    #[test]
    fn tab_single_and_stops() {
        assert_eq!(apply("", &tab("", 0, 0, false)).0, "    ");
        let t = "ab";
        assert_eq!(apply(t, &tab(t, 2, 2, false)).0, "ab  ");
        let t = "      x";
        let (r, p, _) = apply(t, &tab(t, 6, 6, true));
        assert_eq!(r, "  x");
        assert_eq!(p, 2);
    }

    #[test]
    fn tab_multiline() {
        let t = "a\nb\n\nc";
        let (r, a, b) = apply(t, &tab(t, 0, 6, false));
        assert_eq!(r, "    a\n    b\n\n    c");
        assert!(a <= b);
        let (r2, _, _) = apply(&r, &tab(&r, 0, r.chars().count(), true));
        assert_eq!(r2, t);
        // Auswahl endet direkt nach Zeilenumbruch: letzte Zeile nicht anfassen
        let t = "a\nb\n";
        assert_eq!(apply(t, &tab(t, 0, 4, false)).0, "    a\n    b\n");
    }

    #[test]
    fn backspace_indent_and_pairs() {
        let t = "        x";
        assert_eq!(apply(t, &backspace(t, 8, 8).unwrap()).0, "    x");
        let t = "      x";
        assert_eq!(apply(t, &backspace(t, 6, 6).unwrap()).0, "    x");
        assert!(backspace("ab", 1, 1).is_none());
        let t = "f()";
        assert_eq!(apply(t, &backspace(t, 2, 2).unwrap()).0, "f");
        let t = "x = \"\"";
        assert_eq!(apply(t, &backspace(t, 5, 5).unwrap()).0, "x = ");
    }

    #[test]
    fn comment_toggle() {
        let t = "a\n    b";
        let (r, _, _) = apply(t, &toggle_comment(t, 0, 7));
        assert_eq!(r, "# a\n#     b");
        let (r2, _, _) = apply(&r, &toggle_comment(&r, 0, r.chars().count()));
        assert_eq!(r2, t);
        let t = "    x = 1";
        let (r, p, _) = apply(t, &toggle_comment(t, 6, 6));
        assert_eq!(r, "    # x = 1");
        assert_eq!(p, 8);
    }

    #[test]
    fn typed_pairs() {
        let (r, p, _) = apply("", &typed("", 0, 0, '(').unwrap());
        assert_eq!((r.as_str(), p), ("()", 1));
        // ueberspringen
        let (_, p, _) = apply("()", &typed("()", 1, 1, ')').unwrap());
        assert_eq!(p, 2);
        // Auswahl umschliessen
        let (r, a, b) = apply("abc", &typed("abc", 0, 3, '"').unwrap());
        assert_eq!((r.as_str(), a, b), ("\"abc\"", 1, 4));
        // Apostroph im Wort: kein Paar
        assert!(typed("don", 3, 3, '\'').is_none());
        // vor Text kein Paar
        assert!(typed("x", 0, 0, '(').is_none());
        assert_eq!(apply("", &typed("", 0, 0, '"').unwrap()).0, "\"\"");
    }
}
