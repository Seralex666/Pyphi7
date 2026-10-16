//! Erzeugt Tkinter- bzw. Qt-Code (Python) und haelt die Logik-Datei am Leben.
//! Port der Python-Version: die Ausgabe ist Zeichen fuer Zeichen identisch.

use crate::model::{self, items_of, Project, WType};
use std::path::{Path, PathBuf};

const TK_HEAD: &str = include_str!("../templates/tk_head.txt");
const TK_TAIL: &str = include_str!("../templates/tk_tail.txt");
const QT_HEAD: &str = include_str!("../templates/qt_head.txt");
const QT_TAIL: &str = include_str!("../templates/qt_tail.txt");
const LOGIC_HEADER: &str = include_str!("../templates/logic_header.txt");
const DB_HUB: &str = include_str!("../templates/db_hub.txt");
const STUB: &str = include_str!("../templates/stub.txt");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Toolkit {
    Tk,
    Qt,
}

impl Toolkit {
    pub fn file_suffix(self) -> &'static str {
        match self {
            Toolkit::Tk => "_tk.py",
            Toolkit::Qt => "_qt.py",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Toolkit::Tk => "Tkinter",
            Toolkit::Qt => "Qt",
        }
    }
}

/// Python-`repr()` fuer Strings (gleiche Quote-Wahl und Escapes wie CPython).
pub fn py_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut o = String::with_capacity(s.len() + 2);
    o.push(quote);
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if c == quote => {
                o.push('\\');
                o.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f || ((c as u32) >= 0x80 && (c as u32) < 0xa0) => {
                o.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => o.push(c),
        }
    }
    o.push(quote);
    o
}

fn py_list(items: &[String]) -> String {
    format!("[{}]", items.iter().map(|s| py_repr(s)).collect::<Vec<_>>().join(", "))
}

fn rep(s: &str, stem: &str, proj: &Project) -> String {
    s.replace("@@DBHUB@@", DB_HUB)
        .replace("@@STEM@@", stem)
        .replace("@@TITLE@@", &py_repr(&proj.title))
        .replace("@@W@@", &proj.width.to_string())
        .replace("@@H@@", &proj.height.to_string())
}

fn b(v: bool) -> &'static str {
    if v { "True" } else { "False" }
}

/// Nur #rgb / #rrggbb zulassen (kommt in Python-Strings, also sauber halten).
fn color_ok(c: &str) -> bool {
    let c = c.trim();
    (c.len() == 4 || c.len() == 7) && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

struct Look {
    size: i32,
    bold: bool,
    color: Option<String>,
}

fn look(wd: &model::Widget) -> Look {
    if !wd.wtype.has_font() {
        return Look { size: 0, bold: false, color: None };
    }
    let c = wd.prop("font_color");
    Look { size: wd.prop_int("font_size").clamp(0, 200), bold: wd.prop_bool("font_bold"), color: color_ok(&c).then(|| c.trim().to_string()) }
}

impl Look {
    fn tk_font(&self) -> Option<String> {
        (self.size > 0 || self.bold).then(|| format!("self._font({}, {})", self.size, b(self.bold)))
    }
}

/// Form-Ereignis- und Kennwerte (alles nach dem Kopf, vor den Komponenten).
fn tk_extra_events(wd: &model::Widget) -> String {
    let mut o = String::new();
    let tgt = if wd.wtype == WType::Datagrid { "_w.tree" } else { "_w" };
    for e in wd.wtype.events().iter().skip(0) {
        let f = wd.event_fn(e.name);
        if f.is_empty() {
            continue;
        }
        let f = py_repr(&f);
        match e.name {
            "OnEnter" => o.push_str(&format!("        _w.bind('<FocusIn>', lambda e: self._call({}), add='+')\n", f)),
            "OnExit" => o.push_str(&format!("        _w.bind('<FocusOut>', lambda e: self._call({}), add='+')\n", f)),
            "OnKeyPress" => o.push_str(&format!("        _w.bind('<KeyPress>', lambda e: self._key(e, {}), add='+')\n", f)),
            "OnDblClick" => o.push_str(&format!("        {}.bind('<Double-Button-1>', lambda e: self._call({}), add='+')\n", tgt, f)),
            _ => {}
        }
    }
    o
}

pub fn gen_tk(proj: &Project, stem: &str) -> String {
    let mut o = rep(TK_HEAD, stem, proj);
    // Klassische Tk-Widgets (Panel, Image, TrackBar) kennen die ttk-Formularfarbe nicht
    let form_bg = if color_ok(&proj.color) { format!(", background={}", py_repr(proj.color.trim())) } else { String::new() };
    if !proj.on_close.is_empty() {
        o.push_str(&format!("        self._on_close_fn = {}\n", py_repr(&proj.on_close)));
    }
    if color_ok(&proj.color) {
        let c = py_repr(proj.color.trim());
        o.push_str(&format!("        self.root.configure(background={})\n", c));
        o.push_str(&format!("        ttk.Style(self.root).configure('.', background={})\n", c));
    }
    for wd in &proj.widgets {
        let (t, n, h, text) = (wd.wtype, &wd.name, &wd.handler, &wd.text);
        let lk = look(wd);
        let font = lk.tk_font();
        o.push_str(&format!("\n        # {} ({})\n", n, t.as_str()));
        if t == WType::Timer {
            o.push_str(&format!(
                "        self._timer({}, {}, {}, {})\n",
                py_repr(n),
                wd.prop_int("interval").max(1),
                b(wd.enabled),
                py_repr(h)
            ));
            continue;
        }
        if t == WType::Database {
            o.push_str(&format!("        self._database({}, {})\n", py_repr(n), py_repr(wd.prop("file").trim())));
            continue;
        }
        let cmd = if h.is_empty() { String::new() } else { format!(", command=lambda: self._call({})", py_repr(h)) };
        // ttk-Stil fuer Schrift/Farbe (Button, Checkbox, Radiobutton)
        let ttk_style = |base: &str| -> String {
            if font.is_none() && lk.color.is_none() {
                return String::new();
            }
            let mut args = String::new();
            if let Some(f) = &font {
                args.push_str(&format!(", font={}", f));
            }
            if let Some(c) = &lk.color {
                args.push_str(&format!(", fg={}", py_repr(c)));
            }
            format!(", style=self._style({}, {}{})", py_repr(n), py_repr(base), args)
        };
        // direkte Widget-Optionen (Label, Entry, Listbox, Text, ...)
        let opts = |fg_key: &str| -> String {
            let mut s = String::new();
            if let Some(f) = &font {
                s.push_str(&format!(", font={}", f));
            }
            if let Some(c) = &lk.color {
                s.push_str(&format!(", {}={}", fg_key, py_repr(c)));
            }
            s
        };
        match t {
            WType::Button => o.push_str(&format!("        _w = ttk.Button(self.root, text={}{}{})\n", py_repr(text), cmd, ttk_style("TButton"))),
            WType::Label => {
                let (anchor, justify) = match wd.prop("alignment").as_str() {
                    "taCenter" => ("n", "center"),
                    "taRightJustify" => ("ne", "right"),
                    _ => ("nw", "left"),
                };
                o.push_str(&format!(
                    "        _w = ttk.Label(self.root, text={}, anchor='{}', justify='{}'{})\n",
                    py_repr(text),
                    anchor,
                    justify,
                    opts("foreground")
                ));
            }
            WType::Entry => {
                o.push_str(&format!("        _sv = tk.StringVar(value={})\n", py_repr(text)));
                o.push_str(&format!("        self._vars[{}] = _sv   # Referenz halten, sonst verfaellt der Trace\n", py_repr(n)));
                let mut extra = String::new();
                if let Some(ch) = wd.prop("password").chars().next() {
                    extra.push_str(&format!(", show={}", py_repr(&ch.to_string())));
                }
                if wd.prop_bool("readonly") {
                    extra.push_str(", state='readonly'");
                }
                o.push_str(&format!("        _w = ttk.Entry(self.root, textvariable=_sv{}{})\n", extra, opts("foreground")));
                if !h.is_empty() {
                    o.push_str(&format!("        _sv.trace_add('write', lambda *_: self._call({}))\n", py_repr(h)));
                }
            }
            WType::Checkbox => {
                o.push_str(&format!("        _v = tk.BooleanVar(value={})\n", b(wd.prop_bool("checked"))));
                o.push_str(&format!("        self._vars[{}] = _v\n", py_repr(n)));
                o.push_str(&format!(
                    "        _w = ttk.Checkbutton(self.root, text={}, variable=_v{}{})\n",
                    py_repr(text),
                    cmd,
                    ttk_style("TCheckbutton")
                ));
            }
            WType::Radiobutton => {
                o.push_str(&format!("        _v = self._group({})\n", wd.prop_int("group")));
                o.push_str(&format!("        self._vars[{}] = _v\n", py_repr(n)));
                o.push_str(&format!(
                    "        _w = ttk.Radiobutton(self.root, text={}, variable=_v, value={}{}{})\n",
                    py_repr(text),
                    py_repr(n),
                    cmd,
                    ttk_style("TRadiobutton")
                ));
                if wd.prop_bool("checked") {
                    o.push_str(&format!("        _v.set({})\n", py_repr(n)));
                }
            }
            WType::Listbox => {
                o.push_str(&format!("        _w = tk.Listbox(self.root, exportselection=False{})\n", opts("fg")));
                o.push_str(&format!("        for _i in {}:\n            _w.insert('end', _i)\n", py_list(&items_of(wd))));
                let idx = wd.prop_int("index");
                if idx >= 0 && (idx as usize) < items_of(wd).len() {
                    o.push_str(&format!("        _w.selection_set({})\n", idx));
                }
                if !h.is_empty() {
                    o.push_str(&format!("        _w.bind('<<ListboxSelect>>', lambda e: self._call({}))\n", py_repr(h)));
                }
            }
            WType::Text => {
                let wrap = if wd.prop_bool("wrap") { "word" } else { "none" };
                o.push_str(&format!("        _w = tk.Text(self.root, wrap='{}'{})\n", wrap, opts("fg")));
                if !text.is_empty() {
                    o.push_str(&format!("        _w.insert('1.0', {})\n", py_repr(text)));
                }
                if !h.is_empty() {
                    o.push_str("        _w.edit_modified(False)\n");
                    o.push_str(&format!("        _w.bind('<<Modified>>', lambda e: self._modified(e.widget, {}))\n", py_repr(h)));
                }
                if wd.prop_bool("readonly") {
                    o.push_str(&format!("        self._ro.add({})\n", py_repr(n)));
                    o.push_str("        _w.configure(state='disabled')\n");
                }
            }
            WType::Combobox => {
                let editable = wd.prop("style") == "csDropDown";
                let mut extra = String::new();
                if editable && !h.is_empty() {
                    o.push_str("        _sv = tk.StringVar()\n");
                    o.push_str(&format!("        self._vars[{}] = _sv\n", py_repr(n)));
                    extra.push_str(", textvariable=_sv");
                }
                o.push_str(&format!(
                    "        _w = ttk.Combobox(self.root, values={}, state='{}'{}{})\n",
                    py_list(&items_of(wd)),
                    if editable { "normal" } else { "readonly" },
                    extra,
                    opts("foreground")
                ));
                let idx = wd.prop_int("index");
                if idx >= 0 && (idx as usize) < items_of(wd).len() {
                    o.push_str(&format!("        _w.current({})\n", idx));
                }
                if !h.is_empty() {
                    if editable {
                        o.push_str(&format!("        _sv.trace_add('write', lambda *_: self._call({}))\n", py_repr(h)));
                    } else {
                        o.push_str(&format!("        _w.bind('<<ComboboxSelected>>', lambda e: self._call({}))\n", py_repr(h)));
                    }
                }
            }
            WType::Groupbox => {
                let mut style = String::new();
                if font.is_some() || lk.color.is_some() {
                    let mut args = String::new();
                    if let Some(f) = &font {
                        args.push_str(&format!(", font={}", f));
                    }
                    if let Some(c) = &lk.color {
                        args.push_str(&format!(", fg={}", py_repr(c)));
                    }
                    o.push_str(&format!("        self._style({}, 'TLabelframe.Label'{})\n", py_repr(n), args));
                    style = format!(", style={}", py_repr(&format!("{}.TLabelframe", n)));
                }
                o.push_str(&format!("        _w = ttk.LabelFrame(self.root, text={}{})\n", py_repr(text), style));
            }
            WType::Panel => {
                let relief = match wd.prop("bevel").as_str() {
                    "bvNone" => "flat', borderwidth=0",
                    "bvLowered" => "sunken', borderwidth=2",
                    _ => "raised', borderwidth=2",
                };
                let mut extra = opts("fg");
                let c = wd.prop("color");
                if color_ok(&c) {
                    extra.push_str(&format!(", background={}", py_repr(c.trim())));
                } else {
                    extra.push_str(&form_bg);
                }
                o.push_str(&format!("        _w = tk.Label(self.root, text={}, relief='{}{})\n", py_repr(text), relief, extra));
            }
            WType::Progressbar => {
                let (mn, mx) = (wd.prop_int("min"), wd.prop_int("max").max(wd.prop_int("min") + 1));
                let pos = wd.prop_int("pos").clamp(mn, mx);
                o.push_str(&format!("        self._min[{}] = {}\n", py_repr(n), mn));
                o.push_str(&format!(
                    "        _w = ttk.Progressbar(self.root, orient='horizontal', maximum={}, value={})\n",
                    mx - mn,
                    pos - mn
                ));
            }
            WType::Trackbar => {
                let (mn, mx) = (wd.prop_int("min"), wd.prop_int("max").max(wd.prop_int("min") + 1));
                let orient = if wd.prop("orient") == "trVertical" { "vertical" } else { "horizontal" };
                o.push_str(&format!(
                    "        _w = tk.Scale(self.root, from_={}, to={}, orient='{}', showvalue=False, resolution=1, highlightthickness=0{})\n",
                    mn, mx, orient, form_bg
                ));
                o.push_str(&format!("        _w.set({})\n", wd.prop_int("pos").clamp(mn, mx)));
                if !h.is_empty() {
                    o.push_str(&format!("        _w.configure(command=lambda v: self._call({}))\n", py_repr(h)));
                }
            }
            WType::Spinedit => {
                let (mn, mx) = (wd.prop_int("min"), wd.prop_int("max").max(wd.prop_int("min")));
                o.push_str(&format!("        _sv = tk.StringVar(value=str({}))\n", wd.prop_int("pos").clamp(mn, mx)));
                o.push_str(&format!("        self._vars[{}] = _sv\n", py_repr(n)));
                o.push_str(&format!(
                    "        _w = ttk.Spinbox(self.root, from_={}, to={}, textvariable=_sv{})\n",
                    mn,
                    mx,
                    opts("foreground")
                ));
                if !h.is_empty() {
                    o.push_str(&format!("        _sv.trace_add('write', lambda *_: self._call({}))\n", py_repr(h)));
                }
            }
            WType::Image => o.push_str(&format!("        _w = tk.Label(self.root, borderwidth=0{})\n", form_bg)),
            WType::Datagrid => {
                o.push_str(&format!("        _w = _Grid(self.root, {})\n", py_list(&items_of(wd))));
                if !h.is_empty() {
                    o.push_str(&format!(
                        "        _w.tree.bind('<<TreeviewSelect>>', lambda e, g=_w: None if g.silent else self._call({}))\n",
                        py_repr(h)
                    ));
                }
            }
            WType::Timer | WType::Database => unreachable!(),
        }
        o.push_str(&tk_extra_events(wd));
        let hint = wd.prop("hint");
        if !hint.is_empty() {
            o.push_str(&format!("        _Tip(_w, {})\n", py_repr(&hint)));
        }
        o.push_str(&format!(
            "        self._add({}, {}, _w, {}, {}, {}, {})\n",
            py_repr(n),
            py_repr(t.as_str()),
            wd.x,
            wd.y,
            wd.w,
            wd.h
        ));
        if t == WType::Image {
            o.push_str(&format!("        self._stretch[{}] = {}\n", py_repr(n), b(wd.prop_bool("stretch"))));
            let pic = wd.prop("picture");
            if !pic.is_empty() {
                o.push_str(&format!("        self.set_image({}, {})\n", py_repr(n), py_repr(&pic)));
            }
        }
        if !wd.enabled {
            o.push_str(&format!("        self.set_enabled({}, False)\n", py_repr(n)));
        }
        if !wd.visible {
            o.push_str(&format!("        self.set_visible({}, False)\n", py_repr(n)));
        }
    }
    if proj.center {
        o.push_str(&format!(
            "\n        self.root.update_idletasks()\n        self.root.geometry(\"{w}x{h}+%d+%d\" % ((self.root.winfo_screenwidth() - {w}) // 2, (self.root.winfo_screenheight() - {h}) // 2))\n",
            w = proj.width,
            h = proj.height
        ));
    }
    if !proj.on_create.is_empty() {
        o.push_str(&format!("\n        self._call({})\n", py_repr(&proj.on_create)));
    }
    o.push_str(&rep(TK_TAIL, stem, proj));
    o
}

fn qt_style_sheet(parts: &[String]) -> Option<String> {
    (!parts.is_empty()).then(|| parts.join(" "))
}

pub fn gen_qt(proj: &Project, stem: &str) -> String {
    let mut o = rep(QT_HEAD, stem, proj);
    if !proj.on_close.is_empty() {
        o.push_str(&format!("        self._on_close_fn = {}\n", py_repr(&proj.on_close)));
    }
    if color_ok(&proj.color) {
        o.push_str(&format!(
            "        _pal = self.palette()\n        _pal.setColor(self.backgroundRole(), QColor({}))\n        self.setPalette(_pal)\n        self.setAutoFillBackground(True)\n",
            py_repr(proj.color.trim())
        ));
    }
    for wd in &proj.widgets {
        let (t, n, h, text) = (wd.wtype, &wd.name, &wd.handler, &wd.text);
        let lk = look(wd);
        o.push_str(&format!("\n        # {} ({})\n", n, t.as_str()));
        if t == WType::Timer {
            o.push_str(&format!(
                "        self._timer({}, {}, {}, {})\n",
                py_repr(n),
                wd.prop_int("interval").max(1),
                b(wd.enabled),
                py_repr(h)
            ));
            continue;
        }
        if t == WType::Database {
            o.push_str(&format!("        self._database({}, {})\n", py_repr(n), py_repr(wd.prop("file").trim())));
            continue;
        }
        let call = format!("lambda *_: self._call({})", py_repr(h));
        let connect = |sig: &str| -> String {
            if h.is_empty() {
                String::new()
            } else {
                format!("        _w.{}.connect({})\n", sig, call)
            }
        };
        let mut sheet: Vec<String> = Vec::new();
        if let Some(c) = &lk.color {
            sheet.push(format!("color: {};", c));
        }
        match t {
            WType::Button => {
                o.push_str(&format!("        _w = QPushButton({}, self)\n", py_repr(text)));
                o.push_str(&connect("clicked"));
            }
            WType::Label => {
                o.push_str(&format!("        _w = QLabel({}, self)\n", py_repr(text)));
                let h_al = match wd.prop("alignment").as_str() {
                    "taCenter" => "AlignHCenter",
                    "taRightJustify" => "AlignRight",
                    _ => "AlignLeft",
                };
                o.push_str(&format!("        _w.setAlignment(Qt.AlignmentFlag.{} | Qt.AlignmentFlag.AlignTop)\n", h_al));
            }
            WType::Entry => {
                o.push_str(&format!("        _w = QLineEdit({}, self)\n", py_repr(text)));
                if let Some(ch) = wd.prop("password").chars().next() {
                    let _ = ch;
                    o.push_str("        _w.setEchoMode(QLineEdit.EchoMode.Password)\n");
                }
                if wd.prop_bool("readonly") {
                    o.push_str("        _w.setReadOnly(True)\n");
                }
                o.push_str(&connect("textChanged"));
            }
            WType::Checkbox => {
                o.push_str(&format!("        _w = QCheckBox({}, self)\n", py_repr(text)));
                if wd.prop_bool("checked") {
                    o.push_str("        _w.setChecked(True)\n");
                }
                o.push_str(&connect("toggled"));
            }
            WType::Radiobutton => {
                o.push_str(&format!("        _w = QRadioButton({}, self)\n", py_repr(text)));
                o.push_str(&format!("        self._group({}).addButton(_w)\n", wd.prop_int("group")));
                if wd.prop_bool("checked") {
                    o.push_str("        _w.setChecked(True)\n");
                }
                o.push_str(&connect("clicked"));
            }
            WType::Listbox => {
                o.push_str("        _w = QListWidget(self)\n");
                o.push_str(&format!("        _w.addItems({})\n", py_list(&items_of(wd))));
                let idx = wd.prop_int("index");
                if idx >= 0 && (idx as usize) < items_of(wd).len() {
                    o.push_str(&format!("        _w.setCurrentRow({})\n", idx));
                }
                o.push_str(&connect("itemSelectionChanged"));
                let f = wd.event_fn("OnDblClick");
                if !f.is_empty() {
                    o.push_str(&format!("        _w.itemDoubleClicked.connect(lambda *_: self._call({}))\n", py_repr(&f)));
                }
            }
            WType::Text => {
                o.push_str(&format!("        _w = QPlainTextEdit({}, self)\n", py_repr(text)));
                if !wd.prop_bool("wrap") {
                    o.push_str("        _w.setLineWrapMode(QPlainTextEdit.LineWrapMode.NoWrap)\n");
                }
                if wd.prop_bool("readonly") {
                    o.push_str("        _w.setReadOnly(True)\n");
                }
                o.push_str(&connect("textChanged"));
            }
            WType::Combobox => {
                let editable = wd.prop("style") == "csDropDown";
                o.push_str("        _w = QComboBox(self)\n");
                if editable {
                    o.push_str("        _w.setEditable(True)\n");
                }
                o.push_str(&format!("        _w.addItems({})\n", py_list(&items_of(wd))));
                let idx = wd.prop_int("index");
                let idx = if idx >= 0 && (idx as usize) < items_of(wd).len() { idx } else { -1 };
                o.push_str(&format!("        _w.setCurrentIndex({})\n", idx));
                o.push_str(&connect(if editable { "currentTextChanged" } else { "currentIndexChanged" }));
            }
            WType::Groupbox => o.push_str(&format!("        _w = QGroupBox({}, self)\n", py_repr(text))),
            WType::Panel => {
                o.push_str(&format!("        _w = QLabel({}, self)\n", py_repr(text)));
                let shape = match wd.prop("bevel").as_str() {
                    "bvNone" => "QFrame.Shape.NoFrame".to_string(),
                    "bvLowered" => "QFrame.Shape.Panel | QFrame.Shadow.Sunken".to_string(),
                    _ => "QFrame.Shape.Panel | QFrame.Shadow.Raised".to_string(),
                };
                o.push_str(&format!("        _w.setFrameStyle({})\n", shape));
                o.push_str("        _w.setLineWidth(2)\n        _w.setAlignment(Qt.AlignmentFlag.AlignCenter)\n");
                let c = wd.prop("color");
                if color_ok(&c) {
                    sheet.push(format!("background-color: {};", c.trim()));
                }
            }
            WType::Progressbar => {
                let (mn, mx) = (wd.prop_int("min"), wd.prop_int("max").max(wd.prop_int("min") + 1));
                o.push_str("        _w = QProgressBar(self)\n");
                o.push_str(&format!("        _w.setRange({}, {})\n", mn, mx));
                o.push_str(&format!("        _w.setValue({})\n", wd.prop_int("pos").clamp(mn, mx)));
                o.push_str("        _w.setTextVisible(False)\n");
            }
            WType::Trackbar => {
                let (mn, mx) = (wd.prop_int("min"), wd.prop_int("max").max(wd.prop_int("min") + 1));
                let orient = if wd.prop("orient") == "trVertical" { "Vertical" } else { "Horizontal" };
                o.push_str(&format!("        _w = QSlider(Qt.Orientation.{}, self)\n", orient));
                o.push_str(&format!("        _w.setRange({}, {})\n", mn, mx));
                o.push_str(&format!("        _w.setValue({})\n", wd.prop_int("pos").clamp(mn, mx)));
                o.push_str(&connect("valueChanged"));
            }
            WType::Spinedit => {
                let (mn, mx) = (wd.prop_int("min"), wd.prop_int("max").max(wd.prop_int("min")));
                o.push_str("        _w = QSpinBox(self)\n");
                o.push_str(&format!("        _w.setRange({}, {})\n", mn, mx));
                o.push_str(&format!("        _w.setValue({})\n", wd.prop_int("pos").clamp(mn, mx)));
                o.push_str(&connect("valueChanged"));
            }
            WType::Image => {
                o.push_str("        _w = QLabel(self)\n");
                o.push_str(&format!("        _w.setScaledContents({})\n", b(wd.prop_bool("stretch"))));
                o.push_str("        _w.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop)\n");
            }
            WType::Datagrid => {
                let cols = items_of(wd);
                o.push_str("        _w = QTableWidget(self)\n");
                o.push_str(&format!("        _w.setColumnCount({})\n", cols.len()));
                o.push_str(&format!("        _w.setHorizontalHeaderLabels({})\n", py_list(&cols)));
                o.push_str("        _w.setEditTriggers(QAbstractItemView.EditTrigger.NoEditTriggers)\n");
                o.push_str("        _w.setSelectionBehavior(QAbstractItemView.SelectionBehavior.SelectRows)\n");
                o.push_str("        _w.setSelectionMode(QAbstractItemView.SelectionMode.SingleSelection)\n");
                o.push_str("        _w.verticalHeader().setVisible(False)\n");
                o.push_str("        _w.horizontalHeader().setSectionResizeMode(QHeaderView.ResizeMode.Stretch)\n");
                o.push_str(&connect("itemSelectionChanged"));
                let f = wd.event_fn("OnDblClick");
                if !f.is_empty() {
                    o.push_str(&format!("        _w.cellDoubleClicked.connect(lambda *_: self._call({}))\n", py_repr(&f)));
                }
            }
            WType::Timer | WType::Database => unreachable!(),
        }
        if lk.size > 0 || lk.bold {
            o.push_str("        _f = _w.font()\n");
            if lk.size > 0 {
                o.push_str(&format!("        _f.setPointSize({})\n", lk.size));
            }
            if lk.bold {
                o.push_str("        _f.setBold(True)\n");
            }
            o.push_str("        _w.setFont(_f)\n");
        }
        if let Some(sh) = qt_style_sheet(&sheet) {
            o.push_str(&format!("        _w.setStyleSheet({})\n", py_repr(&sh)));
        }
        // Fokus-/Tasten-Ereignisse
        let mut spec = Vec::new();
        for (ev, key) in [("OnEnter", "enter"), ("OnExit", "exit"), ("OnKeyPress", "key")] {
            let f = wd.event_fn(ev);
            if !f.is_empty() {
                spec.push(format!("{}={}", key, py_repr(&f)));
            }
        }
        if !spec.is_empty() {
            o.push_str(&format!("        self._watch(_w, {})\n", spec.join(", ")));
        }
        let hint = wd.prop("hint");
        if !hint.is_empty() {
            o.push_str(&format!("        _w.setToolTip({})\n", py_repr(&hint)));
        }
        o.push_str(&format!(
            "        self._add({}, {}, _w, {}, {}, {}, {})\n",
            py_repr(n),
            py_repr(t.as_str()),
            wd.x,
            wd.y,
            wd.w,
            wd.h
        ));
        if t == WType::Image {
            let pic = wd.prop("picture");
            if !pic.is_empty() {
                o.push_str(&format!("        self.set_image({}, {})\n", py_repr(n), py_repr(&pic)));
            }
        }
        if !wd.enabled {
            o.push_str(&format!("        self.set_enabled({}, False)\n", py_repr(n)));
        }
        if !wd.visible {
            o.push_str(&format!("        self.set_visible({}, False)\n", py_repr(n)));
        }
    }
    if proj.center {
        o.push_str("\n        _scr = self._app.primaryScreen().availableGeometry()\n        self.move(_scr.center() - self.rect().center())\n");
    }
    if !proj.on_create.is_empty() {
        o.push_str(&format!("\n        self._call({})\n", py_repr(&proj.on_create)));
    }
    o.push_str(&rep(QT_TAIL, stem, proj));
    o
}

// ---------------------------------------------------------------- Logik-Datei

pub fn logic_path(folder: &Path, stem: &str) -> PathBuf {
    folder.join(format!("{}_logic.py", stem))
}

pub fn header(stem: Option<&str>) -> String {
    LOGIC_HEADER.replace("@@STEM@@", stem.unwrap_or("dein Projekt"))
}

pub fn stub_text(name: &str) -> String {
    if name.ends_with("Click") {
        STUB.replace("{name}", name)
    } else {
        // Ereignisse, die oft/laufend feuern (Timer, Change, KeyPress ...): leere Funktion wie in Delphi
        format!("\n\ndef {}(ui):\n    pass\n", name)
    }
}

/// Byte-Bereich des Namens, falls die Zeile `def <name>(` ist.
pub fn def_name_range(line: &str, name: &str) -> Option<(usize, usize)> {
    let rest = line.strip_prefix("def")?;
    let trimmed = rest.trim_start();
    if trimmed.len() == rest.len() {
        return None; // 'def' ohne Leerraum danach
    }
    let after = trimmed.strip_prefix(name)?;
    if !after.trim_start().starts_with('(') {
        return None;
    }
    let start = line.len() - trimmed.len();
    Some((start, start + name.len()))
}

/// Gibt es `def name(` am Zeilenanfang?
pub fn has_def(text: &str, name: &str) -> bool {
    text.lines().any(|l| def_name_range(l, name).is_some())
}

/// Alle Funktionsnamen, die das Projekt als Ereignis-Funktion erwartet.
pub fn handler_names(proj: &Project) -> Vec<String> {
    let mut v: Vec<String> = proj.widgets.iter().flat_map(|w| w.all_handlers()).collect();
    if !proj.on_create.is_empty() {
        v.push(proj.on_create.clone());
    }
    if !proj.on_close.is_empty() {
        v.push(proj.on_close.clone());
    }
    let mut seen = std::collections::HashSet::new();
    v.retain(|n| seen.insert(n.clone()));
    v
}

/// Legt die Logik-Datei an bzw. ergaenzt fehlende Funktionen. -> neu angelegte Namen
pub fn ensure_logic(proj: &Project, folder: &Path, stem: &str) -> std::io::Result<Vec<String>> {
    let path = logic_path(folder, stem);
    let existed = path.exists();
    let mut text = if existed { std::fs::read_to_string(&path)? } else { header(Some(stem)) };
    let mut added = Vec::new();
    for name in handler_names(proj) {
        if !has_def(&text, &name) {
            text = format!("{}\n{}", text.trim_end_matches('\n'), stub_text(&name));
            added.push(name);
        }
    }
    if !existed || !added.is_empty() {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        std::fs::write(&path, text)?;
    }
    Ok(added)
}

/// Schreibt die erzeugten Dateien. -> (Dateinamen, neue Funktionen)
pub fn generate(
    proj: &Project,
    folder: &Path,
    stem: &str,
    toolkits: &[Toolkit],
) -> std::io::Result<(Vec<String>, Vec<String>)> {
    let stem = model::safe_stem(stem);
    let mut written = Vec::new();
    for tk in toolkits {
        let name = format!("{}{}", stem, tk.file_suffix());
        let code = match tk {
            Toolkit::Tk => gen_tk(proj, &stem),
            Toolkit::Qt => gen_qt(proj, &stem),
        };
        std::fs::write(folder.join(&name), code)?;
        written.push(name);
    }
    let added = ensure_logic(proj, folder, &stem)?;
    written.push(format!("{}_logic.py", stem));
    Ok((written, added))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repr_like_python() {
        assert_eq!(py_repr("abc"), "'abc'");
        assert_eq!(py_repr("it's"), "\"it's\"");
        assert_eq!(py_repr("say \"hi\" it's"), "'say \"hi\" it\\'s'");
        assert_eq!(py_repr("a\\b\nc"), "'a\\\\b\\nc'");
        assert_eq!(py_repr("Begrüßen"), "'Begrüßen'");
        assert_eq!(py_list(&["a".into(), "b".into()]), "['a', 'b']");
        assert_eq!(py_list(&[]), "[]");
    }

    #[test]
    fn has_def_works() {
        let t = "x = 1\n\ndef Foo(ui):\n    pass\n\ndef  Bar (ui):\n    pass\n    def Inner(x): pass\n";
        assert!(has_def(t, "Foo") && has_def(t, "Bar"));
        assert!(!has_def(t, "Inner") && !has_def(t, "Fo") && !has_def(t, "Baz"));
        assert!(!has_def("define Foo(", "Foo"));
    }

    #[test]
    fn stub_matches_python_layout() {
        assert_eq!(
            stub_text("Button1Click"),
            "\n\ndef Button1Click(ui):\n    ui.message(\"Button1Click wurde aufgerufen - hier deinen Code einfuegen\")\n"
        );
        assert_eq!(stub_text("Timer1Timer"), "\n\ndef Timer1Timer(ui):\n    pass\n");
    }
}
