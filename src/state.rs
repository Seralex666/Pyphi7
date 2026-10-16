//! Zustand und Verhalten des Designers - ohne jede GUI-Abhaengigkeit (gut testbar).
//! Entspricht 1:1 der Logik der Python-Version.

use crate::codegen::{self, Toolkit};
use crate::model::{self, Project, WType, Widget};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const GRID: i32 = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    Form,
    Code,
    Libs,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogKind {
    Normal,
    Info,
    Err,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Handle {
    NW,
    N,
    NE,
    E,
    SE,
    S,
    SW,
    W,
}

impl Handle {
    fn west(self) -> bool {
        matches!(self, Handle::NW | Handle::W | Handle::SW)
    }
    fn east(self) -> bool {
        matches!(self, Handle::NE | Handle::E | Handle::SE)
    }
    fn north(self) -> bool {
        matches!(self, Handle::NW | Handle::N | Handle::NE)
    }
    fn south(self) -> bool {
        matches!(self, Handle::SW | Handle::S | Handle::SE)
    }
}

/// Anfasser am Formular selbst (Groesse mit der Maus aendern).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FormEdge {
    E,
    S,
    SE,
}

pub fn handle_points(wd: &Widget) -> [(Handle, (i32, i32)); 8] {
    let (x, y, w, h) = (wd.x, wd.y, wd.w, wd.h);
    let (xm, ym, xr, yb) = (x + w / 2, y + h / 2, x + w, y + h);
    [
        (Handle::NW, (x, y)),
        (Handle::N, (xm, y)),
        (Handle::NE, (xr, y)),
        (Handle::E, (xr, ym)),
        (Handle::SE, (xr, yb)),
        (Handle::S, (xm, yb)),
        (Handle::SW, (x, yb)),
        (Handle::W, (x, ym)),
    ]
}

#[derive(Clone)]
pub struct Snapshot {
    proj: Project,
    sel: Option<usize>,
}

#[derive(Clone)]
pub enum Mode {
    None,
    Create { wt: WType, sx: i32, sy: i32, ex: i32, ey: i32, moved: bool },
    Move { fx: f32, fy: f32, snap: Snapshot, ox: i32, oy: i32, kids: Vec<(usize, i32, i32)>, pushed: bool },
    FormResize { edge: FormEdge, fx: f32, fy: f32, snap: Snapshot, ow: i32, oh: i32, pushed: bool },
    Resize { h: Handle, fx: f32, fy: f32, snap: Snapshot, o: (i32, i32, i32, i32), pushed: bool },
}

pub type PropKind = model::PK;

pub struct PropRow {
    pub key: String,
    pub label: String,
    pub value: String,
    pub kind: PropKind,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogicPolicy {
    Ask,
    Overwrite,
    LoadExisting,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SaveResult {
    Saved,
    /// Logik-Datei existiert bereits bzw. wurde extern geaendert -> Nachfrage noetig
    Conflict,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GenResult {
    Done(Vec<String>),
    Conflict,
}

pub struct State {
    pub proj: Project,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub sel: Option<usize>,
    pub undo: Vec<Snapshot>,
    pub redo: Vec<Snapshot>,
    pub tool: Option<WType>,
    pub snap: bool,
    pub show_grid: bool,
    pub mode: Mode,
    pub view: View,
    pub logic: String,
    pub logic_dirty: bool,
    pub logic_mtime: Option<SystemTime>,
    pub toolkit_choice: usize, // 0 = Tkinter, 1 = Qt, 2 = Beide
    pub log: Vec<(String, LogKind)>,
    /// Wunsch an die UI: Cursor im Editor auf dieses Zeichen setzen und hinscrollen.
    pub goto_char: Option<usize>,
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl State {
    pub fn new() -> Self {
        State {
            proj: Project::default(),
            path: None,
            dirty: false,
            sel: None,
            undo: Vec::new(),
            redo: Vec::new(),
            tool: None,
            snap: true,
            show_grid: true,
            mode: Mode::None,
            view: View::Form,
            logic: codegen::header(None),
            logic_dirty: false,
            logic_mtime: None,
            toolkit_choice: 0,
            log: Vec::new(),
            goto_char: None,
        }
    }

    // ------------------------------------------------------------ Hilfen
    pub fn stem(&self) -> Option<String> {
        self.path
            .as_ref()
            .and_then(|p| p.file_stem())
            .and_then(|s| s.to_str())
            .map(model::safe_stem)
    }

    pub fn folder(&self) -> Option<PathBuf> {
        self.path.as_ref().map(|p| {
            let f = p.parent().map(|x| x.to_path_buf()).unwrap_or_default();
            if f.as_os_str().is_empty() { PathBuf::from(".") } else { f }
        })
    }

    pub fn logic_file(&self) -> Option<PathBuf> {
        Some(codegen::logic_path(&self.folder()?, &self.stem()?))
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty || self.logic_dirty
    }

    pub fn log(&mut self, text: impl Into<String>, kind: LogKind) {
        self.log.push((text.into(), kind));
        if self.log.len() > 2000 {
            self.log.drain(0..500);
        }
    }

    pub fn toolkits(&self) -> Vec<Toolkit> {
        match self.toolkit_choice {
            0 => vec![Toolkit::Tk],
            1 => vec![Toolkit::Qt],
            _ => vec![Toolkit::Tk, Toolkit::Qt],
        }
    }

    pub fn snap_v(&self, v: f32) -> i32 {
        if self.snap {
            ((v / GRID as f32).round() as i32) * GRID
        } else {
            v as i32
        }
    }

    pub fn touch(&mut self) {
        self.dirty = true;
    }

    pub fn selected(&self) -> Option<&Widget> {
        self.sel.and_then(|i| self.proj.widgets.get(i))
    }

    // ------------------------------------------------------------ Undo
    pub fn snapshot(&self) -> Snapshot {
        Snapshot { proj: self.proj.clone(), sel: self.sel }
    }

    fn push(&mut self, snap: Snapshot) {
        self.undo.push(snap);
        if self.undo.len() > 100 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn restore(&mut self, snap: Snapshot) {
        self.proj = snap.proj;
        self.sel = snap.sel.filter(|&i| i < self.proj.widgets.len());
    }

    pub fn cmd_undo(&mut self) {
        if let Some(s) = self.undo.pop() {
            let cur = self.snapshot();
            self.redo.push(cur);
            self.restore(s);
            self.touch();
        }
    }

    pub fn cmd_redo(&mut self) {
        if let Some(s) = self.redo.pop() {
            let cur = self.snapshot();
            self.undo.push(cur);
            self.restore(s);
            self.touch();
        }
    }

    // ------------------------------------------------------------ Treffer
    pub fn hit_widget(&self, fx: f32, fy: f32) -> Option<usize> {
        self.proj.widgets.iter().enumerate().rev().find_map(|(i, w)| {
            let inside = fx >= w.x as f32 && fx < (w.x + w.w) as f32 && fy >= w.y as f32 && fy < (w.y + w.h) as f32;
            inside.then_some(i)
        })
    }

    /// Rand/Ecke des Formulars unter dem Mauszeiger (zum Vergroessern).
    pub fn hit_form_edge(&self, fx: f32, fy: f32) -> Option<FormEdge> {
        let (w, h) = (self.proj.width as f32, self.proj.height as f32);
        let reach = 10.0;
        let right = fx >= w && fx < w + reach;
        let bottom = fy >= h && fy < h + reach;
        if right && bottom {
            Some(FormEdge::SE)
        } else if right && fy >= 0.0 && fy < h {
            Some(FormEdge::E)
        } else if bottom && fx >= 0.0 && fx < w {
            Some(FormEdge::S)
        } else {
            None
        }
    }

    pub fn hit_handle(&self, fx: f32, fy: f32) -> Option<Handle> {
        let wd = self.selected()?;
        if !wd.wtype.is_visual() {
            return None;
        }
        handle_points(wd)
            .iter()
            .find(|(_, (px, py))| (fx - *px as f32).abs() <= 4.0 && (fy - *py as f32).abs() <= 4.0)
            .map(|(h, _)| *h)
    }

    pub fn select(&mut self, sel: Option<usize>) {
        self.sel = sel;
    }

    // ------------------------------------------------------------ Maus
    pub fn press(&mut self, fx: f32, fy: f32) {
        if let Some(wt) = self.tool {
            if fx < 0.0 || fy < 0.0 || fx >= self.proj.width as f32 || fy >= self.proj.height as f32 {
                self.mode = Mode::None;
                return;
            }
            let sx = self.snap_v(fx.max(0.0));
            let sy = self.snap_v(fy.max(0.0));
            self.mode = Mode::Create { wt, sx, sy, ex: sx, ey: sy, moved: false };
            return;
        }
        if let Some(h) = self.hit_handle(fx, fy) {
            let wd = self.selected().unwrap();
            let o = (wd.x, wd.y, wd.w, wd.h);
            self.mode = Mode::Resize { h, fx, fy, snap: self.snapshot(), o, pushed: false };
            return;
        }
        if self.hit_widget(fx, fy).is_none() {
            if let Some(edge) = self.hit_form_edge(fx, fy) {
                self.select(None);
                self.mode = Mode::FormResize { edge, fx, fy, snap: self.snapshot(), ow: self.proj.width, oh: self.proj.height, pushed: false };
                return;
            }
        }
        let hit = self.hit_widget(fx, fy);
        self.select(hit);
        self.mode = match hit {
            Some(i) => {
                let w = &self.proj.widgets[i];
                // Container nehmen alles mit, was komplett in ihnen liegt
                let kids: Vec<(usize, i32, i32)> = if w.wtype.is_container() {
                    self.proj
                        .widgets
                        .iter()
                        .enumerate()
                        .filter(|(j, k)| *j != i && k.x >= w.x && k.y >= w.y && k.x + k.w <= w.x + w.w && k.y + k.h <= w.y + w.h)
                        .map(|(j, k)| (j, k.x, k.y))
                        .collect()
                } else {
                    Vec::new()
                };
                Mode::Move { fx, fy, snap: self.snapshot(), ox: w.x, oy: w.y, kids, pushed: false }
            }
            None => Mode::None,
        };
    }

    pub fn drag(&mut self, fx: f32, fy: f32) {
        let (fw, fh) = (self.proj.width, self.proj.height);
        let snap_on = self.snap;
        let sn = |v: f32| -> i32 {
            if snap_on { ((v / GRID as f32).round() as i32) * GRID } else { v as i32 }
        };
        let mode = std::mem::replace(&mut self.mode, Mode::None);
        match mode {
            Mode::Create { wt, sx, sy, ex: _, ey: _, mut moved } => {
                let ex = sn(fx.clamp(0.0, fw as f32));
                let ey = sn(fy.clamp(0.0, fh as f32));
                if (fx - sx as f32).abs() > 4.0 || (fy - sy as f32).abs() > 4.0 {
                    moved = true;
                }
                self.mode = Mode::Create { wt, sx, sy, ex, ey, moved };
            }
            Mode::Move { fx: px, fy: py, snap, ox, oy, kids, mut pushed } => {
                let (dx, dy) = (fx - px, fy - py);
                if !pushed && (dx.abs() > 1.0 || dy.abs() > 1.0) {
                    self.push(snap.clone());
                    pushed = true;
                    self.touch();
                }
                if let Some(i) = self.sel {
                    let w = &mut self.proj.widgets[i];
                    w.x = sn(ox as f32 + dx).min((fw - w.w).max(0)).max(0);
                    w.y = sn(oy as f32 + dy).min((fh - w.h).max(0)).max(0);
                    let (ddx, ddy) = (w.x - ox, w.y - oy);
                    for &(j, kx, ky) in &kids {
                        if let Some(k) = self.proj.widgets.get_mut(j) {
                            k.x = kx + ddx;
                            k.y = ky + ddy;
                        }
                    }
                }
                self.mode = Mode::Move { fx: px, fy: py, snap, ox, oy, kids, pushed };
            }
            Mode::FormResize { edge, fx: px, fy: py, snap, ow, oh, mut pushed } => {
                let (dx, dy) = (fx - px, fy - py);
                if !pushed && (dx.abs() > 1.0 || dy.abs() > 1.0) {
                    self.push(snap.clone());
                    pushed = true;
                    self.touch();
                }
                if matches!(edge, FormEdge::E | FormEdge::SE) {
                    self.proj.width = sn(ow as f32 + dx).clamp(100, 4000);
                }
                if matches!(edge, FormEdge::S | FormEdge::SE) {
                    self.proj.height = sn(oh as f32 + dy).clamp(60, 4000);
                }
                self.mode = Mode::FormResize { edge, fx: px, fy: py, snap, ow, oh, pushed };
            }
            Mode::Resize { h, fx: px, fy: py, snap, o, mut pushed } => {
                let (dx, dy) = (fx - px, fy - py);
                if !pushed && (dx.abs() > 1.0 || dy.abs() > 1.0) {
                    self.push(snap.clone());
                    pushed = true;
                    self.touch();
                }
                if let Some(i) = self.sel {
                    let (ox, oy, ow, oh) = o;
                    let (mut x1, mut y1, mut x2, mut y2) = (ox, oy, ox + ow, oy + oh);
                    if h.west() {
                        x1 = sn(ox as f32 + dx).min(x2 - 4);
                    }
                    if h.east() {
                        x2 = sn((ox + ow) as f32 + dx).max(x1 + 4);
                    }
                    if h.north() {
                        y1 = sn(oy as f32 + dy).min(y2 - 4);
                    }
                    if h.south() {
                        y2 = sn((oy + oh) as f32 + dy).max(y1 + 4);
                    }
                    let w = &mut self.proj.widgets[i];
                    w.x = x1;
                    w.y = y1;
                    w.w = x2 - x1;
                    w.h = y2 - y1;
                }
                self.mode = Mode::Resize { h, fx: px, fy: py, snap, o, pushed };
            }
            Mode::None => {}
        }
    }

    pub fn release(&mut self) {
        let mode = std::mem::replace(&mut self.mode, Mode::None);
        if let Mode::Create { wt, sx, sy, ex, ey, moved } = mode {
            let snap = self.snapshot();
            let (x1, x2) = (sx.min(ex), sx.max(ex));
            let (y1, y2) = (sy.min(ey), sy.max(ey));
            let wd = if moved && x2 - x1 >= 8 && y2 - y1 >= 8 && wt.is_visual() {
                model::new_widget(&self.proj, wt, x1, y1, Some(x2 - x1), Some(y2 - y1))
            } else {
                model::new_widget(&self.proj, wt, sx, sy, None, None)
            };
            self.push(snap);
            let idx = self.insert_index(wd.wtype);
            self.proj.widgets.insert(idx, wd);
            self.tool = None;
            self.touch();
            self.sel = Some(idx);
        }
    }

    /// Rechteck, das beim Aufziehen gezeigt wird.
    pub fn rubber_band(&self) -> Option<(i32, i32, i32, i32)> {
        match &self.mode {
            Mode::Create { sx, sy, ex, ey, moved: true, .. } => Some((*sx, *sy, *ex, *ey)),
            _ => None,
        }
    }

    /// Neue Container kommen hinter alle anderen Komponenten (sonst verdecken sie ihren Inhalt).
    fn insert_index(&self, wt: WType) -> usize {
        if wt.is_container() {
            self.proj.widgets.iter().rposition(|w| w.wtype.is_container()).map(|p| p + 1).unwrap_or(0)
        } else {
            self.proj.widgets.len()
        }
    }

    pub fn double(&mut self, fx: f32, fy: f32) {
        if let Some(i) = self.hit_widget(fx, fy) {
            self.select(Some(i));
            self.create_event(Some(i), None);
        } else if fx >= 0.0 && fy >= 0.0 && fx < self.proj.width as f32 && fy < self.proj.height as f32 {
            self.select(None);
            self.create_event(None, None);
        }
    }

    // ------------------------------------------------------------ Befehle
    pub fn cmd_delete(&mut self) {
        if let Some(i) = self.sel {
            let s = self.snapshot();
            self.push(s);
            self.proj.widgets.remove(i);
            self.sel = None;
            self.touch();
        }
    }

    pub fn cmd_duplicate(&mut self) {
        if let Some(i) = self.sel {
            let s = self.snapshot();
            self.push(s);
            let mut wd = self.proj.widgets[i].clone();
            wd.name = model::unique_name(&self.proj, wd.wtype);
            if wd.text == self.proj.widgets[i].name && wd.wtype.has_text() && !wd.wtype.is_list() {
                wd.text = wd.name.clone();
            }
            wd.handler.clear();
            wd.events.clear();
            wd.x += GRID;
            wd.y += GRID;
            let idx = self.insert_index(wd.wtype);
            self.proj.widgets.insert(idx, wd);
            self.sel = Some(idx);
            self.touch();
        }
    }

    pub fn cmd_zorder(&mut self, front: bool) {
        if let Some(i) = self.sel {
            let s = self.snapshot();
            self.push(s);
            let w = self.proj.widgets.remove(i);
            if front {
                self.proj.widgets.push(w);
                self.sel = Some(self.proj.widgets.len() - 1);
            } else {
                self.proj.widgets.insert(0, w);
                self.sel = Some(0);
            }
            self.touch();
        }
    }

    /// Pfeiltasten: verschieben, mit `resize` (Strg) die Groesse aendern.
    pub fn cmd_nudge(&mut self, dx: i32, dy: i32, big: bool, resize: bool) {
        if let Some(i) = self.sel {
            let step = if big { GRID } else { 1 };
            let s = self.snapshot();
            self.push(s);
            let w = &mut self.proj.widgets[i];
            if resize {
                if w.wtype.is_visual() {
                    w.w = (w.w + dx * step).max(4);
                    w.h = (w.h + dy * step).max(4);
                }
            } else {
                w.x = (w.x + dx * step).max(0);
                w.y = (w.y + dy * step).max(0);
            }
            self.touch();
        }
    }

    /// Im Formular zentrieren (waagrecht oder senkrecht).
    pub fn cmd_center(&mut self, horizontal: bool) {
        if let Some(i) = self.sel {
            let s = self.snapshot();
            self.push(s);
            let (fw, fh) = (self.proj.width, self.proj.height);
            let w = &mut self.proj.widgets[i];
            if horizontal {
                w.x = ((fw - w.w) / 2).max(0);
            } else {
                w.y = ((fh - w.h) / 2).max(0);
            }
            self.touch();
        }
    }

    // ------------------------------------------------------------ Objektinspektor
    pub fn prop_rows(&self) -> Vec<PropRow> {
        let row = |key: &str, label: &str, value: String, kind| PropRow { key: key.to_string(), label: label.to_string(), value, kind };
        match self.selected() {
            None => vec![
                row("form_title", "Caption", self.proj.title.clone(), PropKind::Text),
                row("form_h", "ClientHeight", self.proj.height.to_string(), PropKind::Int),
                row("form_w", "ClientWidth", self.proj.width.to_string(), PropKind::Int),
                row("form_color", "Color", self.proj.color.clone(), PropKind::Color),
                row(
                    "form_pos",
                    "Position",
                    if self.proj.center { "poScreenCenter" } else { "poDefault" }.to_string(),
                    PropKind::Choice(&["poDefault", "poScreenCenter"]),
                ),
            ],
            Some(wd) => {
                let info = wd.wtype.info();
                let mut rows = vec![
                    row("enabled", "Enabled", py_bool(wd.enabled), PropKind::Bool),
                    row("x", "Left", wd.x.to_string(), PropKind::Int),
                    row("name", "Name", wd.name.clone(), PropKind::Text),
                    row("y", "Top", wd.y.to_string(), PropKind::Int),
                ];
                if wd.wtype.is_visual() {
                    rows.push(row("h", "Height", wd.h.to_string(), PropKind::Int));
                    rows.push(row("visible", "Visible", py_bool(wd.visible), PropKind::Bool));
                    rows.push(row("w", "Width", wd.w.to_string(), PropKind::Int));
                }
                if wd.wtype.has_text() {
                    let kind = if matches!(wd.wtype, WType::Listbox | WType::Combobox | WType::Text | WType::Datagrid) { PropKind::Items } else { PropKind::Text };
                    rows.push(row("text", info.text_prop, wd.text.replace('\n', "\\n"), kind));
                }
                for p in wd.wtype.props() {
                    rows.push(row(&format!("p:{}", p.key), p.label, wd.prop(p.key), p.kind));
                }
                rows.sort_by(|a, b| a.label.cmp(&b.label));
                rows
            }
        }
    }

    /// (Ereignisname, Anzeigename, Funktionsname)
    pub fn event_rows(&self) -> Vec<(String, String, String)> {
        match self.selected() {
            None => vec![
                ("OnCreate".into(), "OnCreate".into(), self.proj.on_create.clone()),
                ("OnClose".into(), "OnClose".into(), self.proj.on_close.clone()),
            ],
            Some(wd) => wd.wtype.events().iter().map(|e| (e.name.to_string(), e.name.to_string(), wd.event_fn(e.name))).collect(),
        }
    }

    fn parse_int(value: &str, lo: i32, hi: i32) -> Result<i32, String> {
        value
            .trim()
            .parse::<i32>()
            .map(|v| v.clamp(lo, hi))
            .map_err(|_| format!("'{}' ist keine gültige Zahl.", value))
    }

    fn check_color(value: &str) -> Result<String, String> {
        let v = value.trim();
        if v.is_empty() {
            return Ok(String::new());
        }
        let ok = (v.len() == 4 || v.len() == 7) && v.starts_with('#') && v[1..].chars().all(|c| c.is_ascii_hexdigit());
        if ok {
            Ok(v.to_ascii_lowercase())
        } else {
            Err("Farbe bitte als #rrggbb angeben (z. B. #ff0000) oder leer lassen.".into())
        }
    }

    pub fn set_prop(&mut self, key: &str, value: &str) -> Result<(), String> {
        let snap = self.snapshot();
        match self.sel {
            None => match key {
                "form_title" => self.proj.title = value.to_string(),
                "form_w" => self.proj.width = Self::parse_int(value, 100, 4000)?,
                "form_h" => self.proj.height = Self::parse_int(value, 60, 4000)?,
                "form_color" => self.proj.color = Self::check_color(value)?,
                "form_pos" => self.proj.center = value == "poScreenCenter",
                _ => return Ok(()),
            },
            Some(i) => match key {
                "text" => self.proj.widgets[i].text = value.replace("\\n", "\n"),
                "text_raw" => self.proj.widgets[i].text = value.to_string(),
                "name" => {
                    let renamed = model::rename(&mut self.proj, i, value.trim())?;
                    for (old, new) in renamed {
                        self.rename_def(&old, &new);
                    }
                }
                "x" => self.proj.widgets[i].x = Self::parse_int(value, -4000, 4000)?,
                "y" => self.proj.widgets[i].y = Self::parse_int(value, -4000, 4000)?,
                "w" => self.proj.widgets[i].w = Self::parse_int(value, 4, 4000)?,
                "h" => self.proj.widgets[i].h = Self::parse_int(value, 4, 4000)?,
                "enabled" => self.proj.widgets[i].enabled = value == "True",
                "visible" => self.proj.widgets[i].visible = value == "True",
                k if k.starts_with("p:") => {
                    let pk = &k[2..];
                    let def = self.proj.widgets[i].wtype.props().into_iter().find(|p| p.key == pk).ok_or("Unbekannte Eigenschaft.")?;
                    let norm = match def.kind {
                        PropKind::Int => {
                            let (lo, hi) = match pk {
                                "font_size" => (0, 200),
                                "group" => (0, 99),
                                "interval" => (1, 3_600_000),
                                "index" => (-1, 100_000),
                                _ => (-1_000_000, 1_000_000),
                            };
                            Self::parse_int(value, lo, hi)?.to_string()
                        }
                        PropKind::Bool => if value == "True" { "True" } else { "False" }.to_string(),
                        PropKind::Choice(opts) => {
                            if !opts.contains(&value) {
                                return Err(format!("Erlaubt: {}", opts.join(", ")));
                            }
                            value.to_string()
                        }
                        PropKind::Color => Self::check_color(value)?,
                        PropKind::Text | PropKind::File | PropKind::Items => value.to_string(),
                    };
                    self.proj.widgets[i].set_prop(pk, &norm);
                }
                _ => return Ok(()),
            },
        }
        self.push(snap);
        self.touch();
        Ok(())
    }

    // ------------------------------------------------------------ Ereignisse / Editor
    /// Wie Delphi: Doppelklick legt die Ereignis-Funktion an und springt hin.
    /// `ev` = Ereignisname (None = Standard-Ereignis).
    pub fn create_event(&mut self, target: Option<usize>, ev: Option<&str>) {
        let snap = self.snapshot();
        let (name, changed);
        match target {
            None => {
                let close = ev == Some("OnClose");
                let (cur, def) = if close { (self.proj.on_close.clone(), "FormClose") } else { (self.proj.on_create.clone(), "FormCreate") };
                name = if cur.is_empty() { def.to_string() } else { cur.clone() };
                changed = cur != name;
                if close {
                    self.proj.on_close = name.clone();
                } else {
                    self.proj.on_create = name.clone();
                }
            }
            Some(i) => {
                let wd = &mut self.proj.widgets[i];
                let evs = wd.wtype.events();
                let found = match ev {
                    Some(n) => evs.iter().find(|e| e.name == n).copied(),
                    None => evs.first().copied(),
                };
                let Some(e) = found else { return };
                let cur = wd.event_fn(e.name);
                name = if cur.is_empty() { format!("{}{}", wd.name, e.suffix) } else { cur.clone() };
                changed = cur != name;
                wd.set_event_fn(e.name, &name);
            }
        }
        if changed {
            self.push(snap);
            self.touch();
        }
        self.ensure_stub(&name);
        self.goto_def(&name);
    }

    pub fn ensure_stub(&mut self, name: &str) {
        if !codegen::has_def(&self.logic, name) {
            if !self.logic.is_empty() && !self.logic.ends_with('\n') {
                self.logic.push('\n');
            }
            self.logic.push_str(&codegen::stub_text(name));
            self.logic_dirty = true;
        }
    }

    pub fn goto_def(&mut self, name: &str) {
        self.view = View::Code;
        let mut offset = 0usize; // in Zeichen
        let mut lines = self.logic.split('\n').peekable();
        while let Some(line) = lines.next() {
            let len = line.chars().count();
            if codegen::def_name_range(line, name).is_some() {
                // Cursor ans Ende der Zeile nach dem def
                let next_len = lines.peek().map(|l| l.chars().count()).unwrap_or(0);
                self.goto_char = Some(offset + len + 1 + next_len);
                return;
            }
            offset += len + 1;
        }
    }

    fn rename_def(&mut self, old: &str, new: &str) {
        if old.is_empty() || old == new || !codegen::has_def(&self.logic, old) || codegen::has_def(&self.logic, new) {
            return;
        }
        let mut out = String::with_capacity(self.logic.len());
        let mut done = false;
        for (n, line) in self.logic.split('\n').enumerate() {
            if n > 0 {
                out.push('\n');
            }
            if !done {
                if let Some((a, b)) = codegen::def_name_range(line, old) {
                    out.push_str(&line[..a]);
                    out.push_str(new);
                    out.push_str(&line[b..]);
                    done = true;
                    continue;
                }
            }
            out.push_str(line);
        }
        self.logic = out;
        self.logic_dirty = true;
    }

    // ------------------------------------------------------------ Dateien
    pub fn load_logic(&mut self) {
        match self.logic_file().filter(|p| p.exists()) {
            Some(p) => {
                self.logic = std::fs::read_to_string(&p).unwrap_or_default().replace("\r\n", "\n");
                self.logic_mtime = mtime(&p);
            }
            None => {
                self.logic = codegen::header(self.stem().as_deref());
                self.logic_mtime = None;
            }
        }
        self.logic_dirty = false;
    }

    pub fn new_project(&mut self) {
        let (snap, grid, tk) = (self.snap, self.show_grid, self.toolkit_choice);
        let log = std::mem::take(&mut self.log);
        *self = State::new();
        self.snap = snap;
        self.show_grid = grid;
        self.toolkit_choice = tk;
        self.log = log;
    }

    pub fn open(&mut self, path: &Path) -> Result<(), String> {
        let proj = model::load(path)?;
        self.proj = proj;
        self.path = Some(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
        self.sel = None;
        self.undo.clear();
        self.redo.clear();
        self.dirty = false;
        self.mode = Mode::None;
        self.tool = None;
        self.view = View::Form;
        self.load_logic();
        Ok(())
    }

    /// Setzt einen neuen Speicherort ("Speichern unter").
    pub fn set_path(&mut self, path: PathBuf) {
        if self.path.as_ref() != Some(&path) {
            self.logic_mtime = None;
        }
        self.path = Some(path);
    }

    pub fn save(&mut self, policy: LogicPolicy) -> Result<SaveResult, String> {
        let path = self.path.clone().ok_or("Kein Dateiname gesetzt.")?;
        model::save(&path, &self.proj).map_err(|e| format!("Speichern fehlgeschlagen: {}", e))?;
        let lp = self.logic_file().ok_or("Kein Dateiname gesetzt.")?;
        if lp.exists() {
            let changed = self.logic_mtime.is_none() || mtime(&lp) != self.logic_mtime;
            if changed {
                match policy {
                    LogicPolicy::Ask => return Ok(SaveResult::Conflict),
                    LogicPolicy::LoadExisting => {
                        self.load_logic();
                        self.dirty = false;
                        return Ok(SaveResult::Saved);
                    }
                    LogicPolicy::Overwrite => {}
                }
            }
        }
        let mut text = self.logic.trim_end_matches('\n').to_string();
        text.push('\n');
        std::fs::write(&lp, text).map_err(|e| format!("Speichern fehlgeschlagen: {}", e))?;
        self.logic_mtime = mtime(&lp);
        self.logic_dirty = false;
        self.dirty = false;
        Ok(SaveResult::Saved)
    }

    /// Code erzeugen (legt fehlende Funktions-Stubs im Editor an, speichert, schreibt Dateien).
    pub fn generate(&mut self, policy: LogicPolicy) -> Result<GenResult, String> {
        for name in codegen::handler_names(&self.proj) {
            self.ensure_stub(&name);
        }
        if self.save(policy)? == SaveResult::Conflict {
            return Ok(GenResult::Conflict);
        }
        let folder = self.folder().ok_or("Kein Projektordner.")?;
        let stem = self.stem().ok_or("Kein Dateiname gesetzt.")?;
        let (written, _) = codegen::generate(&self.proj, &folder, &stem, &self.toolkits())
            .map_err(|e| format!("Code erzeugen fehlgeschlagen: {}", e))?;
        self.log(format!("Code erzeugt: {}", written.join(", ")), LogKind::Info);
        Ok(GenResult::Done(written))
    }
}

fn py_bool(b: bool) -> String {
    if b { "True".into() } else { "False".into() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pyphi7_test_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn click_create(s: &mut State, wt: WType, x: f32, y: f32) {
        s.tool = Some(wt);
        s.press(x, y);
        s.release();
    }

    #[test]
    fn create_by_click_and_by_drag() {
        let mut s = State::new();
        click_create(&mut s, WType::Button, 250.0, 232.0);
        let w = s.selected().unwrap();
        assert_eq!((w.name.as_str(), w.x, w.y, w.w, w.h), ("Button1", 248, 232, 75, 25));
        assert!(s.tool.is_none());

        s.tool = Some(WType::Entry);
        s.press(16.0, 240.0);
        s.drag(100.0, 262.0);
        assert!(s.rubber_band().is_some());
        s.drag(150.0, 264.0);
        s.release();
        let w = s.selected().unwrap();
        assert_eq!((w.x, w.y, w.w, w.h), (16, 240, 136, 24));
    }

    #[test]
    fn move_resize_undo_redo() {
        let mut s = State::new();
        click_create(&mut s, WType::Button, 248.0, 232.0);
        // verschieben
        s.press(280.0, 244.0);
        s.drag(300.0, 254.0);
        s.drag(330.0, 252.0);
        s.release();
        let w = s.selected().unwrap().clone();
        assert_eq!((w.x, w.y), (296, 240));
        // Groesse am Griff SE
        let (_, (hx, hy)) = handle_points(&w)[4];
        s.press(hx as f32, hy as f32);
        s.drag(hx as f32 + 20.0, hy as f32 + 10.0);
        s.release();
        let w = s.selected().unwrap().clone();
        assert_eq!((w.w, w.h), (96, 32));
        // Undo: Groesse -> Position -> Anlegen -> leer
        s.cmd_undo();
        assert_eq!(s.selected().unwrap().w, 75);
        s.cmd_undo();
        assert_eq!(s.selected().unwrap().x, 248);
        s.cmd_undo();
        assert!(s.proj.widgets.is_empty());
        s.cmd_redo();
        assert_eq!(s.proj.widgets.len(), 1);
    }

    #[test]
    fn click_without_move_creates_no_undo_entry() {
        let mut s = State::new();
        click_create(&mut s, WType::Label, 10.0, 10.0);
        let n = s.undo.len();
        s.press(12.0, 12.0);
        s.release();
        assert_eq!(s.undo.len(), n);
    }

    #[test]
    fn delete_duplicate_zorder() {
        let mut s = State::new();
        click_create(&mut s, WType::Button, 8.0, 8.0);
        click_create(&mut s, WType::Entry, 64.0, 64.0);
        s.select(Some(0));
        s.cmd_duplicate();
        assert_eq!(s.proj.widgets.len(), 3);
        assert_eq!(s.selected().unwrap().name, "Button2");
        assert_eq!(s.selected().unwrap().text, "Button2");
        assert_eq!((s.selected().unwrap().x, s.selected().unwrap().y), (16, 16));
        s.cmd_zorder(false);
        assert_eq!(s.sel, Some(0));
        assert_eq!(s.proj.widgets[0].name, "Button2");
        s.cmd_delete();
        assert_eq!(s.proj.widgets.len(), 2);
        s.cmd_undo();
        assert_eq!(s.proj.widgets.len(), 3);
    }

    #[test]
    fn event_creates_stub_and_rename_follows() {
        let mut s = State::new();
        click_create(&mut s, WType::Button, 8.0, 8.0);
        s.double(10.0, 10.0);
        assert_eq!(s.selected().unwrap().handler, "Button1Click");
        assert!(codegen::has_def(&s.logic, "Button1Click"));
        assert_eq!(s.view, View::Code);
        assert!(s.goto_char.is_some());
        // Cursorzeile = Zeile nach dem def
        let upto: String = s.logic.chars().take(s.goto_char.unwrap()).collect();
        assert!(upto.ends_with("hier deinen Code einfuegen\")"), "{:?}", &upto[upto.len().saturating_sub(40)..]);
        // Zweiter Doppelklick legt kein Duplikat an
        s.double(10.0, 10.0);
        assert_eq!(s.logic.matches("def Button1Click(").count(), 1);
        // Umbenennen zieht Funktion, Handler und Caption mit
        s.set_prop("name", "btnOK").unwrap();
        let w = s.selected().unwrap();
        assert_eq!((w.handler.as_str(), w.text.as_str()), ("btnOKClick", "btnOK"));
        assert!(codegen::has_def(&s.logic, "btnOKClick") && !codegen::has_def(&s.logic, "Button1Click"));
        // Formular-Doppelklick -> FormCreate
        s.double(300.0, 200.0);
        assert_eq!(s.proj.on_create, "FormCreate");
        assert!(codegen::has_def(&s.logic, "FormCreate"));
        // Label hat kein Ereignis
        click_create(&mut s, WType::Label, 100.0, 100.0);
        let n = s.logic.len();
        s.double(101.0, 101.0);
        assert_eq!(s.logic.len(), n);
    }

    #[test]
    fn set_prop_validation() {
        let mut s = State::new();
        click_create(&mut s, WType::Button, 8.0, 8.0);
        click_create(&mut s, WType::Button, 100.0, 100.0);
        s.select(Some(1));
        assert!(s.set_prop("name", "Button1").is_err());
        assert!(s.set_prop("name", "2bad").is_err());
        assert!(s.set_prop("x", "abc").is_err());
        assert_eq!(s.selected().unwrap().name, "Button2");
        s.set_prop("x", "40").unwrap();
        s.set_prop("w", "1").unwrap();
        assert_eq!((s.selected().unwrap().x, s.selected().unwrap().w), (40, 4)); // Mindestbreite
        s.set_prop("enabled", "False").unwrap();
        assert!(!s.selected().unwrap().enabled);
        s.select(None);
        s.set_prop("form_title", "Mein Programm").unwrap();
        s.set_prop("form_w", "640").unwrap();
        assert_eq!((s.proj.title.as_str(), s.proj.width), ("Mein Programm", 640));
        // Reihenfolge der Zeilen wie Delphi (alphabetisch)
        s.select(Some(0));
        let labels: Vec<String> = s.prop_rows().into_iter().map(|r| r.label).collect();
        assert_eq!(
            labels,
            ["Caption", "Enabled", "Font.Bold", "Font.Color", "Font.Size", "Height", "Hint", "Left", "Name", "Top", "Visible", "Width"]
        );
    }

    #[test]
    fn save_generate_never_clobbers_user_logic() {
        let d = tmpdir("save");
        let mut s = State::new();
        s.set_path(d.join("Mein Projekt.pyphi"));
        click_create(&mut s, WType::Button, 8.0, 8.0);
        s.double(10.0, 10.0);
        assert!(matches!(s.generate(LogicPolicy::Ask), Ok(GenResult::Done(_))));
        let lp = d.join("Mein_Projekt_logic.py");
        assert!(lp.exists() && d.join("Mein_Projekt_tk.py").exists());
        assert!(!s.is_dirty());
        // Nutzer aendert die Datei extern -> Konflikt statt Ueberschreiben
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(&lp, "# MEINE EIGENEN AENDERUNGEN\ndef Button1Click(ui):\n    pass\n").unwrap();
        assert_eq!(s.generate(LogicPolicy::Ask), Ok(GenResult::Conflict));
        assert!(std::fs::read_to_string(&lp).unwrap().starts_with("# MEINE EIGENEN"));
        // "vorhandene Datei laden": Editor uebernimmt sie, nichts geht verloren
        assert!(matches!(s.generate(LogicPolicy::LoadExisting), Ok(GenResult::Done(_))));
        assert!(s.logic.starts_with("# MEINE EIGENEN"));
        assert!(std::fs::read_to_string(&lp).unwrap().starts_with("# MEINE EIGENEN"));
        // erneut oeffnen: Layout und Logik kommen zurueck
        let mut t = State::new();
        t.open(&d.join("Mein Projekt.pyphi")).unwrap();
        assert_eq!(t.proj, s.proj);
        assert!(t.logic.starts_with("# MEINE EIGENEN"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn container_moves_children_and_sits_behind() {
        let mut s = State::new();
        click_create(&mut s, WType::Button, 40.0, 40.0);
        click_create(&mut s, WType::Panel, 24.0, 24.0);
        // Panel muss hinter dem Button liegen (Index 0)
        assert_eq!(s.proj.widgets[0].wtype, WType::Panel);
        assert_eq!(s.sel, Some(0));
        let (px, py) = (s.proj.widgets[0].x, s.proj.widgets[0].y);
        let (bx, by) = (s.proj.widgets[1].x, s.proj.widgets[1].y);
        assert!(s.proj.widgets[1].x >= px && s.proj.widgets[1].x + 75 <= px + s.proj.widgets[0].w);
        // Klick auf freie Panelflaeche (nicht auf dem Button) -> Panel ziehen
        s.press(px as f32 + 150.0, py as f32 + 30.0);
        s.drag(px as f32 + 166.0, py as f32 + 46.0);
        s.release();
        assert_eq!((s.proj.widgets[0].x, s.proj.widgets[0].y), (px + 16, py + 16));
        assert_eq!((s.proj.widgets[1].x, s.proj.widgets[1].y), (bx + 16, by + 16));
    }

    #[test]
    fn extra_props_events_and_defaults() {
        let mut s = State::new();
        click_create(&mut s, WType::Entry, 16.0, 16.0);
        // Standardwerte werden nicht gespeichert
        s.set_prop("p:readonly", "True").unwrap();
        assert!(s.selected().unwrap().prop_bool("readonly"));
        s.set_prop("p:readonly", "False").unwrap();
        assert!(s.selected().unwrap().props.is_empty());
        assert!(s.set_prop("p:font_color", "rot").is_err());
        s.set_prop("p:font_color", "#FF0000").unwrap();
        assert_eq!(s.selected().unwrap().prop("font_color"), "#ff0000");
        assert!(s.set_prop("p:font_size", "abc").is_err());
        s.set_prop("p:font_size", "9999").unwrap();
        assert_eq!(s.selected().unwrap().prop_int("font_size"), 200);
        // weitere Ereignisse
        s.create_event(Some(0), Some("OnEnter"));
        assert_eq!(s.proj.widgets[0].events.get("OnEnter").map(|x| x.as_str()), Some("Edit1Enter"));
        assert!(codegen::has_def(&s.logic, "Edit1Enter"));
        assert_eq!(s.proj.widgets[0].handler, "");
        // Umbenennen zieht alle Standardnamen mit
        s.create_event(Some(0), None);
        s.set_prop("name", "txtName").unwrap();
        assert_eq!(s.proj.widgets[0].handler, "txtNameChange");
        assert_eq!(s.proj.widgets[0].event_fn("OnEnter"), "txtNameEnter");
        assert!(codegen::has_def(&s.logic, "txtNameEnter") && !codegen::has_def(&s.logic, "Edit1Enter"));
        // Form-Eigenschaften
        s.select(None);
        s.set_prop("form_pos", "poScreenCenter").unwrap();
        assert!(s.proj.center);
        assert!(s.set_prop("form_color", "blau").is_err());
        s.create_event(None, Some("OnClose"));
        assert_eq!(s.proj.on_close, "FormClose");
    }

    #[test]
    fn items_roundtrip_and_form_resize() {
        let mut s = State::new();
        click_create(&mut s, WType::Listbox, 16.0, 16.0);
        s.set_prop("text_raw", "Eins|Zwei|Drei").unwrap();
        assert_eq!(model::items_of(s.selected().unwrap()), ["Eins", "Zwei", "Drei"]);
        let rows = s.prop_rows();
        let items = rows.iter().find(|r| r.key == "text").unwrap();
        assert_eq!(items.kind, PropKind::Items);
        // Formular an der Ecke vergroessern
        s.select(None);
        let (w, h) = (s.proj.width as f32, s.proj.height as f32);
        s.press(w + 4.0, h + 4.0);
        s.drag(w + 44.0, h + 28.0);
        s.release();
        assert_eq!((s.proj.width, s.proj.height), (440, 328));
        s.cmd_undo();
        assert_eq!((s.proj.width, s.proj.height), (400, 300));
        // Timer: nur Klick, keine Griffe
        click_create(&mut s, WType::Timer, 100.0, 100.0);
        let t = s.selected().unwrap().clone();
        assert_eq!((t.w, t.h), (24, 24));
        assert!(s.hit_handle(t.x as f32 + 24.0, t.y as f32 + 24.0).is_none());
        assert_eq!(s.prop_rows().iter().map(|r| r.label.as_str()).collect::<Vec<_>>(), ["Enabled", "Interval", "Left", "Name", "Top"]);
    }

    #[test]
    fn database_and_grid_components() {
        let mut s = State::new();
        click_create(&mut s, WType::Database, 20.0, 20.0);
        let d = s.selected().unwrap().clone();
        assert_eq!((d.name.as_str(), d.w, d.h), ("Database1", 24, 24));
        assert!(!d.wtype.is_visual() && d.wtype.events().is_empty());
        assert_eq!(d.prop("file"), "daten.db");
        assert!(s.hit_handle(d.x as f32 + 24.0, d.y as f32 + 24.0).is_none());
        click_create(&mut s, WType::Datagrid, 60.0, 60.0);
        assert_eq!(s.selected().unwrap().name, "Grid1");
        s.set_prop("text_raw", "ID|Name|Telefon").unwrap();
        let rows = s.prop_rows();
        assert_eq!(rows.iter().find(|r| r.key == "text").unwrap().kind, PropKind::Items);
        assert_eq!(rows.iter().find(|r| r.key == "text").unwrap().label, "Columns");
        assert_eq!(model::items_of(s.selected().unwrap()), ["ID", "Name", "Telefon"]);
        // Doppelklick -> Grid1Click; zweites Ereignis OnDblClick
        s.create_event(Some(1), None);
        assert_eq!(s.proj.widgets[1].handler, "Grid1Click");
        s.create_event(Some(1), Some("OnDblClick"));
        assert_eq!(s.proj.widgets[1].event_fn("OnDblClick"), "Grid1DblClick");
        // Palette hat den Reiter
        assert_eq!(model::PALETTE[3].0, "Bibliotheken");
        // Code-Erzeugung
        let tk = codegen::gen_tk(&s.proj, "t");
        assert!(tk.contains("self._database('Database1', 'daten.db')"));
        assert!(tk.contains("_w = _Grid(self.root, ['ID', 'Name', 'Telefon'])"));
        assert!(tk.contains("_w.tree.bind('<Double-Button-1>'"));
        let qt = codegen::gen_qt(&s.proj, "t");
        assert!(qt.contains("QTableWidget(self)") && qt.contains("setHorizontalHeaderLabels(['ID', 'Name', 'Telefon'])"));
        assert!(qt.contains("self._database('Database1', 'daten.db')") && qt.contains("cellDoubleClicked"));
        // Speichern/Laden
        let json = serde_json::to_string(&s.proj).unwrap();
        let back: model::Project = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s.proj);
    }
}
