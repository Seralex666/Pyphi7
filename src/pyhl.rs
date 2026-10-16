//! Python-Syntax-Highlighter fuer den Code-Editor: Schluesselwoerter, eingebaute
//! Funktionen, Funktions-/Klassennamen, Aufrufe, Dekoratoren, self/ui, Zahlen,
//! Strings (auch f-Strings mit {Ausdruecken}), Docstrings, Kommentare, Operatoren.

use crate::model::is_py_keyword;
use eframe::egui::{text::LayoutJob, text::TextFormat, Color32, FontId};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Normal,
    Keyword,
    Builtin,
    DefName,
    Call,
    Decorator,
    SelfKw,
    Ui,
    Number,
    Str,
    FExpr,
    Doc,
    Comment,
    Op,
}

const BUILTINS: &[&str] = &[
    "print", "len", "range", "int", "float", "str", "bool", "list", "dict", "set", "tuple", "sum", "min", "max", "abs",
    "round", "sorted", "reversed", "enumerate", "zip", "map", "filter", "any", "all", "open", "input", "type",
    "isinstance", "issubclass", "hasattr", "getattr", "setattr", "delattr", "repr", "format", "chr", "ord", "hex",
    "bin", "oct", "divmod", "pow", "id", "iter", "next", "callable", "super", "object", "bytes", "bytearray",
    "frozenset", "slice", "vars", "dir", "globals", "locals", "exec", "eval", "compile", "property", "staticmethod",
    "classmethod", "Exception", "ValueError", "TypeError", "KeyError", "IndexError", "OSError", "RuntimeError",
    "ZeroDivisionError", "AttributeError", "StopIteration", "NotImplementedError", "ImportError",
];

fn style(kind: Kind, font: &FontId, dark: bool) -> TextFormat {
    let c = Color32::from_rgb;
    let (color, italics) = if dark {
        match kind {
            Kind::Normal => (c(0xd4, 0xd4, 0xd4), false),
            Kind::Keyword => (c(0x56, 0x9c, 0xd6), false),
            Kind::Builtin => (c(0xc5, 0x86, 0xc0), false),
            Kind::DefName => (c(0x4e, 0xc9, 0xb0), false),
            Kind::Call => (c(0xdc, 0xdc, 0xaa), false),
            Kind::Decorator => (c(0xd7, 0xba, 0x7d), false),
            Kind::SelfKw => (c(0x9c, 0xdc, 0xfe), true),
            Kind::Ui => (c(0xff, 0x7e, 0xb6), false),
            Kind::Number => (c(0xb5, 0xce, 0xa8), false),
            Kind::Str => (c(0xce, 0x91, 0x78), false),
            Kind::FExpr => (c(0x7f, 0xb7, 0xff), false),
            Kind::Doc => (c(0x7f, 0x9a, 0x7f), true),
            Kind::Comment => (c(0x6a, 0x99, 0x55), true),
            Kind::Op => (c(0xaa, 0xb0, 0xbb), false),
        }
    } else {
        match kind {
            Kind::Normal => (c(0x10, 0x10, 0x10), false),
            Kind::Keyword => (c(0x00, 0x00, 0xc0), false),
            Kind::Builtin => (c(0x7a, 0x3e, 0x9d), false),
            Kind::DefName => (c(0x00, 0x62, 0x7a), false),
            Kind::Call => (c(0x79, 0x5e, 0x26), false),
            Kind::Decorator => (c(0x9e, 0x88, 0x0d), false),
            Kind::SelfKw => (c(0x94, 0x55, 0x8d), true),
            Kind::Ui => (c(0xb0, 0x00, 0x6a), false),
            Kind::Number => (c(0x09, 0x86, 0x58), false),
            Kind::Str => (c(0xa3, 0x15, 0x15), false),
            Kind::FExpr => (c(0x1a, 0x5a, 0xa0), false),
            Kind::Doc => (c(0x4d, 0x6b, 0x4d), true),
            Kind::Comment => (c(0x00, 0x80, 0x00), true),
            Kind::Op => (c(0x40, 0x40, 0x40), false),
        }
    };
    TextFormat { font_id: font.clone(), color, italics, ..Default::default() }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}
fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
fn is_quote(c: char) -> bool {
    c == '"' || c == '\''
}
fn is_prefix(w: &str) -> bool {
    !w.is_empty() && w.len() <= 2 && w.chars().all(|c| "rRbBfFuU".contains(c))
}

/// Zerlegt den Text in (Art, Bytebereich)-Abschnitte, lueckenlos.
fn tokenize(text: &str) -> Vec<(Kind, usize, usize)> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let n = chars.len();
    let ch = |i: usize| -> char { if i < n { chars[i].1 } else { '\0' } };
    let byte = |i: usize| if i < n { chars[i].0 } else { text.len() };
    let mut out: Vec<(Kind, usize, usize)> = Vec::new();
    let mut push = |k: Kind, a: usize, b: usize| {
        if a >= b {
            return;
        }
        if let Some(last) = out.last_mut() {
            if last.0 == k && last.2 == a {
                last.2 = b;
                return;
            }
        }
        out.push((k, a, b));
    };

    let mut i = 0;
    let mut line_start = true; // vor dem ersten Zeichen der Zeile nur Leerraum
    let mut prev_word: Option<String> = None;
    let mut prev_dot = false;

    while i < n {
        let c = ch(i);
        let start = i;
        if c == '\n' {
            i += 1;
            push(Kind::Normal, byte(start), byte(i));
            line_start = true;
            prev_word = None;
            prev_dot = false;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            push(Kind::Normal, byte(start), byte(i));
            continue;
        }
        let was_line_start = line_start;
        line_start = false;

        if c == '#' {
            while i < n && ch(i) != '\n' {
                i += 1;
            }
            push(Kind::Comment, byte(start), byte(i));
            continue;
        }

        if c == '@' && was_line_start {
            i += 1;
            while i < n && (is_ident(ch(i)) || ch(i) == '.') {
                i += 1;
            }
            push(Kind::Decorator, byte(start), byte(i));
            continue;
        }

        // Strings (auch mit Praefix r/b/f/u): Praefix wird beim Bezeichner erkannt
        let mut str_prefix_end: Option<usize> = None; // Index des ersten Anfuehrungszeichens
        let mut prefix_text = String::new();
        if is_quote(c) {
            str_prefix_end = Some(i);
        } else if is_ident_start(c) {
            let mut j = i;
            while j < n && is_ident(ch(j)) {
                j += 1;
            }
            let w: String = chars[i..j].iter().map(|(_, c)| *c).collect();
            if is_prefix(&w) && is_quote(ch(j)) {
                str_prefix_end = Some(j);
                prefix_text = w.to_ascii_lowercase();
            }
        }
        if let Some(q) = str_prefix_end {
            let quote = ch(q);
            let is_f = prefix_text.contains('f');
            let triple = ch(q + 1) == quote && ch(q + 2) == quote;
            let mut j = q + if triple { 3 } else { 1 };
            let base = if triple && was_line_start && !is_f { Kind::Doc } else { Kind::Str };
            let mut seg_start = start;
            let mut seg_kind = base;
            loop {
                if j >= n {
                    break;
                }
                let cj = ch(j);
                if cj == '\\' && j + 1 < n && !(ch(j + 1) == '\n' && !triple) {
                    j += 2;
                    continue;
                }
                if !triple && cj == '\n' {
                    break; // unbeendeter String endet am Zeilenende
                }
                if triple {
                    if cj == quote && ch(j + 1) == quote && ch(j + 2) == quote {
                        j += 3;
                        break;
                    }
                } else if cj == quote {
                    j += 1;
                    break;
                }
                if is_f && cj == '{' {
                    if ch(j + 1) == '{' {
                        j += 2;
                        continue;
                    }
                    // Ausdruck bis zur passenden schliessenden Klammer
                    push(seg_kind, byte(seg_start), byte(j));
                    let es = j;
                    let mut depth = 0;
                    while j < n {
                        let cc = ch(j);
                        if cc == '{' {
                            depth += 1;
                        } else if cc == '}' {
                            depth -= 1;
                            if depth == 0 {
                                j += 1;
                                break;
                            }
                        } else if cc == '\n' && !triple {
                            break;
                        }
                        j += 1;
                    }
                    push(Kind::FExpr, byte(es), byte(j));
                    seg_start = j;
                    seg_kind = base;
                    continue;
                }
                j += 1;
            }
            push(seg_kind, byte(seg_start), byte(j.min(n)));
            i = j.min(n);
            prev_word = None;
            prev_dot = false;
            continue;
        }

        // Zahlen
        if (c.is_ascii_digit() || (c == '.' && ch(i + 1).is_ascii_digit())) && !(i > 0 && is_ident(ch(i - 1))) {
            let hex = c == '0' && matches!(ch(i + 1), 'x' | 'X' | 'b' | 'B' | 'o' | 'O');
            i += 1;
            while i < n {
                let cc = ch(i);
                if cc.is_ascii_alphanumeric() || cc == '_' {
                    i += 1;
                } else if cc == '.' && !hex && ch(i + 1) != '.' && !is_ident_start(ch(i + 1)) {
                    i += 1;
                } else if (cc == '+' || cc == '-') && !hex && matches!(ch(i - 1), 'e' | 'E') {
                    i += 1;
                } else {
                    break;
                }
            }
            push(Kind::Number, byte(start), byte(i));
            prev_word = None;
            prev_dot = false;
            continue;
        }

        // Bezeichner / Schluesselwoerter
        if is_ident_start(c) {
            while i < n && is_ident(ch(i)) {
                i += 1;
            }
            let word: String = chars[start..i].iter().map(|(_, c)| *c).collect();
            let mut k = i;
            while k < n && (ch(k) == ' ' || ch(k) == '\t') {
                k += 1;
            }
            let called = ch(k) == '(';
            let kind = if matches!(prev_word.as_deref(), Some("def") | Some("class")) && !prev_dot {
                Kind::DefName
            } else if prev_dot {
                if called { Kind::Call } else { Kind::Normal }
            } else if is_py_keyword(&word) {
                Kind::Keyword
            } else if word == "self" || word == "cls" {
                Kind::SelfKw
            } else if word == "ui" {
                Kind::Ui
            } else if BUILTINS.contains(&word.as_str()) {
                Kind::Builtin
            } else if called {
                Kind::Call
            } else {
                Kind::Normal
            };
            push(kind, byte(start), byte(i));
            prev_word = Some(word);
            prev_dot = false;
            continue;
        }

        // Operatoren und Klammern
        i += 1;
        if "+-*/%=<>!&|^~:,;()[]{}.".contains(c) {
            push(Kind::Op, byte(start), byte(i));
            prev_dot = c == '.';
            if c != '.' {
                prev_word = None;
            }
        } else {
            push(Kind::Normal, byte(start), byte(i));
            prev_word = None;
            prev_dot = false;
        }
    }
    out
}

pub fn highlight(text: &str, font: &FontId, dark: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (kind, a, b) in tokenize(text) {
        job.append(&text[a..b], 0.0, style(kind, font, dark));
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(s: &str) -> Vec<(char, String)> {
        tokenize(s)
            .into_iter()
            .map(|(k, a, b)| {
                let c = match k {
                    Kind::Normal => 'n',
                    Kind::Keyword => 'k',
                    Kind::Builtin => 'b',
                    Kind::DefName => 'f',
                    Kind::Call => 'c',
                    Kind::Decorator => '@',
                    Kind::SelfKw => 's',
                    Kind::Ui => 'u',
                    Kind::Number => '1',
                    Kind::Str => '"',
                    Kind::FExpr => 'x',
                    Kind::Doc => 'd',
                    Kind::Comment => '#',
                    Kind::Op => 'o',
                };
                (c, s[a..b].to_string())
            })
            .collect()
    }

    fn has(t: &[(char, String)], k: char, s: &str) -> bool {
        t.iter().any(|(kk, ss)| *kk == k && ss == s)
    }

    #[test]
    fn tokens() {
        let t = kinds("def foo(x):  # hi\n    return len(\"a#b\") + 0x1F\n");
        assert!(has(&t, 'k', "def") && has(&t, 'f', "foo") && has(&t, '#', "# hi"));
        assert!(has(&t, 'b', "len") && has(&t, '"', "\"a#b\"") && has(&t, '1', "0x1F") && has(&t, 'k', "return"));
    }

    #[test]
    fn calls_ui_self_decorators() {
        let t = kinds("@property\ndef f(self):\n    ui.set_text(\"a\", x.y(1))\n    print(ui)\n");
        assert!(has(&t, '@', "@property") && has(&t, 's', "self") && has(&t, 'u', "ui"));
        assert!(has(&t, 'c', "set_text") && has(&t, 'c', "y") && has(&t, 'b', "print"));
        assert!(!t.iter().any(|(k, s)| *k == 'k' && s == "print"));
    }

    #[test]
    fn fstrings_and_numbers() {
        let t = kinds("s = f\"Summe: {euro(x):>8} {{lit}}\"\nz = 1_000.5e-3 + .5\n");
        assert!(has(&t, 'x', "{euro(x):>8}"));
        assert!(t.iter().any(|(k, s)| *k == '"' && s.contains("{{lit}}")));
        assert!(has(&t, '1', "1_000.5e-3") && has(&t, '1', ".5"));
    }

    #[test]
    fn docstring_and_unicode() {
        let t = kinds("\"\"\"Gruß\nzwei\"\"\"\nx = 'ä'");
        assert_eq!(t[0].0, 'd');
        assert!(t[0].1.ends_with("\"\"\""));
        assert!(has(&t, '"', "'ä'"));
        let src = "a = 'b' # c\n";
        let joined: String = tokenize(src).iter().map(|(_, a, b)| &src[*a..*b]).collect();
        assert_eq!(joined, src);
    }

    #[test]
    fn always_covers_everything_and_never_panics() {
        for s in [
            "\"\"\"abc", "'abc", "\"", "#", "\"\"", "'''", "x = \"a\\", "1.", "", "f\"{", "f\"{a", "rb'x'", "@", "a.b.c(",
            "def", "x = 1e", "0x", "é = 2", "f'''{x}\n{y}'''", "\\", "$ ? `",
        ] {
            let t = tokenize(s);
            let joined: String = t.iter().map(|(_, a, b)| &s[*a..*b]).collect();
            assert_eq!(joined, s, "lueckenlos fuer {:?}", s);
        }
    }
}
