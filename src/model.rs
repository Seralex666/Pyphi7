//! Datenmodell von Pyphi7: Projekt = Formular + Liste von Komponenten (JSON).
//! Das Dateiformat ist identisch zur Python-Version (.pyrad), nur die Endung ist .pyphi.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum WType {
    Button,
    Label,
    Entry,
    Checkbox,
    Listbox,
    Text,
    Combobox,
    Radiobutton,
    Groupbox,
    Panel,
    Progressbar,
    Trackbar,
    Spinedit,
    Image,
    Timer,
    /// SQLite-Datenbank (nicht sichtbar)
    Database,
    /// Tabelle (Spalten x Zeilen), z. B. fuer Abfrage-Ergebnisse
    Datagrid,
}

/// Palettenseiten wie in Delphi: (Name, Komponenten)
pub const PALETTE: [(&str, &[WType]); 4] = [
    (
        "Standard",
        &[
            WType::Label,
            WType::Entry,
            WType::Text,
            WType::Button,
            WType::Checkbox,
            WType::Radiobutton,
            WType::Listbox,
            WType::Combobox,
            WType::Groupbox,
            WType::Panel,
        ],
    ),
    ("Zusätzlich", &[WType::Progressbar, WType::Trackbar, WType::Spinedit, WType::Image]),
    ("System", &[WType::Timer]),
    ("Bibliotheken", &[WType::Database, WType::Datagrid]),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PK {
    Text,
    Int,
    Bool,
    /// mehrzeilige Liste (Items/Lines) mit Editor-Dialog
    Items,
    Choice(&'static [&'static str]),
    Color,
    File,
}

#[derive(Clone, Copy, Debug)]
pub struct PropDef {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: PK,
    pub def: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct EventDef {
    pub name: &'static str,
    pub suffix: &'static str,
}

pub struct Info {
    pub cls: &'static str,
    pub base: &'static str,
    pub text_prop: &'static str,
    pub w: i32,
    pub h: i32,
    pub event: Option<&'static str>,
    pub suffix: Option<&'static str>,
    pub hint: &'static str,
}

const fn pd(key: &'static str, label: &'static str, kind: PK, def: &'static str) -> PropDef {
    PropDef { key, label, kind, def }
}
const fn ev(name: &'static str, suffix: &'static str) -> EventDef {
    EventDef { name, suffix }
}

const P_HINT: PropDef = pd("hint", "Hint", PK::Text, "");
const P_FSIZE: PropDef = pd("font_size", "Font.Size", PK::Int, "0");
const P_FBOLD: PropDef = pd("font_bold", "Font.Bold", PK::Bool, "False");
const P_FCOLOR: PropDef = pd("font_color", "Font.Color", PK::Color, "");
const ALIGN: &[&str] = &["taLeftJustify", "taCenter", "taRightJustify"];
const BEVEL: &[&str] = &["bvNone", "bvRaised", "bvLowered"];
const CBSTYLE: &[&str] = &["csDropDownList", "csDropDown"];
const ORIENT: &[&str] = &["trHorizontal", "trVertical"];

const EV_FOCUS_KEY: [EventDef; 3] = [ev("OnEnter", "Enter"), ev("OnExit", "Exit"), ev("OnKeyPress", "KeyPress")];

impl WType {
    pub fn as_str(self) -> &'static str {
        match self {
            WType::Button => "Button",
            WType::Label => "Label",
            WType::Entry => "Entry",
            WType::Checkbox => "Checkbox",
            WType::Listbox => "Listbox",
            WType::Text => "Text",
            WType::Combobox => "Combobox",
            WType::Radiobutton => "Radiobutton",
            WType::Groupbox => "Groupbox",
            WType::Panel => "Panel",
            WType::Progressbar => "Progressbar",
            WType::Trackbar => "Trackbar",
            WType::Spinedit => "Spinedit",
            WType::Image => "Image",
            WType::Timer => "Timer",
            WType::Database => "Database",
            WType::Datagrid => "Datagrid",
        }
    }

    pub fn is_list(self) -> bool {
        matches!(self, WType::Listbox | WType::Combobox | WType::Datagrid)
    }

    /// Container zeichnen sich hinter ihren "Kindern" und ziehen diese beim Verschieben mit.
    pub fn is_container(self) -> bool {
        matches!(self, WType::Groupbox | WType::Panel)
    }

    /// Nicht-visuelle Komponente (Timer): nur ein Symbol im Designer.
    pub fn is_visual(self) -> bool {
        !matches!(self, WType::Timer | WType::Database)
    }

    /// Hat Schrift-Eigenschaften (Font.Size / Bold / Color)?
    pub fn has_font(self) -> bool {
        !matches!(self, WType::Progressbar | WType::Trackbar | WType::Image | WType::Timer | WType::Database | WType::Datagrid)
    }

    /// Hat der Typ einen Text (Caption/Text/Items/Lines)?
    pub fn has_text(self) -> bool {
        !matches!(self, WType::Progressbar | WType::Trackbar | WType::Image | WType::Timer | WType::Spinedit | WType::Database)
    }

    pub fn info(self) -> Info {
        match self {
            WType::Button => Info { cls: "TButton", base: "Button", text_prop: "Caption", w: 75, h: 25,
                event: Some("OnClick"), suffix: Some("Click"), hint: "Button - Schaltfläche" },
            WType::Label => Info { cls: "TLabel", base: "Label", text_prop: "Caption", w: 65, h: 17,
                event: None, suffix: None, hint: "Label - Beschriftung" },
            WType::Entry => Info { cls: "TEdit", base: "Edit", text_prop: "Text", w: 121, h: 21,
                event: Some("OnChange"), suffix: Some("Change"), hint: "Edit - Eingabefeld" },
            WType::Checkbox => Info { cls: "TCheckBox", base: "CheckBox", text_prop: "Caption", w: 97, h: 17,
                event: Some("OnClick"), suffix: Some("Click"), hint: "CheckBox - Kontrollkästchen" },
            WType::Listbox => Info { cls: "TListBox", base: "ListBox", text_prop: "Items", w: 121, h: 97,
                event: Some("OnClick"), suffix: Some("Click"), hint: "ListBox - Liste (Items: Zeile pro Eintrag)" },
            WType::Text => Info { cls: "TMemo", base: "Memo", text_prop: "Lines", w: 185, h: 89,
                event: Some("OnChange"), suffix: Some("Change"), hint: "Memo - mehrzeiliges Textfeld" },
            WType::Combobox => Info { cls: "TComboBox", base: "ComboBox", text_prop: "Items", w: 145, h: 21,
                event: Some("OnChange"), suffix: Some("Change"), hint: "ComboBox - Auswahlfeld (Items: Zeile pro Eintrag)" },
            WType::Radiobutton => Info { cls: "TRadioButton", base: "RadioButton", text_prop: "Caption", w: 113, h: 17,
                event: Some("OnClick"), suffix: Some("Click"), hint: "RadioButton - Optionsfeld (GroupIndex = Gruppe)" },
            WType::Groupbox => Info { cls: "TGroupBox", base: "GroupBox", text_prop: "Caption", w: 185, h: 105,
                event: None, suffix: None, hint: "GroupBox - Gruppenrahmen (zieht Inhalt beim Verschieben mit)" },
            WType::Panel => Info { cls: "TPanel", base: "Panel", text_prop: "Caption", w: 185, h: 41,
                event: None, suffix: None, hint: "Panel - Fläche (zieht Inhalt beim Verschieben mit)" },
            WType::Progressbar => Info { cls: "TProgressBar", base: "ProgressBar", text_prop: "", w: 150, h: 17,
                event: None, suffix: None, hint: "ProgressBar - Fortschrittsbalken" },
            WType::Trackbar => Info { cls: "TTrackBar", base: "TrackBar", text_prop: "", w: 150, h: 45,
                event: Some("OnChange"), suffix: Some("Change"), hint: "TrackBar - Schieberegler" },
            WType::Spinedit => Info { cls: "TSpinEdit", base: "SpinEdit", text_prop: "", w: 121, h: 22,
                event: Some("OnChange"), suffix: Some("Change"), hint: "SpinEdit - Zahlenfeld mit Pfeilen" },
            WType::Image => Info { cls: "TImage", base: "Image", text_prop: "", w: 105, h: 105,
                event: None, suffix: None, hint: "Image - Bild (PNG/GIF, mit Pillow auch JPG)" },
            WType::Timer => Info { cls: "TTimer", base: "Timer", text_prop: "", w: 24, h: 24,
                event: Some("OnTimer"), suffix: Some("Timer"), hint: "Timer - ruft in festem Takt eine Funktion auf" },
            WType::Database => Info { cls: "TSQLite", base: "Database", text_prop: "", w: 24, h: 24,
                event: None, suffix: None, hint: "Database - SQLite-Datenbank (Datei wird bei Bedarf angelegt)" },
            WType::Datagrid => Info { cls: "TDataGrid", base: "Grid", text_prop: "Columns", w: 241, h: 129,
                event: Some("OnClick"), suffix: Some("Click"), hint: "Grid - Tabelle (Columns: Spaltenüberschriften, Zeile pro Spalte)" },
        }
    }

    /// Alle Ereignisse; das erste ist das Standard-Ereignis (Feld `handler`).
    pub fn events(self) -> Vec<EventDef> {
        let mut v = Vec::new();
        if let (Some(n), Some(s)) = (self.info().event, self.info().suffix) {
            v.push(ev(n, s));
        }
        match self {
            WType::Entry | WType::Text | WType::Combobox => v.extend(EV_FOCUS_KEY.iter().take(if self == WType::Combobox { 2 } else { 3 })),
            WType::Listbox => {
                v.push(ev("OnDblClick", "DblClick"));
                v.extend(EV_FOCUS_KEY.iter());
            }
            WType::Datagrid => v.push(ev("OnDblClick", "DblClick")),
            _ => {}
        }
        v
    }

    /// Zusaetzliche Eigenschaften (ausser Name/Position/Groesse/Caption/Enabled/Visible).
    pub fn props(self) -> Vec<PropDef> {
        let mut v: Vec<PropDef> = Vec::new();
        match self {
            WType::Label => v.push(pd("alignment", "Alignment", PK::Choice(ALIGN), "taLeftJustify")),
            WType::Entry => {
                v.push(pd("readonly", "ReadOnly", PK::Bool, "False"));
                v.push(pd("password", "PasswordChar", PK::Text, ""));
            }
            WType::Checkbox => v.push(pd("checked", "Checked", PK::Bool, "False")),
            WType::Radiobutton => {
                v.push(pd("checked", "Checked", PK::Bool, "False"));
                v.push(pd("group", "GroupIndex", PK::Int, "0"));
            }
            WType::Listbox => v.push(pd("index", "ItemIndex", PK::Int, "-1")),
            WType::Text => {
                v.push(pd("readonly", "ReadOnly", PK::Bool, "False"));
                v.push(pd("wrap", "WordWrap", PK::Bool, "True"));
            }
            WType::Combobox => {
                v.push(pd("index", "ItemIndex", PK::Int, "-1"));
                v.push(pd("style", "Style", PK::Choice(CBSTYLE), "csDropDownList"));
            }
            WType::Panel => {
                v.push(pd("bevel", "BevelOuter", PK::Choice(BEVEL), "bvRaised"));
                v.push(pd("color", "Color", PK::Color, ""));
            }
            WType::Progressbar => {
                v.push(pd("min", "Min", PK::Int, "0"));
                v.push(pd("max", "Max", PK::Int, "100"));
                v.push(pd("pos", "Position", PK::Int, "0"));
            }
            WType::Trackbar => {
                v.push(pd("min", "Min", PK::Int, "0"));
                v.push(pd("max", "Max", PK::Int, "10"));
                v.push(pd("pos", "Position", PK::Int, "0"));
                v.push(pd("orient", "Orientation", PK::Choice(ORIENT), "trHorizontal"));
            }
            WType::Spinedit => {
                v.push(pd("min", "MinValue", PK::Int, "0"));
                v.push(pd("max", "MaxValue", PK::Int, "100"));
                v.push(pd("pos", "Value", PK::Int, "0"));
            }
            WType::Image => {
                v.push(pd("picture", "Picture", PK::File, ""));
                v.push(pd("stretch", "Stretch", PK::Bool, "True"));
            }
            WType::Timer => v.push(pd("interval", "Interval", PK::Int, "1000")),
            WType::Database => v.push(pd("file", "FileName", PK::Text, "daten.db")),
            _ => {}
        }
        if self.has_font() {
            v.push(P_FSIZE);
            v.push(P_FBOLD);
            v.push(P_FCOLOR);
        }
        if self.is_visual() {
            v.push(P_HINT);
        }
        v
    }
}

fn t() -> bool {
    true
}
fn d_w() -> i32 {
    80
}
fn d_h() -> i32 {
    24
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Widget {
    #[serde(rename = "type")]
    pub wtype: WType,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,
    #[serde(default = "d_w")]
    pub w: i32,
    #[serde(default = "d_h")]
    pub h: i32,
    #[serde(default = "t")]
    pub enabled: bool,
    #[serde(default = "t")]
    pub visible: bool,
    #[serde(default)]
    pub handler: String,
    /// weitere Eigenschaften (nur gesetzte Werte; Standardwerte stehen in `WType::props`)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: BTreeMap<String, String>,
    /// weitere Ereignisse: Ereignisname -> Funktionsname
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub events: BTreeMap<String, String>,
}

impl Widget {
    /// Wert einer Zusatz-Eigenschaft (oder ihr Standardwert).
    pub fn prop(&self, key: &str) -> String {
        if let Some(v) = self.props.get(key) {
            return v.clone();
        }
        self.wtype.props().iter().find(|p| p.key == key).map(|p| p.def.to_string()).unwrap_or_default()
    }
    pub fn prop_bool(&self, key: &str) -> bool {
        self.prop(key) == "True"
    }
    pub fn prop_int(&self, key: &str) -> i32 {
        self.prop(key).trim().parse().unwrap_or(0)
    }
    pub fn set_prop(&mut self, key: &str, value: &str) {
        let default = self.wtype.props().iter().find(|p| p.key == key).map(|p| p.def).unwrap_or("");
        if value == default {
            self.props.remove(key);
        } else {
            self.props.insert(key.to_string(), value.to_string());
        }
    }
    /// Funktionsname fuer ein Ereignis ("" = keiner). Das erste Ereignis liegt in `handler`.
    pub fn event_fn(&self, name: &str) -> String {
        let evs = self.wtype.events();
        if evs.first().map(|e| e.name) == Some(name) {
            self.handler.clone()
        } else {
            self.events.get(name).cloned().unwrap_or_default()
        }
    }
    pub fn set_event_fn(&mut self, name: &str, func: &str) {
        let evs = self.wtype.events();
        if evs.first().map(|e| e.name) == Some(name) {
            self.handler = func.to_string();
        } else if func.is_empty() {
            self.events.remove(name);
        } else {
            self.events.insert(name.to_string(), func.to_string());
        }
    }
    /// Alle belegten Funktionsnamen.
    pub fn all_handlers(&self) -> Vec<String> {
        let mut v = Vec::new();
        if !self.handler.is_empty() {
            v.push(self.handler.clone());
        }
        v.extend(self.events.values().filter(|s| !s.is_empty()).cloned());
        v
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Project {
    pub version: i32,
    pub title: String,
    pub width: i32,
    pub height: i32,
    pub on_create: String,
    #[serde(default)]
    pub on_close: String,
    #[serde(default)]
    pub color: String,
    #[serde(default)]
    pub center: bool,
    pub widgets: Vec<Widget>,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            version: 2,
            title: "Form1".into(),
            width: 400,
            height: 300,
            on_create: String::new(),
            on_close: String::new(),
            color: String::new(),
            center: false,
            widgets: Vec::new(),
        }
    }
}

const PY_KEYWORDS: [&str; 35] = [
    "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue",
    "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in",
    "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
];

pub fn is_py_keyword(s: &str) -> bool {
    PY_KEYWORDS.contains(&s)
}

pub fn valid_name(name: &str) -> bool {
    let mut ch = name.chars();
    match ch.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !is_py_keyword(name)
}

/// Dateiname -> gueltiger Python-Modulname
pub fn safe_stem(stem: &str) -> String {
    let mapped: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
        .collect();
    let mut s = mapped.trim_matches('_').to_string();
    if s.is_empty() {
        s = "fenster".into();
    }
    if s.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        s = format!("ui_{}", s);
    }
    if is_py_keyword(&s) {
        s.push('_');
    }
    s
}

#[allow(dead_code)]
pub fn default_handler(wd: &Widget) -> String {
    match wd.wtype.info().suffix {
        Some(s) => format!("{}{}", wd.name, s),
        None => String::new(),
    }
}

pub fn unique_name(proj: &Project, wt: WType) -> String {
    let base = wt.info().base;
    let mut n = 1;
    loop {
        let cand = format!("{}{}", base, n);
        if !proj.widgets.iter().any(|w| w.name == cand) {
            return cand;
        }
        n += 1;
    }
}

pub fn new_widget(proj: &Project, wt: WType, x: i32, y: i32, w: Option<i32>, h: Option<i32>) -> Widget {
    let info = wt.info();
    let name = unique_name(proj, wt);
    Widget {
        wtype: wt,
        text: if wt.is_list() || !wt.has_text() { String::new() } else { name.clone() },
        name,
        x,
        y,
        w: w.unwrap_or(info.w),
        h: h.unwrap_or(info.h),
        enabled: true,
        visible: true,
        handler: String::new(),
        props: BTreeMap::new(),
        events: BTreeMap::new(),
    }
}

/// Benennt um (wie Delphi: Caption und Standard-Ereignis ziehen mit).
pub fn rename(proj: &mut Project, idx: usize, newname: &str) -> Result<Vec<(String, String)>, String> {
    if proj.widgets[idx].name == newname {
        return Ok(vec![]);
    }
    if !valid_name(newname) {
        return Err("Ungültiger Name. Erlaubt: Buchstaben, Ziffern, _ (nicht mit einer Ziffer beginnen).".into());
    }
    if proj.widgets.iter().enumerate().any(|(i, o)| i != idx && o.name == newname) {
        return Err(format!("Der Name '{}' ist schon vergeben.", newname));
    }
    let wd = &mut proj.widgets[idx];
    let old = wd.name.clone();
    let text_was_name = wd.text == old && wd.wtype.has_text() && !wd.wtype.is_list();
    // Ereignisse mit Standardnamen ziehen mit (Edit1Enter -> Edit2Enter)
    let mut renamed = Vec::new();
    for e in wd.wtype.events() {
        let cur = wd.event_fn(e.name);
        if !cur.is_empty() && cur == format!("{}{}", old, e.suffix) {
            let new = format!("{}{}", newname, e.suffix);
            wd.set_event_fn(e.name, &new);
            renamed.push((cur, new));
        }
    }
    wd.name = newname.to_string();
    if text_was_name {
        wd.text = newname.to_string();
    }
    Ok(renamed)
}

pub fn items_of(wd: &Widget) -> Vec<String> {
    wd.text
        .split('|')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Laedt ein Projekt. Unbekannte Komponenten werden uebersprungen, fehlende Felder aufgefuellt.
pub fn load(path: &Path) -> Result<Project, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let raw: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut p = Project::default();
    if let Some(s) = raw.get("title").and_then(|v| v.as_str()) {
        p.title = s.to_string();
    }
    if let Some(n) = raw.get("width").and_then(|v| v.as_i64()) {
        p.width = n as i32;
    }
    if let Some(n) = raw.get("height").and_then(|v| v.as_i64()) {
        p.height = n as i32;
    }
    if let Some(s) = raw.get("on_create").and_then(|v| v.as_str()) {
        p.on_create = s.to_string();
    }
    if let Some(s) = raw.get("on_close").and_then(|v| v.as_str()) {
        p.on_close = s.to_string();
    }
    if let Some(s) = raw.get("color").and_then(|v| v.as_str()) {
        p.color = s.to_string();
    }
    if let Some(b) = raw.get("center").and_then(|v| v.as_bool()) {
        p.center = b;
    }
    if let Some(list) = raw.get("widgets").and_then(|v| v.as_array()) {
        for w in list {
            if let Ok(wd) = serde_json::from_value::<Widget>(w.clone()) {
                p.widgets.push(wd);
            }
        }
    }
    Ok(p)
}

pub fn save(path: &Path, proj: &Project) -> Result<(), String> {
    let mut s = serde_json::to_string_pretty(proj).map_err(|e| e.to_string())?;
    s.push('\n');
    std::fs::write(path, s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!(valid_name("Button1") && valid_name("_x") && !valid_name("1a") && !valid_name("class"));
        assert_eq!(safe_stem("Mein Fenster"), "Mein_Fenster");
        assert_eq!(safe_stem("1abc"), "ui_1abc");
        assert_eq!(safe_stem("***"), "fenster");
        assert_eq!(safe_stem("class"), "class_");
    }

    #[test]
    fn rename_follows_like_delphi() {
        let mut p = Project::default();
        let mut w = new_widget(&p, WType::Button, 0, 0, None, None);
        w.handler = default_handler(&w);
        p.widgets.push(w);
        assert_eq!(p.widgets[0].name, "Button1");
        rename(&mut p, 0, "btnOK").unwrap();
        assert_eq!(p.widgets[0].handler, "btnOKClick");
        assert_eq!(p.widgets[0].text, "btnOK");
        let w2 = new_widget(&p, WType::Button, 0, 0, None, None);
        p.widgets.push(w2);
        assert!(rename(&mut p, 1, "btnOK").is_err());
        assert!(rename(&mut p, 1, "1x").is_err());
    }
}
