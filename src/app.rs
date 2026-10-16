//! Die egui-Oberflaeche von Pyphi7 im Delphi-Look.

use crate::codegen::{self, Toolkit};
use crate::editing;
use crate::libs;
use crate::model::{self, WType};
use crate::pyhl;
use crate::state::{handle_points, FormEdge, GenResult, Handle, LogKind, LogicPolicy, PropKind, SaveResult, State, View, GRID};
use eframe::egui::{
    self, pos2, text::CCursor, text::CCursorRange, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId,
    Key, Modifiers, Painter, Rect, Sense, Stroke, Ui,
};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

const APP_NAME: &str = "Pyphi7";
const FACE: Color32 = Color32::from_rgb(0xd4, 0xd0, 0xc8);
const WHITE: Color32 = Color32::WHITE;
const GRAY: Color32 = Color32::from_rgb(0x80, 0x80, 0x80);
const DARK: Color32 = Color32::from_rgb(0x40, 0x40, 0x40);
const BLACK: Color32 = Color32::BLACK;
const NAVY: Color32 = Color32::from_rgb(0x0a, 0x24, 0x6a);
const WORKSPACE: Color32 = Color32::from_rgb(0x7b, 0x7b, 0x7b);
const TITLE_H: f32 = 20.0;
const OX: f32 = 28.0;
const OY: f32 = 28.0 + TITLE_H;
const FORM_CLASS: &str = "Form1";

const HELP_TEXT: &str = "So arbeitest du (wie früher in Delphi):\n\n\
1. Komponente in der Palette oben anklicken, dann im Formular klicken\n   (oder ein Rechteck aufziehen, um die Größe gleich festzulegen).\n\
2. Mit der Maus verschieben, an den schwarzen Griffen in der Größe ändern.\n\
3. Eigenschaften im Objektinspektor links ändern (Wert anklicken).\n   Items/Lines (ListBox, ComboBox, Memo): Knopf \"...\" öffnet den Zeilen-Editor.\n\
4. DOPPELKLICK auf eine Komponente legt die Ereignis-Funktion\n   (z. B. Button1Click) im Code-Editor an und springt dorthin.\n\
5. F9 erzeugt den Code und startet das Programm.\n\n\
Tastatur:  F12 Formular/Code umschalten   F9 Start   Strg+F2 Programm beenden\n\
           Entf löschen   Pfeile verschieben (Umschalt = grob)\n\
           Strg+Pfeile Größe ändern   Formular-Rand/Ecke ziehen = Formulargröße\n\
           Strg+D duplizieren   Strg+Z / Strg+Y rückgängig / wiederholen\n\
           Strg+S alles speichern\n\n\
Oben links wählst du das Toolkit: Tkinter, Qt (PySide6) oder Beide.\n\
Deine Funktionen stehen in <Projekt>_logic.py und laufen mit beiden.";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    New,
    Open,
    Save,
    SaveAs,
    Run,
    Generate,
    Stop,
    Quit,
    ToggleView,
    Undo,
    Redo,
    Duplicate,
    Delete,
    ZFront,
    ZBack,
    ChoosePython,
    ExportExe,
    Help,
    CenterH,
    CenterV,
}

enum Modal {
    Info(String),
    /// Zeilen-Editor fuer Items/Lines (list = Eintraege mit | speichern)
    Items { text: String, list: bool },
    SaveChanges(Action),
    LogicConflict(Action),
}

struct Proc {
    child: Arc<Mutex<Child>>,
}

pub struct App {
    st: State,
    modal: Option<Modal>,
    queue: Vec<Action>,
    procs: Vec<Proc>,
    tx: Sender<(String, LogKind)>,
    rx: Receiver<(String, LogKind)>,
    python: Option<(String, Vec<String>)>,
    hint: String,
    insp_tab: usize,
    pal_tab: usize,
    insp_key: Option<String>,
    insp_buf: String,
    insp_rect: Rect,
    cursor_text: String,
    allow_close: bool,
    last_title: String,
    /// Zusatz-Bibliotheken: (normierter Name, Anzeigename, Version) aus `pip list`
    libs: Vec<(String, String, String)>,
    libs_state: LibsState,
    lib_input: String,
    lib_tx: Sender<Result<Vec<(String, String, String)>, String>>,
    lib_rx: Receiver<Result<Vec<(String, String, String)>, String>>,
    pip_busy: bool,
    libs_open: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LibsState {
    Unknown,
    Loading,
    Loaded,
    Failed,
}

// ------------------------------------------------------------------ Zeichenhilfen

fn fill(p: &Painter, x1: f32, y1: f32, x2: f32, y2: f32, c: Color32) {
    p.rect_filled(Rect::from_min_max(pos2(x1, y1), pos2(x2 + 1.0, y2 + 1.0)), CornerRadius::ZERO, c);
}

fn hline(p: &Painter, x1: f32, x2: f32, y: f32, c: Color32) {
    fill(p, x1, y, x2, y, c);
}

fn vline(p: &Painter, x: f32, y1: f32, y2: f32, c: Color32) {
    fill(p, x, y1, x, y2, c);
}

/// Win9x-Rahmen. Koordinaten sind inklusive (x2/y2 = letzter Pixel).
fn bevel(p: &Painter, x1: f32, y1: f32, x2: f32, y2: f32, sunken: bool, face: Color32) {
    fill(p, x1, y1, x2, y2, face);
    let (otl, obr, itl, ibr) = if sunken { (GRAY, WHITE, DARK, FACE) } else { (WHITE, BLACK, FACE, GRAY) };
    hline(p, x1, x2, y1, otl);
    vline(p, x1, y1, y2, otl);
    hline(p, x1, x2, y2, obr);
    vline(p, x2, y1, y2, obr);
    hline(p, x1 + 1.0, x2 - 1.0, y1 + 1.0, itl);
    vline(p, x1 + 1.0, y1 + 1.0, y2 - 1.0, itl);
    hline(p, x1 + 1.0, x2 - 1.0, y2 - 1.0, ibr);
    vline(p, x2 - 1.0, y1 + 1.0, y2 - 1.0, ibr);
}

fn rect_bevel(p: &Painter, r: Rect, sunken: bool, face: Color32) {
    bevel(p, r.min.x.round(), r.min.y.round(), r.max.x.round() - 1.0, r.max.y.round() - 1.0, sunken, face);
}

fn fit(p: &Painter, text: &str, font: &FontId, width: f32) -> String {
    let mut s = text.to_string();
    while !s.is_empty() && p.layout_no_wrap(s.clone(), font.clone(), BLACK).size().x > width {
        s.pop();
    }
    s
}

fn paint_icon(p: &Painter, r: Rect, kind: Option<WType>, font: &FontId) {
    let c = r.center();
    let (x1, y1) = (c.x - 9.0, c.y - 7.0);
    let (x2, y2) = (c.x + 9.0, c.y + 7.0);
    match kind {
        None => {
            let o = pos2(c.x - 4.0, c.y - 8.0);
            let pts = vec![
                o,
                o + vec2(0.0, 14.0),
                o + vec2(3.5, 10.5),
                o + vec2(6.0, 16.0),
                o + vec2(8.0, 15.0),
                o + vec2(5.5, 9.5),
                o + vec2(10.0, 9.5),
            ];
            p.add(egui::Shape::convex_polygon(pts, WHITE, Stroke::new(1.0, BLACK)));
        }
        Some(WType::Label) => {
            p.text(c, Align2::CENTER_CENTER, "A", FontId::proportional(17.0), NAVY);
        }
        Some(WType::Entry) => {
            bevel(p, x1, y1 + 1.0, x2, y2 - 1.0, true, WHITE);
            p.text(c, Align2::CENTER_CENTER, "ab|", FontId::proportional(11.0), BLACK);
        }
        Some(WType::Text) => {
            bevel(p, x1, y1 - 1.0, x2, y2 + 1.0, true, WHITE);
            for i in 0..3 {
                hline(p, x1 + 3.0, x2 - 3.0 - (i as f32) * 3.0, y1 + 3.0 + i as f32 * 4.0, BLACK);
            }
        }
        Some(WType::Button) => {
            bevel(p, x1, y1, x2, y2, false, FACE);
            p.text(c, Align2::CENTER_CENTER, "OK", FontId::proportional(10.0), BLACK);
        }
        Some(WType::Checkbox) => {
            bevel(p, c.x - 6.0, c.y - 6.0, c.x + 6.0, c.y + 6.0, true, WHITE);
            let pts = [pos2(c.x - 3.5, c.y), pos2(c.x - 1.0, c.y + 3.0), pos2(c.x + 4.0, c.y - 3.5)];
            p.line_segment([pts[0], pts[1]], Stroke::new(2.0, BLACK));
            p.line_segment([pts[1], pts[2]], Stroke::new(2.0, BLACK));
        }
        Some(WType::Listbox) => {
            bevel(p, x1 + 1.0, y1 - 1.0, x2 - 1.0, y2 + 1.0, true, WHITE);
            for i in 0..3 {
                hline(p, x1 + 4.0, x2 - 4.0 - (i as f32) * 2.0, y1 + 3.0 + i as f32 * 4.0, BLACK);
            }
        }
        Some(WType::Combobox) => {
            bevel(p, x1, y1 + 1.0, x2, y2 - 1.0, true, WHITE);
            bevel(p, x2 - 8.0, y1 + 3.0, x2 - 2.0, y2 - 3.0, false, FACE);
            let a = pos2(x2 - 5.0, c.y + 1.0);
            p.add(egui::Shape::convex_polygon(vec![a + vec2(-2.5, -1.5), a + vec2(2.5, -1.5), a + vec2(0.0, 1.5)], BLACK, Stroke::NONE));
        }
        Some(WType::Radiobutton) => {
            p.circle_filled(c, 6.5, WHITE);
            p.circle_stroke(c, 6.5, Stroke::new(1.0, GRAY));
            p.circle_filled(c, 2.5, BLACK);
        }
        Some(WType::Groupbox) => {
            let r = Rect::from_min_max(pos2(x1, y1 + 1.0), pos2(x2, y2 + 2.0));
            p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, GRAY), egui::StrokeKind::Inside);
            fill(p, x1 + 3.0, y1 - 1.0, x1 + 9.0, y1 + 3.0, FACE);
            hline(p, x1 + 4.0, x1 + 8.0, y1 + 1.0, BLACK);
        }
        Some(WType::Panel) => {
            bevel(p, x1, y1 - 1.0, x2, y2 + 1.0, false, FACE);
        }
        Some(WType::Progressbar) => {
            bevel(p, x1, c.y - 4.0, x2, c.y + 4.0, true, FACE);
            for i in 0..3 {
                fill(p, x1 + 2.0 + i as f32 * 5.0, c.y - 2.0, x1 + 5.0 + i as f32 * 5.0, c.y + 2.0, NAVY);
            }
        }
        Some(WType::Trackbar) => {
            bevel(p, x1, c.y - 1.0, x2, c.y + 1.0, true, FACE);
            bevel(p, c.x - 2.0, c.y - 6.0, c.x + 3.0, c.y + 6.0, false, FACE);
        }
        Some(WType::Spinedit) => {
            bevel(p, x1, y1, x2, y2, true, WHITE);
            bevel(p, x2 - 8.0, y1 + 2.0, x2 - 2.0, c.y, false, FACE);
            bevel(p, x2 - 8.0, c.y, x2 - 2.0, y2 - 2.0, false, FACE);
            p.text(pos2(x1 + 5.0, c.y), Align2::LEFT_CENTER, "0", FontId::proportional(10.0), BLACK);
        }
        Some(WType::Image) => {
            bevel(p, x1 + 1.0, y1 - 1.0, x2 - 1.0, y2 + 1.0, true, WHITE);
            p.circle_filled(pos2(x2 - 5.0, y1 + 3.0), 2.0, Color32::from_rgb(0xe0, 0xa0, 0x00));
            p.add(egui::Shape::convex_polygon(
                vec![pos2(x1 + 3.0, y2), pos2(x1 + 9.0, y1 + 5.0), pos2(x1 + 13.0, y2)],
                Color32::from_rgb(0x20, 0x80, 0x20),
                Stroke::NONE,
            ));
        }
        Some(WType::Database) => {
            let (ex, ey) = (6.5, 2.5);
            fill(p, c.x - ex, c.y - 4.5, c.x + ex, c.y + 4.5, Color32::from_rgb(0xc8, 0xd8, 0xf0));
            vline(p, c.x - ex, c.y - 4.5, c.y + 4.5, DARK);
            vline(p, c.x + ex, c.y - 4.5, c.y + 4.5, DARK);
            for dy in [-4.5f32, 0.0, 4.5] {
                let top = pos2(c.x, c.y + dy);
                let mut pts = Vec::new();
                for i in 0..=12 {
                    let a = std::f32::consts::PI * (i as f32) / 12.0;
                    pts.push(top + vec2(-ex * a.cos(), if dy < 0.0 { -ey * a.sin() } else { ey * a.sin() }));
                }
                p.add(egui::Shape::line(pts, Stroke::new(1.0, DARK)));
            }
            p.add(egui::Shape::closed_line(
                (0..16).map(|i| {
                    let a = std::f32::consts::TAU * (i as f32) / 16.0;
                    pos2(c.x + ex * a.cos(), c.y - 4.5 + ey * a.sin())
                }).collect(),
                Stroke::new(1.0, DARK),
            ));
        }
        Some(WType::Datagrid) => {
            bevel(p, x1, y1 - 1.0, x2, y2 + 1.0, true, WHITE);
            fill(p, x1 + 2.0, y1 + 1.0, x2 - 2.0, y1 + 4.0, FACE);
            vline(p, c.x, y1 + 1.0, y2 - 1.0, GRAY);
            for i in 0..3 {
                hline(p, x1 + 2.0, x2 - 2.0, y1 + 4.0 + i as f32 * 3.0, GRAY);
            }
        }
        Some(WType::Timer) => {
            p.circle_filled(c, 7.0, WHITE);
            p.circle_stroke(c, 7.0, Stroke::new(1.0, BLACK));
            p.line_segment([c, c + vec2(0.0, -5.0)], Stroke::new(1.0, BLACK));
            p.line_segment([c, c + vec2(3.5, 1.0)], Stroke::new(1.0, BLACK));
        }
    }
    let _ = font;
}

/// Klassischer Werkzeug-Knopf: erhaben, gedrueckt/gewaehlt = eingedrueckt.
fn tool_button(ui: &mut Ui, size: egui::Vec2, selected: bool, paint: impl FnOnce(&Painter, Rect)) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let down = resp.is_pointer_button_down_on();
    let sunken = selected || down;
    let p = ui.painter();
    rect_bevel(p, rect, sunken, if selected { Color32::from_rgb(0xf4, 0xf2, 0xec) } else { FACE });
    let inner = if sunken { rect.translate(vec2(1.0, 1.0)) } else { rect };
    paint(p, inner);
    resp
}

fn line_col(text: &str, char_idx: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, c) in text.chars().enumerate() {
        if i >= char_idx {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn char_to_byte(s: &str, ci: usize) -> usize {
    s.char_indices().nth(ci).map(|(b, _)| b).unwrap_or(s.len())
}

// ------------------------------------------------------------------ App

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Option<PathBuf>) -> Self {
        setup_style(&cc.egui_ctx);
        let (tx, rx) = channel();
        let (lib_tx, lib_rx) = channel();
        let mut st = State::new();
        if let Some(p) = open {
            if let Err(e) = st.open(&p) {
                st.log(format!("Konnte {} nicht öffnen: {}", p.display(), e), LogKind::Err);
            }
        }
        App {
            st,
            modal: None,
            queue: Vec::new(),
            procs: Vec::new(),
            tx,
            rx,
            python: None,
            hint: String::new(),
            insp_tab: 0,
            pal_tab: 0,
            insp_key: None,
            insp_buf: String::new(),
            insp_rect: Rect::NOTHING,
            cursor_text: "1: 1".into(),
            allow_close: false,
            last_title: String::new(),
            libs: Vec::new(),
            libs_state: LibsState::Unknown,
            lib_input: String::new(),
            lib_tx,
            lib_rx,
            pip_busy: false,
            libs_open: false,
        }
    }

    // -------------------------------------------------------------- Aktionen
    fn request(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::New | Action::Open | Action::Quit if self.st.is_dirty() => {
                self.modal = Some(Modal::SaveChanges(a));
            }
            _ => self.perform(ctx, a),
        }
    }

    fn perform(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::New => {
                self.stop_all();
                self.st.new_project();
            }
            Action::Open => {
                let picked = rfd::FileDialog::new()
                    .set_title("Projekt öffnen")
                    .add_filter("Pyphi7-Projekt", &["pyphi", "pyrad"])
                    .pick_file();
                if let Some(p) = picked {
                    if let Err(e) = self.st.open(&p) {
                        self.modal = Some(Modal::Info(format!("Konnte {} nicht öffnen:\n{}", p.display(), e)));
                    }
                }
            }
            Action::Save => self.persist(ctx, Action::Save, false, LogicPolicy::Ask),
            Action::SaveAs => self.persist(ctx, Action::SaveAs, true, LogicPolicy::Ask),
            Action::Generate => self.persist(ctx, Action::Generate, false, LogicPolicy::Ask),
            Action::Run => self.persist(ctx, Action::Run, false, LogicPolicy::Ask),
            Action::ExportExe => self.persist(ctx, Action::ExportExe, false, LogicPolicy::Ask),
            Action::Stop => self.stop_all(),
            Action::Quit => {
                self.stop_all();
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Action::ToggleView => {
                self.st.view = if self.st.view == View::Form { View::Code } else { View::Form };
            }
            Action::Undo => self.st.cmd_undo(),
            Action::Redo => self.st.cmd_redo(),
            Action::Duplicate => self.st.cmd_duplicate(),
            Action::Delete => self.st.cmd_delete(),
            Action::ZFront => self.st.cmd_zorder(true),
            Action::ZBack => self.st.cmd_zorder(false),
            Action::ChoosePython => {
                if let Some(p) = rfd::FileDialog::new().set_title("Python-Interpreter wählen").pick_file() {
                    self.python = Some((p.to_string_lossy().to_string(), vec![]));
                    self.st.log(format!("Python: {}", p.display()), LogKind::Info);
                }
            }
            Action::CenterH => self.st.cmd_center(true),
            Action::CenterV => self.st.cmd_center(false),
            Action::Help => self.modal = Some(Modal::Info(HELP_TEXT.to_string())),
        }
    }

    /// Speichern / Erzeugen / Starten mit allen Rueckfragen.
    fn persist(&mut self, ctx: &egui::Context, what: Action, force_dialog: bool, policy: LogicPolicy) {
        let _ = ctx;
        if self.st.path.is_none() || force_dialog {
            let name = self.st.path.as_ref().and_then(|p| p.file_name()).and_then(|s| s.to_str()).unwrap_or("Projekt1.pyphi").to_string();
            let picked = rfd::FileDialog::new()
                .set_title("Projekt speichern unter")
                .add_filter("Pyphi7-Projekt", &["pyphi", "pyrad"])
                .set_file_name(name)
                .save_file();
            match picked {
                Some(mut p) => {
                    if p.extension().is_none() {
                        p.set_extension("pyphi");
                    }
                    self.st.set_path(p);
                }
                None => return,
            }
        }
        let result = if matches!(what, Action::Save | Action::SaveAs) {
            self.st.save(policy).map(|r| match r {
                SaveResult::Saved => GenResult::Done(vec![]),
                SaveResult::Conflict => GenResult::Conflict,
            })
        } else {
            self.st.generate(policy)
        };
        match result {
            Ok(GenResult::Done(_)) => {
                if what == Action::Run {
                    self.start_all();
                } else if what == Action::ExportExe {
                    self.export_exe();
                }
            }
            Ok(GenResult::Conflict) => self.modal = Some(Modal::LogicConflict(what)),
            Err(e) => self.modal = Some(Modal::Info(e)),
        }
    }

    // -------------------------------------------------------------- Programme starten
    fn find_python(&mut self) -> Option<(String, Vec<String>)> {
        if let Some(p) = &self.python {
            return Some(p.clone());
        }
        let candidates: Vec<(&str, Vec<&str>)> = if cfg!(windows) {
            vec![("py", vec!["-3"]), ("python", vec![]), ("python3", vec![])]
        } else {
            vec![("python3", vec![]), ("python", vec![])]
        };
        for (prog, pre) in candidates {
            let mut cmd = Command::new(prog);
            cmd.args(&pre).arg("--version").stdout(Stdio::null()).stderr(Stdio::null());
            hide_console(&mut cmd);
            if cmd.status().map(|s| s.success()).unwrap_or(false) {
                let found = (prog.to_string(), pre.iter().map(|s| s.to_string()).collect::<Vec<_>>());
                self.python = Some(found.clone());
                return Some(found);
            }
        }
        None
    }

    fn start_all(&mut self) {
        self.stop_all();
        let Some((folder, stem)) = self.st.folder().zip(self.st.stem()) else { return };
        let Some((prog, pre)) = self.find_python() else {
            self.st.log(
                "Python wurde nicht gefunden. Installiere Python (python.org) oder wähle ihn unter Start > Python-Interpreter wählen.",
                LogKind::Err,
            );
            return;
        };
        for tk in self.st.toolkits() {
            let script = format!("{}{}", stem, tk.file_suffix());
            self.st.log(format!("\u{25b6} Starte {}: {}", tk.label(), script), LogKind::Info);
            self.spawn(&prog, &pre, &folder, &script, tk);
        }
    }

    fn spawn(&mut self, prog: &str, pre: &[String], folder: &PathBuf, script: &str, tk: Toolkit) {
        let mut cmd = Command::new(prog);
        cmd.args(pre)
            .arg("-u")
            .arg(script)
            .current_dir(folder)
            .env("PYTHONUNBUFFERED", "1")
            .env("PYTHONIOENCODING", "utf-8")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut cmd);
        self.run_cmd(cmd, tk.label().to_string(), true);
    }

    /// Startet einen Prozess, leitet seine Ausgabe in die Meldungen und merkt ihn sich.
    fn run_cmd(&mut self, mut cmd: Command, label: String, track: bool) {
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                self.st.log(format!("Start fehlgeschlagen: {}", e), LogKind::Err);
                return;
            }
        };
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let child = Arc::new(Mutex::new(child));
        if track {
            self.procs.push(Proc { child: child.clone() });
        }
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let mut readers = Vec::new();
            for stream in [stdout.map(|s| Box::new(s) as Box<dyn std::io::Read + Send>), stderr.map(|s| Box::new(s) as Box<dyn std::io::Read + Send>)]
                .into_iter()
                .flatten()
            {
                let tx = tx.clone();
                readers.push(std::thread::spawn(move || {
                    let mut r = BufReader::new(stream);
                    let mut buf = Vec::new();
                    let mut hinted = false;
                    loop {
                        buf.clear();
                        match r.read_until(b'\n', &mut buf) {
                            Ok(0) | Err(_) => break,
                            Ok(_) => {
                                let line = String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string();
                                let kind = if line.contains("Traceback") || line.contains("Error") { LogKind::Err } else { LogKind::Normal };
                                if line.contains("No module named 'PySide6'") && !hinted {
                                    hinted = true;
                                    let _ = tx.send((line.clone(), kind));
                                    let _ = tx.send(("Tipp: Qt braucht PySide6 ->  pip install PySide6".into(), LogKind::Info));
                                } else {
                                    let _ = tx.send((line, kind));
                                }
                            }
                        }
                    }
                }));
            }
            for r in readers {
                let _ = r.join();
            }
            let code = loop {
                match child.lock().map(|mut c| c.try_wait()) {
                    Ok(Ok(Some(status))) => break status.code(),
                    Ok(Ok(None)) => {}
                    _ => break None,
                }
                std::thread::sleep(std::time::Duration::from_millis(40));
            };
            let _ = tx.send((format!("[{} beendet, Code {}]", label, code.map(|c| c.to_string()).unwrap_or("?".into())), LogKind::Info));
        });
    }

    // -------------------------------------------------------------- Bibliotheken (pip)
    fn refresh_libs(&mut self) {
        let Some((prog, pre)) = self.find_python() else {
            self.libs_state = LibsState::Failed;
            return;
        };
        self.libs_state = LibsState::Loading;
        let tx = self.lib_tx.clone();
        std::thread::spawn(move || {
            let mut cmd = Command::new(prog);
            cmd.args(&pre)
                .args(["-m", "pip", "list", "--format=json", "--disable-pip-version-check"])
                .env("PYTHONIOENCODING", "utf-8")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            hide_console(&mut cmd);
            let res = match cmd.output() {
                Ok(o) if o.status.success() => libs::parse_pip_list(&String::from_utf8_lossy(&o.stdout)),
                Ok(o) => Err(String::from_utf8_lossy(&o.stderr).lines().last().unwrap_or("pip meldet einen Fehler").to_string()),
                Err(e) => Err(e.to_string()),
            };
            let _ = tx.send(res);
        });
    }

    /// pip install / install --upgrade / uninstall; Ausgabe landet in den Meldungen.
    fn run_pip(&mut self, verb: &str, specs: &[String]) {
        if self.pip_busy {
            return;
        }
        let Some((prog, pre)) = self.find_python() else {
            self.st.log("Python wurde nicht gefunden (Start > Python-Interpreter wählen).", LogKind::Err);
            return;
        };
        let mut cmd = Command::new(prog);
        cmd.args(&pre).args(["-m", "pip"]);
        match verb {
            "install" => cmd.arg("install"),
            "upgrade" => cmd.args(["install", "--upgrade"]),
            _ => cmd.args(["uninstall", "-y"]),
        };
        cmd.arg("--disable-pip-version-check")
            .args(specs)
            .env("PYTHONUNBUFFERED", "1")
            .env("PYTHONIOENCODING", "utf-8")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        hide_console(&mut cmd);
        let what = match verb {
            "install" => "Installiere",
            "upgrade" => "Aktualisiere",
            _ => "Entferne",
        };
        self.st.log(format!("\u{25b6} {} {} ...", what, specs.join(" ")), LogKind::Info);
        self.pip_busy = true;
        self.run_cmd(cmd, "pip".to_string(), false);
    }

    fn libs_view(&mut self, ui: &mut Ui) {
        if self.libs_state == LibsState::Unknown {
            self.refresh_libs();
        }
        let installed = |libs: &Vec<(String, String, String)>, pkg: &str| -> Option<String> {
            let n = libs::norm(&libs::base_name(pkg));
            libs.iter().find(|e| e.0 == n).map(|e| e.2.clone())
        };
        let mut todo: Option<(&'static str, Vec<String>)> = None;
        let mut reload = false;
        egui::Frame::NONE.fill(WHITE).inner_margin(egui::Margin::same(10)).show(ui, |ui| {
            ui.heading("Zusatz-Bibliotheken (pip)");
            ui.label("Pakete werden in den Python-Interpreter installiert, mit dem Pyphi7 deine Programme startet.");
            ui.horizontal(|ui| {
                let py = self.python.as_ref().map(|(p, pre)| format!("{} {}", p, pre.join(" ")).trim().to_string()).unwrap_or_else(|| "(wird gesucht)".into());
                ui.label(format!("Python: {}", py));
                if ui.button("Liste aktualisieren").clicked() {
                    reload = true;
                }
                match self.libs_state {
                    LibsState::Loading => {
                        ui.spinner();
                    }
                    LibsState::Failed => {
                        ui.colored_label(Color32::from_rgb(0xa0, 0, 0), "pip nicht erreichbar");
                    }
                    _ => {}
                }
                if self.pip_busy {
                    ui.spinner();
                    ui.label("pip arbeitet – Ausgabe siehe Meldungen");
                }
            });
            ui.separator();
            ui.label("Empfohlen");
            let busy = self.pip_busy;
            egui::Grid::new("libs_grid").num_columns(4).spacing([14.0, 6.0]).striped(true).show(ui, |ui| {
                for l in libs::CURATED {
                    let ver = installed(&self.libs, l.pkg);
                    ui.label(egui::RichText::new(l.pkg).strong());
                    match &ver {
                        Some(v) => ui.colored_label(Color32::from_rgb(0, 0x70, 0), format!("installiert {}", v)),
                        None => ui.colored_label(GRAY, "nicht installiert"),
                    };
                    ui.label(l.info);
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(!busy, |ui| {
                            if ver.is_none() {
                                if ui.button("Installieren").clicked() {
                                    todo = Some(("install", vec![l.pkg.to_string()]));
                                }
                            } else {
                                if ui.button("Aktualisieren").clicked() {
                                    todo = Some(("upgrade", vec![l.pkg.to_string()]));
                                }
                                if ui.button("Entfernen").clicked() {
                                    todo = Some(("uninstall", vec![l.pkg.to_string()]));
                                }
                            }
                        });
                    });
                    ui.end_row();
                }
            });
            ui.add_space(6.0);
            ui.label("Schon in Python enthalten (nichts zu installieren)");
            egui::Grid::new("libs_builtin").num_columns(3).spacing([14.0, 4.0]).striped(true).show(ui, |ui| {
                for l in libs::BUILTIN {
                    ui.label(egui::RichText::new(l.pkg).strong());
                    ui.colored_label(Color32::from_rgb(0, 0x70, 0), "eingebaut");
                    ui.label(l.info);
                    ui.end_row();
                }
            });
            ui.add_space(4.0);
            ui.separator();
            ui.label("Anderes Paket (Name wie bei pip, mehrere mit Leerzeichen)");
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut self.lib_input).desired_width(260.0).hint_text("z. B. beautifulsoup4"));
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                ui.add_enabled_ui(!busy, |ui| {
                    if ui.button("Installieren").clicked() || (enter && !busy) {
                        match libs::parse_specs(&self.lib_input) {
                            Ok(v) => todo = Some(("install", v)),
                            Err(e) => self.modal = Some(Modal::Info(e)),
                        }
                    }
                    if ui.button("Entfernen").clicked() {
                        match libs::parse_specs(&self.lib_input) {
                            Ok(v) => todo = Some(("uninstall", v.iter().map(|s| libs::base_name(s)).collect())),
                            Err(e) => self.modal = Some(Modal::Info(e)),
                        }
                    }
                });
            });
            ui.add_space(6.0);
            ui.collapsing(format!("Alle installierten Pakete ({})", self.libs.len()), |ui| {
                egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                    egui::Grid::new("libs_all").num_columns(2).spacing([20.0, 2.0]).show(ui, |ui| {
                        for (_, name, ver) in &self.libs {
                            ui.label(name);
                            ui.label(ver);
                            ui.end_row();
                        }
                    });
                });
            });
        });
        if reload {
            self.refresh_libs();
        }
        if let Some((verb, specs)) = todo {
            self.run_pip(verb, &specs);
        }
    }

    /// Baut aus den erzeugten .py-Dateien mit PyInstaller eine einzelne EXE (pro Toolkit eine).
    fn export_exe(&mut self) {
        self.stop_all();
        let Some((folder, stem)) = self.st.folder().zip(self.st.stem()) else { return };
        let Some((prog, pre)) = self.find_python() else {
            self.st.log("Python wurde nicht gefunden (Start > Python-Interpreter wählen).", LogKind::Err);
            return;
        };
        // PyInstaller vorhanden?
        let mut chk = Command::new(&prog);
        chk.args(&pre).args(["-m", "PyInstaller", "--version"]).stdout(Stdio::null()).stderr(Stdio::null());
        hide_console(&mut chk);
        if !chk.status().map(|s| s.success()).unwrap_or(false) {
            self.st.log("PyInstaller fehlt – unter „Bibliotheken…“ installieren.", LogKind::Err);
            self.libs_open = true;
            self.modal = Some(Modal::Info(
                "Für den EXE-Export wird PyInstaller gebraucht.\n\nIm Fenster „Bibliotheken“ (öffnet sich gleich) bei pyinstaller auf „Installieren“ klicken, danach Projekt > Als EXE exportieren erneut wählen.".into(),
            ));
            return;
        }
        let sep = if cfg!(windows) { ";" } else { ":" };
        let mut pics: Vec<String> = self
            .st
            .proj
            .widgets
            .iter()
            .map(|w| w.prop("picture"))
            .filter(|p| !p.is_empty() && !std::path::Path::new(p).is_absolute())
            .collect();
        pics.sort();
        pics.dedup();
        let mut started = false;
        for tk in self.st.toolkits() {
            let script = format!("{}{}", stem, tk.file_suffix());
            let script_path = folder.join(&script);
            let name = script.trim_end_matches(".py").to_string();
            let mut cmd = Command::new(&prog);
            cmd.args(&pre)
                .args(["-m", "PyInstaller", "--noconfirm", "--clean", "--onefile", "--windowed"])
                .arg("--name")
                .arg(&name)
                .arg("--distpath")
                .arg(folder.join("dist"))
                .arg("--workpath")
                .arg(folder.join("build").join(&name))
                .arg("--specpath")
                .arg(folder.join("build"));
            for pic in &pics {
                let src = folder.join(pic);
                if !src.exists() {
                    self.st.log(format!("Bild nicht gefunden (wird nicht eingebaut): {}", pic), LogKind::Err);
                    continue;
                }
                let dest = std::path::Path::new(pic).parent().map(|d| d.to_string_lossy().to_string()).filter(|d| !d.is_empty()).unwrap_or_else(|| ".".into());
                cmd.arg("--add-data").arg(format!("{}{}{}", src.display(), sep, dest));
            }
            cmd.arg(&script_path)
                .current_dir(&folder)
                .env("PYTHONUNBUFFERED", "1")
                .env("PYTHONIOENCODING", "utf-8")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            hide_console(&mut cmd);
            self.st.log(format!("\u{25b6} Erzeuge EXE ({}) – das dauert eine Weile...", tk.label()), LogKind::Info);
            self.run_cmd(cmd, format!("EXE {}", tk.label()), true);
            started = true;
        }
        if started {
            let ext = if cfg!(windows) { ".exe" } else { "" };
            self.st.log(
                format!("Ergebnis: {}{}{}", folder.join("dist").display(), std::path::MAIN_SEPARATOR, format!("<name>{}", ext)),
                LogKind::Info,
            );
        }
    }

    fn stop_all(&mut self) {
        for p in self.procs.drain(..) {
            if let Ok(mut c) = p.child.lock() {
                let _ = c.kill();
            }
        }
    }

    // -------------------------------------------------------------- Oberflaeche
    fn menu_bar(&mut self, ui: &mut Ui) {
        let mut act: Option<Action> = None;
        egui::MenuBar::new().ui(ui, |ui| {
            let item = |ui: &mut Ui, label: &str, short: &str, a: Action, act: &mut Option<Action>| {
                let mut b = egui::Button::new(label);
                if !short.is_empty() {
                    b = b.shortcut_text(short);
                }
                if ui.add(b).clicked() {
                    *act = Some(a);
                    ui.close();
                }
            };
            ui.menu_button("Datei", |ui| {
                item(ui, "Neu", "Strg+N", Action::New, &mut act);
                item(ui, "Öffnen...", "Strg+O", Action::Open, &mut act);
                item(ui, "Speichern", "Strg+S", Action::Save, &mut act);
                item(ui, "Speichern unter...", "", Action::SaveAs, &mut act);
                ui.separator();
                item(ui, "Beenden", "Alt+F4", Action::Quit, &mut act);
            });
            ui.menu_button("Bearbeiten", |ui| {
                item(ui, "Rückgängig", "Strg+Z", Action::Undo, &mut act);
                item(ui, "Wiederholen", "Strg+Y", Action::Redo, &mut act);
                ui.separator();
                item(ui, "Duplizieren", "Strg+D", Action::Duplicate, &mut act);
                item(ui, "Löschen", "Entf", Action::Delete, &mut act);
                item(ui, "Nach vorne", "", Action::ZFront, &mut act);
                item(ui, "Nach hinten", "", Action::ZBack, &mut act);
                ui.separator();
                item(ui, "Waagrecht zentrieren", "", Action::CenterH, &mut act);
                item(ui, "Senkrecht zentrieren", "", Action::CenterV, &mut act);
            });
            ui.menu_button("Ansicht", |ui| {
                item(ui, "Formular/Code umschalten", "F12", Action::ToggleView, &mut act);
                ui.separator();
                ui.checkbox(&mut self.st.show_grid, "Raster anzeigen");
                ui.checkbox(&mut self.st.snap, "Am Raster ausrichten");
            });
            ui.menu_button("Projekt", |ui| {
                item(ui, "Code erzeugen", "Strg+F9", Action::Generate, &mut act);
                ui.separator();
                item(ui, "Als EXE exportieren...", "", Action::ExportExe, &mut act);
            });
            ui.menu_button("Start", |ui| {
                item(ui, "Starten", "F9", Action::Run, &mut act);
                item(ui, "Programm beenden", "Strg+F2", Action::Stop, &mut act);
                ui.separator();
                item(ui, "Python-Interpreter wählen...", "", Action::ChoosePython, &mut act);
            });
            ui.menu_button("Hilfe", |ui| {
                item(ui, "Kurzanleitung", "", Action::Help, &mut act);
            });
        });
        if let Some(a) = act {
            self.queue.push(a);
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        let font = FontId::proportional(13.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    for (label, a, hint) in [
                        ("Neu", Action::New, "Neues Projekt (Strg+N)"),
                        ("Öffnen", Action::Open, "Projekt öffnen (Strg+O)"),
                        ("Speichern", Action::Save, "Alles speichern (Strg+S)"),
                    ] {
                        let w = 14.0 + ui.painter().layout_no_wrap(label.into(), font.clone(), BLACK).size().x;
                        let r = tool_button(ui, vec2(w, 24.0), false, |p, rc| {
                            p.text(rc.center(), Align2::CENTER_CENTER, label, font.clone(), BLACK);
                        });
                        if r.clicked() {
                            self.queue.push(a);
                        }
                        if r.hovered() {
                            self.hint = hint.into();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    let r = tool_button(ui, vec2(62.0, 24.0), false, |p, rc| {
                        let c = pos2(rc.min.x + 14.0, rc.center().y);
                        p.add(egui::Shape::convex_polygon(
                            vec![c + vec2(-4.0, -6.0), c + vec2(-4.0, 6.0), c + vec2(5.0, 0.0)],
                            Color32::from_rgb(0x00, 0x80, 0x00),
                            Stroke::new(1.0, BLACK),
                        ));
                        p.text(pos2(rc.min.x + 22.0, rc.center().y), Align2::LEFT_CENTER, "Start", font.clone(), BLACK);
                    });
                    if r.clicked() {
                        self.queue.push(Action::Run);
                    }
                    if r.hovered() {
                        self.hint = "Code erzeugen und starten (F9)".into();
                    }
                    let r = tool_button(ui, vec2(26.0, 24.0), false, |p, rc| {
                        let c = rc.center();
                        fill(p, c.x - 5.0, c.y - 5.0, c.x + 4.0, c.y + 4.0, Color32::from_rgb(0xa0, 0x00, 0x00));
                    });
                    if r.clicked() {
                        self.queue.push(Action::Stop);
                    }
                    if r.hovered() {
                        self.hint = "Programm beenden (Strg+F2)".into();
                    }
                    ui.add_space(8.0);
                    ui.label("Toolkit:");
                    let names = ["Tkinter", "Qt (PySide6)", "Beide"];
                    egui::ComboBox::from_id_salt("toolkit")
                        .width(110.0)
                        .selected_text(names[self.st.toolkit_choice])
                        .show_ui(ui, |ui| {
                            for (i, n) in names.iter().enumerate() {
                                ui.selectable_value(&mut self.st.toolkit_choice, i, *n);
                            }
                        });
                });
            });
            ui.add_space(10.0);
            ui.vertical(|ui| {
                // Registerkarten der Komponentenpalette (Standard / Zusätzlich / System / Daten)
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    for (i, (name, _)) in model::PALETTE.iter().enumerate() {
                        let tw = ui.painter().layout_no_wrap(name.to_string(), font.clone(), BLACK).size().x;
                        let (r, resp) = ui.allocate_exact_size(vec2(tw + 18.0, 20.0), Sense::click());
                        let active = self.pal_tab == i;
                        let top = if active { r.min.y } else { r.min.y + 2.0 };
                        let p = ui.painter();
                        bevel(p, r.min.x, top, r.max.x - 1.0, r.max.y + 1.0, false, FACE);
                        p.text(pos2(r.center().x, (top + r.max.y) / 2.0), Align2::CENTER_CENTER, *name, font.clone(), BLACK);
                        if resp.clicked() {
                            self.pal_tab = i;
                        }
                    }
                    // "Bibliotheken": kein Reiter, sondern ein Knopf, der die Paketverwaltung oeffnet
                    ui.add_space(8.0);
                    let tw = ui.painter().layout_no_wrap("Bibliotheken…".to_string(), font.clone(), BLACK).size().x;
                    let (r, resp) = ui.allocate_exact_size(vec2(tw + 18.0, 20.0), Sense::click());
                    let pressed = resp.is_pointer_button_down_on() || self.libs_open;
                    let p = ui.painter();
                    bevel(p, r.min.x, r.min.y + 1.0, r.max.x - 1.0, r.max.y + 1.0, pressed, FACE);
                    p.text(pos2(r.center().x, r.center().y + 1.0), Align2::CENTER_CENTER, "Bibliotheken…", font.clone(), BLACK);
                    if resp.clicked() {
                        self.libs_open = true;
                    }
                    resp.on_hover_text("Python-Bibliotheken installieren, aktualisieren, entfernen");
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                    let r = tool_button(ui, vec2(34.0, 30.0), self.st.tool.is_none(), |p, rc| {
                        paint_icon(p, rc, None, &font);
                    });
                    if r.clicked() {
                        self.st.tool = None;
                    }
                    if r.hovered() {
                        self.hint = "Auswahl-Werkzeug".into();
                    }
                    for &wt in model::PALETTE[self.pal_tab.min(model::PALETTE.len() - 1)].1 {
                        let sel = self.st.tool == Some(wt);
                        let r = tool_button(ui, vec2(34.0, 30.0), sel, |p, rc| {
                            paint_icon(p, rc, Some(wt), &font);
                        });
                        if r.clicked() {
                            self.st.tool = if sel { None } else { Some(wt) };
                        }
                        if r.hovered() {
                            self.hint = wt.info().hint.into();
                        }
                    }
                });
            });
        });
    }

    /// Im Code-Editor ersetzt diese Uebersicht den Objektinspektor: alle Objekte mit ihren
    /// Funktionen, ein Klick springt (bzw. legt die Funktion an).
    fn code_outline(&mut self, ui: &mut Ui) {
        enum Go {
            Def(String),
            Create(Option<usize>, &'static str),
        }
        let mut go: Option<Go> = None;
        let logic = self.st.logic.clone();
        let has = |n: &str| codegen::has_def(&logic, n);
        let dim = Color32::from_rgb(0x80, 0x80, 0x80);
        let entry = |ui: &mut Ui, go: &mut Option<Go>, label: &str, ev: &str, func: &str, target: Option<usize>, ev_key: &'static str| {
            let exists = !func.is_empty() && has(func);
            let text = if func.is_empty() {
                egui::RichText::new(format!("{}  (neu)", ev)).color(dim)
            } else if exists {
                egui::RichText::new(format!("{}  >  {}", ev, func)).strong()
            } else {
                egui::RichText::new(format!("{}  >  {}  (fehlt)", ev, func)).color(Color32::from_rgb(0xa0, 0, 0))
            };
            let _ = label;
            if ui.add(egui::Button::new(text).frame(false)).clicked() {
                *go = Some(if func.is_empty() { Go::Create(target, ev_key) } else { Go::Def(func.to_string()) });
            }
        };
        ui.add_space(4.0);
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.label(egui::RichText::new("Objekte und Funktionen").strong());
            egui::CollapsingHeader::new(format!("{}: TForm", FORM_CLASS)).id_salt("ol_form").default_open(true).show(ui, |ui| {
                let (c, cl) = (self.st.proj.on_create.clone(), self.st.proj.on_close.clone());
                entry(ui, &mut go, "", "OnCreate", &c, None, "OnCreate");
                entry(ui, &mut go, "", "OnClose", &cl, None, "OnClose");
            });
            for i in 0..self.st.proj.widgets.len() {
                let wd = &self.st.proj.widgets[i];
                let head = format!("{}: {}", wd.name, wd.wtype.info().cls);
                let evs: Vec<(&'static str, String)> = wd.wtype.events().iter().map(|e| (e.name, wd.event_fn(e.name))).collect();
                if evs.is_empty() {
                    continue;
                }
                let any = evs.iter().any(|(_, f)| !f.is_empty());
                egui::CollapsingHeader::new(if any { egui::RichText::new(head).strong() } else { egui::RichText::new(head) }).id_salt(("ol_w", i)).default_open(any).show(ui, |ui| {
                    for (name, func) in &evs {
                        entry(ui, &mut go, "", name, func, Some(i), name);
                    }
                });
            }
            // eigene Funktionen (alles, was nicht zu einem Ereignis gehoert)
            let handlers = codegen::handler_names(&self.st.proj);
            let own: Vec<String> = logic
                .lines()
                .filter_map(|l| {
                    let t = l.strip_prefix("def ")?;
                    let n: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    if n.is_empty() || handlers.contains(&n) { None } else { Some(n) }
                })
                .collect();
            if !own.is_empty() {
                ui.add_space(6.0);
                egui::CollapsingHeader::new("Eigene Funktionen").id_salt("ol_own").default_open(true).show(ui, |ui| {
                    for n in &own {
                        if ui.add(egui::Button::new(egui::RichText::new(format!("def {}", n))).frame(false)).clicked() {
                            go = Some(Go::Def(n.clone()));
                        }
                    }
                });
            }
        });
        match go {
            Some(Go::Def(n)) => {
                self.st.ensure_stub(&n);
                self.st.goto_def(&n);
            }
            Some(Go::Create(t, ev)) => self.st.create_event(t, Some(ev)),
            None => {}
        }
    }

    fn inspector(&mut self, ui: &mut Ui) {
        let font = FontId::proportional(13.0);
        let (title_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
        let p = ui.painter();
        fill(p, title_rect.min.x, title_rect.min.y, title_rect.max.x - 1.0, title_rect.max.y - 1.0, NAVY);
        let t = "Objektinspektor";
        p.text(title_rect.min + vec2(5.0, 10.0), Align2::LEFT_CENTER, t, font.clone(), WHITE);
        p.text(title_rect.min + vec2(6.0, 10.0), Align2::LEFT_CENTER, t, font.clone(), WHITE);

        if self.st.view == View::Code {
            self.code_outline(ui);
            return;
        }

        // Objektauswahl
        let ws_len = self.st.proj.widgets.len();
        let cur_text = match self.st.selected() {
            None => format!("{}: TForm", FORM_CLASS),
            Some(w) => format!("{}: {}", w.name, w.wtype.info().cls),
        };
        let mut new_sel: Option<Option<usize>> = None;
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        egui::ComboBox::from_id_salt("objsel")
            .width(ui.available_width() - 8.0)
            .selected_text(cur_text)
            .show_ui(ui, |ui| {
                if ui.selectable_label(self.st.sel.is_none(), format!("{}: TForm", FORM_CLASS)).clicked() {
                    new_sel = Some(None);
                }
                for i in 0..ws_len {
                    let w = &self.st.proj.widgets[i];
                    let label = format!("{}: {}", w.name, w.wtype.info().cls);
                    if ui.selectable_label(self.st.sel == Some(i), label).clicked() {
                        new_sel = Some(Some(i));
                    }
                }
            });
        if let Some(s) = new_sel {
            self.commit_edit();
            self.st.select(s);
        }

        // Registerkarten Eigenschaften / Ereignisse
        ui.horizontal(|ui| {
            for (i, name) in ["Eigenschaften", "Ereignisse"].iter().enumerate() {
                if ui.selectable_label(self.insp_tab == i, *name).clicked() {
                    self.commit_edit();
                    self.insp_tab = i;
                }
            }
        });

        let total_w = ui.available_width();
        let label_w = 112.0;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            if self.insp_tab == 0 {
                let rows = self.st.prop_rows();
                for row in rows {
                    let (rect, _) = ui.allocate_exact_size(vec2(total_w - 4.0, 20.0), Sense::hover());
                    let p = ui.painter();
                    fill(p, rect.min.x, rect.min.y, rect.max.x - 1.0, rect.max.y - 1.0, WHITE);
                    hline(p, rect.min.x, rect.max.x, rect.max.y - 1.0, Color32::from_rgb(0xe0, 0xe0, 0xe0));
                    vline(p, rect.min.x + label_w, rect.min.y, rect.max.y - 1.0, Color32::from_rgb(0xc0, 0xc0, 0xc0));
                    p.text(rect.min + vec2(4.0, 10.0), Align2::LEFT_CENTER, &row.label, font.clone(), BLACK);
                    let mut vrect = Rect::from_min_max(rect.min + vec2(label_w + 2.0, 0.0), rect.max);
                    let has_btn = matches!(row.kind, PropKind::Items | PropKind::File | PropKind::Color);
                    let btn_rect = Rect::from_min_max(pos2(rect.max.x - 24.0, rect.min.y + 1.0), pos2(rect.max.x - 2.0, rect.max.y - 2.0));
                    if has_btn {
                        vrect.max.x = btn_rect.min.x - 1.0;
                    }
                    let key = row.key.clone();
                    match row.kind {
                        PropKind::Bool => {
                            let r = ui.interact(vrect, ui.id().with(("b", &key)), Sense::click());
                            ui.painter().text(vrect.min + vec2(3.0, 10.0), Align2::LEFT_CENTER, &row.value, font.clone(), BLACK);
                            if r.clicked() {
                                self.commit_edit();
                                let v = if row.value == "True" { "False" } else { "True" };
                                if let Err(e) = self.st.set_prop(&key, v) {
                                    self.modal = Some(Modal::Info(e));
                                }
                            }
                        }
                        PropKind::Choice(opts) => {
                            let mut picked: Option<String> = None;
                            ui.scope_builder(egui::UiBuilder::new().max_rect(vrect), |ui| {
                                egui::ComboBox::from_id_salt(("ch", &key))
                                    .width((vrect.width() - 34.0).max(40.0))
                                    .selected_text(row.value.clone())
                                    .show_ui(ui, |ui| {
                                        for o in opts {
                                            if ui.selectable_label(row.value == *o, *o).clicked() {
                                                picked = Some(o.to_string());
                                            }
                                        }
                                    });
                            });
                            if let Some(v) = picked {
                                self.commit_edit();
                                if let Err(e) = self.st.set_prop(&key, &v) {
                                    self.modal = Some(Modal::Info(e));
                                }
                            }
                        }
                        _ => {
                            let id = ui.id().with(("e", &key));
                            let editing = self.insp_key.as_deref() == Some(key.as_str());
                            let mut tmp = row.value.clone();
                            let buf: &mut String = if editing { &mut self.insp_buf } else { &mut tmp };
                            let r = ui.put(
                                vrect,
                                egui::TextEdit::singleline(buf)
                                    .id(id)
                                    .font(font.clone())
                                    .frame(egui::Frame::NONE)
                                    .margin(egui::Margin::symmetric(3, 2))
                                    .vertical_align(Align::Center)
                                    .desired_width(vrect.width()),
                            );
                            if !editing && r.gained_focus() {
                                self.insp_key = Some(key.clone());
                                self.insp_buf = row.value.clone();
                            }
                            if editing && r.lost_focus() {
                                let escape = ui.input(|i| i.key_pressed(Key::Escape));
                                if escape {
                                    self.insp_key = None;
                                } else {
                                    self.commit_edit();
                                }
                            }
                            match row.kind {
                                PropKind::Items => {
                                    let r = ui.put(btn_rect, egui::Button::new("…"));
                                    if r.on_hover_text("Zeilen-Editor öffnen").clicked() {
                                        self.commit_edit();
                                        if let Some(wd) = self.st.selected() {
                                            let list = wd.wtype.is_list();
                                            let text = if list { model::items_of(wd).join("\n") } else { wd.text.clone() };
                                            self.modal = Some(Modal::Items { text, list });
                                        }
                                    }
                                }
                                PropKind::File => {
                                    let r = ui.put(btn_rect, egui::Button::new("…"));
                                    if r.on_hover_text("Datei wählen").clicked() {
                                        self.commit_edit();
                                        let mut dlg = rfd::FileDialog::new().set_title("Bild wählen").add_filter("Bilder", &["png", "gif", "jpg", "jpeg", "bmp", "webp"]);
                                        if let Some(f) = self.st.folder() {
                                            dlg = dlg.set_directory(f);
                                        }
                                        if let Some(pth) = dlg.pick_file() {
                                            let rel = self
                                                .st
                                                .folder()
                                                .and_then(|f| pth.canonicalize().ok().zip(f.canonicalize().ok()))
                                                .and_then(|(pp, ff)| pp.strip_prefix(&ff).ok().map(|x| x.to_string_lossy().replace('\\', "/")))
                                                .unwrap_or_else(|| pth.to_string_lossy().to_string());
                                            if let Err(e) = self.st.set_prop(&key, &rel) {
                                                self.modal = Some(Modal::Info(e));
                                            }
                                        }
                                    }
                                }
                                PropKind::Color => {
                                    let hex = row.value.trim().trim_start_matches('#').to_string();
                                    let mut rgb = [0xd4u8, 0xd0, 0xc8];
                                    if hex.len() == 6 {
                                        if let Ok(v) = u32::from_str_radix(&hex, 16) {
                                            rgb = [(v >> 16) as u8, (v >> 8) as u8, v as u8];
                                        }
                                    }
                                    let resp = ui
                                        .scope_builder(egui::UiBuilder::new().max_rect(btn_rect), |ui| {
                                            ui.spacing_mut().interact_size = vec2(20.0, 16.0);
                                            ui.spacing_mut().button_padding = vec2(0.0, 0.0);
                                            egui::color_picker::color_edit_button_srgb(ui, &mut rgb)
                                        })
                                        .inner;
                                    if resp.changed() {
                                        self.commit_edit();
                                        let v = format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2]);
                                        if let Err(e) = self.st.set_prop(&key, &v) {
                                            self.modal = Some(Modal::Info(e));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            } else {
                let rows = self.st.event_rows();
                if rows.is_empty() {
                    ui.add_space(6.0);
                    ui.label("Diese Komponente hat kein Ereignis.");
                }
                for (key, label, value) in rows {
                    let (rect, resp) = ui.allocate_exact_size(vec2(total_w - 4.0, 20.0), Sense::click());
                    let p = ui.painter();
                    fill(p, rect.min.x, rect.min.y, rect.max.x - 1.0, rect.max.y - 1.0, WHITE);
                    vline(p, rect.min.x + label_w, rect.min.y, rect.max.y - 1.0, Color32::from_rgb(0xc0, 0xc0, 0xc0));
                    p.text(rect.min + vec2(4.0, 10.0), Align2::LEFT_CENTER, &label, font.clone(), BLACK);
                    p.text(rect.min + vec2(label_w + 5.0, 10.0), Align2::LEFT_CENTER, &value, font.clone(), BLACK);
                    if resp.double_clicked() {
                        let sel = self.st.sel;
                        self.st.create_event(sel, Some(&key));
                    }
                }
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Doppelklick auf den Wert legt die\nFunktion an und springt in den Code.").weak());
            }
        });
    }

    /// Schliesst eine laufende Eingabe im Objektinspektor ab (uebernimmt den Wert).
    fn commit_edit(&mut self) {
        if let Some(key) = self.insp_key.take() {
            let original = self.st.prop_rows().into_iter().find(|r| r.key == key).map(|r| r.value);
            if original.as_deref() != Some(self.insp_buf.as_str()) {
                let buf = self.insp_buf.clone();
                if let Err(e) = self.st.set_prop(&key, &buf) {
                    self.modal = Some(Modal::Info(e));
                }
            }
        }
    }

    // -------------------------------------------------------------- Formular-Canvas
    fn form_view(&mut self, ui: &mut Ui) {
        let font = FontId::proportional(13.0);
        let (w, h) = (self.st.proj.width as f32, self.st.proj.height as f32);
        let need = vec2(OX + w + 40.0, OY + h + 40.0);
        egui::ScrollArea::both().auto_shrink([false, false]).id_salt("form_scroll").show(ui, |ui| {
            let avail = ui.available_size();
            let size = vec2(need.x.max(avail.x), need.y.max(avail.y));
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
            let org = rect.min + vec2(OX, OY);
            let painter = ui.painter_at(rect);

            // ---- Eingaben
            let ptr = ui.input(|i| i.pointer.latest_pos());
            let to_form = |p: egui::Pos2| (p.x - org.x, p.y - org.y);
            let (pressed, down, released) =
                ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down(), i.pointer.primary_released()));
            if let Some(pp) = ptr {
                let (fx, fy) = to_form(pp);
                if pressed && resp.hovered() {
                    self.commit_edit();
                    let o = ui.input(|i| i.pointer.press_origin()).unwrap_or(pp);
                    let (ox, oy) = to_form(o);
                    self.st.press(ox, oy);
                    self.st.drag(fx, fy);
                } else if down {
                    self.st.drag(fx, fy);
                }
                if released {
                    self.st.release();
                }
                if resp.double_clicked() {
                    self.st.double(fx, fy);
                }
                if resp.secondary_clicked() {
                    let hit = self.st.hit_widget(fx, fy);
                    self.st.select(hit);
                }
                // Mauszeiger / Statuszeile
                if resp.hovered() {
                    let icon = if self.st.tool.is_some() {
                        CursorIcon::Crosshair
                    } else if let Some(hd) = self.st.hit_handle(fx, fy) {
                        match hd {
                            Handle::NW | Handle::SE => CursorIcon::ResizeNwSe,
                            Handle::NE | Handle::SW => CursorIcon::ResizeNeSw,
                            Handle::N | Handle::S => CursorIcon::ResizeVertical,
                            Handle::E | Handle::W => CursorIcon::ResizeHorizontal,
                        }
                    } else if self.st.hit_widget(fx, fy).is_some() {
                        CursorIcon::Move
                    } else if let Some(edge) = self.st.hit_form_edge(fx, fy) {
                        match edge {
                            FormEdge::E => CursorIcon::ResizeHorizontal,
                            FormEdge::S => CursorIcon::ResizeVertical,
                            FormEdge::SE => CursorIcon::ResizeNwSe,
                        }
                    } else {
                        CursorIcon::Default
                    };
                    ui.ctx().set_cursor_icon(icon);
                    if fx >= 0.0 && fy >= 0.0 && fx < w && fy < h {
                        self.hint = format!("Left: {}   Top: {}", fx as i32, fy as i32);
                    }
                }
            } else if released {
                self.st.release();
            }
            resp.context_menu(|ui| {
                if let Some(i) = self.st.sel {
                    if self.st.proj.widgets[i].wtype.info().event.is_some() && ui.button("Ereignis-Funktion anlegen / öffnen").clicked() {
                        self.st.create_event(Some(i), None);
                        ui.close();
                    }
                    if ui.button("Duplizieren").clicked() {
                        self.queue.push(Action::Duplicate);
                        ui.close();
                    }
                    if ui.button("Nach vorne").clicked() {
                        self.queue.push(Action::ZFront);
                        ui.close();
                    }
                    if ui.button("Nach hinten").clicked() {
                        self.queue.push(Action::ZBack);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Löschen").clicked() {
                        self.queue.push(Action::Delete);
                        ui.close();
                    }
                } else {
                    if ui.button("OnCreate-Funktion anlegen / öffnen").clicked() {
                        self.st.create_event(None, None);
                        ui.close();
                    }
                    if ui.button("OnClose-Funktion anlegen / öffnen").clicked() {
                        self.st.create_event(None, Some("OnClose"));
                        ui.close();
                    }
                }
            });

            // ---- Zeichnen
            painter.rect_filled(rect, CornerRadius::ZERO, WORKSPACE);
            let (fx0, fy0) = (org.x.round(), org.y.round());
            // Rahmen + Titelleiste
            bevel(&painter, fx0 - 4.0, fy0 - TITLE_H - 4.0, fx0 + w + 3.0, fy0 + h + 3.0, false, FACE);
            fill(&painter, fx0, fy0 - TITLE_H, fx0 + w - 1.0, fy0 - 1.0, NAVY);
            let cap = &self.st.proj.title;
            painter.text(pos2(fx0 + 6.0, fy0 - TITLE_H / 2.0), Align2::LEFT_CENTER, cap, font.clone(), WHITE);
            painter.text(pos2(fx0 + 7.0, fy0 - TITLE_H / 2.0), Align2::LEFT_CENTER, cap, font.clone(), WHITE);
            let cx = fx0 + w - 19.0;
            for (k, bx) in [cx - 34.0, cx - 18.0, cx].into_iter().enumerate() {
                bevel(&painter, bx, fy0 - TITLE_H + 3.0, bx + 15.0, fy0 - 4.0, false, FACE);
                let c = pos2(bx + 7.5, fy0 - TITLE_H / 2.0 + 0.5);
                match k {
                    0 => hline(&painter, c.x - 3.0, c.x + 3.0, c.y + 3.0, BLACK),
                    1 => {
                        painter.rect_stroke(Rect::from_center_size(c, vec2(8.0, 7.0)), CornerRadius::ZERO, Stroke::new(1.0, BLACK), egui::StrokeKind::Inside);
                    }
                    _ => {
                        painter.line_segment([c + vec2(-3.0, -3.0), c + vec2(3.0, 3.0)], Stroke::new(1.5, BLACK));
                        painter.line_segment([c + vec2(-3.0, 3.0), c + vec2(3.0, -3.0)], Stroke::new(1.5, BLACK));
                    }
                }
            }
            let form_face = parse_hex(&self.st.proj.color).unwrap_or(FACE);
            fill(&painter, fx0, fy0, fx0 + w - 1.0, fy0 + h - 1.0, form_face);
            if self.st.show_grid {
                let step = if w * h > 600_000.0 { 2 * GRID } else { GRID } as usize;
                for x in (0..self.st.proj.width).step_by(step) {
                    for y in (0..self.st.proj.height).step_by(step) {
                        fill(&painter, fx0 + x as f32, fy0 + y as f32, fx0 + x as f32 + 1.0, fy0 + y as f32 + 1.0, DARK);
                    }
                }
            }
            // Komponenten
            for wd in &self.st.proj.widgets {
                draw_widget(&painter, org, wd, &font);
            }
            // Griffe
            if let Some(wd) = self.st.selected() {
                if wd.wtype.is_visual() {
                    for (_, (px, py)) in handle_points(wd) {
                        let (cx, cy) = ((org.x + px as f32).round(), (org.y + py as f32).round());
                        fill(&painter, cx - 2.0, cy - 2.0, cx + 2.0, cy + 2.0, BLACK);
                    }
                } else {
                    let r = Rect::from_min_size(org + vec2(wd.x as f32, wd.y as f32), vec2(wd.w as f32, wd.h as f32)).expand(2.0);
                    painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, BLACK), egui::StrokeKind::Outside);
                }
            } else {
                // Formular selbst: Griffe rechts/unten zum Vergroessern
                for (px, py) in [(w, h / 2.0), (w / 2.0, h), (w, h)] {
                    let (cx, cy) = ((org.x + px).round() + 1.0, (org.y + py).round() + 1.0);
                    fill(&painter, cx - 2.0, cy - 2.0, cx + 2.0, cy + 2.0, BLACK);
                }
            }
            if let Some((x1, y1, x2, y2)) = self.st.rubber_band() {
                let r = Rect::from_two_pos(org + vec2(x1 as f32, y1 as f32), org + vec2(x2 as f32, y2 as f32));
                painter.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, BLACK), egui::StrokeKind::Middle);
            }
        });
    }

    // -------------------------------------------------------------- Code-Editor
    /// Wendet eine Editor-Aenderung (editing.rs) auf den Code und den Cursor an.
    fn apply_edit(&mut self, ctx: &egui::Context, id: egui::Id, e: &editing::Edit) {
        let text = &mut self.st.logic;
        let (bf, bt) = (char_to_byte(text, e.from), char_to_byte(text, e.to));
        text.replace_range(bf..bt, &e.insert);
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(e.sel.0), CCursor::new(e.sel.1))));
        state.store(ctx, id);
        self.st.logic_dirty = true;
    }

    fn code_view(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let id = egui::Id::new("code_editor");
        let mono = FontId::monospace(14.0);

        // IDE-Verhalten: Enter, Tab/Umschalt+Tab, Rueck, Strg+/, Klammern/Anfuehrungszeichen
        if ctx.memory(|m| m.has_focus(id)) {
            if let Some(state) = egui::TextEdit::load_state(&ctx, id) {
                if let Some(range) = state.cursor.char_range() {
                    let (pa, pb) = (range.primary.index.0, range.secondary.index.0);
                    let (a, b) = (pa.min(pb), pa.max(pb));
                    let text = self.st.logic.clone();
                    let edit: Option<editing::Edit>;
                    let (enter, shift_tab, tab, comment, back) = ui.input_mut(|i| {
                        (
                            i.consume_key(Modifiers::NONE, Key::Enter),
                            i.consume_key(Modifiers::SHIFT, Key::Tab),
                            i.consume_key(Modifiers::NONE, Key::Tab),
                            i.consume_key(Modifiers::COMMAND, Key::Slash),
                            // Rueck nur abfangen, wenn es etwas Besonderes zu tun gibt
                            editing::backspace(&text, a, b).is_some() && i.consume_key(Modifiers::NONE, Key::Backspace),
                        )
                    });
                    if enter {
                        edit = Some(editing::enter(&text, a, b));
                    } else if shift_tab {
                        edit = Some(editing::tab(&text, a, b, true));
                    } else if tab {
                        edit = Some(editing::tab(&text, a, b, false));
                    } else if comment {
                        edit = Some(editing::toggle_comment(&text, a, b));
                    } else if back {
                        edit = editing::backspace(&text, a, b);
                    } else {
                        // getippte Zeichen: Klammern und Anfuehrungszeichen
                        let mut hit: Option<editing::Edit> = None;
                        ui.input_mut(|i| {
                            let mut k = 0;
                            while k < i.events.len() {
                                if let egui::Event::Text(t) = &i.events[k] {
                                    let mut it = t.chars();
                                    if let (Some(ch), None) = (it.next(), it.next()) {
                                        if let Some(e) = editing::typed(&text, a, b, ch) {
                                            hit = Some(e);
                                            i.events.remove(k);
                                            break;
                                        }
                                    }
                                }
                                k += 1;
                            }
                        });
                        edit = hit;
                    }
                    if let Some(e) = edit {
                        self.apply_edit(&ctx, id, &e);
                    }
                }
            }
        }

        // Sprung zu einer Funktion (Doppelklick im Designer)
        let goto = self.st.goto_char.take();
        if let Some(ci) = goto {
            let mut state = egui::TextEdit::load_state(&ctx, id).unwrap_or_default();
            state.cursor.set_char_range(Some(CCursorRange::one(CCursor::new(ci))));
            state.store(&ctx, id);
            ctx.memory_mut(|m| m.request_focus(id));
        }

        let mut layouter = |ui: &Ui, buf: &dyn egui::TextBuffer, wrap: f32| {
            let mut job = pyhl::highlight(buf.as_str(), &mono);
            job.wrap.max_width = wrap;
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let lines = self.st.logic.matches('\n').count() + 1;
        let gutter_w = 14.0 + 8.5 * (lines.to_string().len().max(3) as f32);
        egui::ScrollArea::both().auto_shrink([false, false]).id_salt("code_scroll").show(ui, |ui| {
            let avail = ui.available_rect_before_wrap();
            let gutter = Rect::from_min_max(avail.min, pos2(avail.min.x + gutter_w, avail.max.y));
            let text_rect = Rect::from_min_max(pos2(avail.min.x + gutter_w, avail.min.y), avail.max);
            let bg_idx = ui.painter().add(egui::Shape::Noop);
            let hl_idx = ui.painter().add(egui::Shape::Noop);
            let out = ui
                .scope_builder(egui::UiBuilder::new().max_rect(text_rect), |ui| {
                    egui::TextEdit::multiline(&mut self.st.logic)
                        .id(id)
                        .font(FontId::monospace(14.0))
                        .lock_focus(true)
                        .desired_width(f32::INFINITY)
                        .desired_rows(40)
                        .layouter(&mut layouter)
                        .show(ui)
                })
                .inner;
            if out.response.changed() {
                self.st.logic_dirty = true;
            }
            let cur_row = out.cursor_range.map(|r| out.galley.layout_from_cursor(r.primary).row);
            // Zeilennummern + aktuelle Zeile
            let full_h = out.response.rect.height().max(avail.height());
            ui.painter().set(
                bg_idx,
                egui::Shape::rect_filled(
                    Rect::from_min_size(gutter.min, vec2(gutter_w, full_h)),
                    CornerRadius::ZERO,
                    Color32::from_rgb(0xec, 0xe9, 0xe0),
                ),
            );
            let mut line_no = 0usize;
            let mut new_line = true;
            for (ri, prow) in out.galley.rows.iter().enumerate() {
                if new_line {
                    line_no += 1;
                    let y = out.galley_pos.y + prow.pos.y;
                    let cur = Some(ri) == cur_row;
                    ui.painter().text(
                        pos2(gutter.max.x - 6.0, y),
                        Align2::RIGHT_TOP,
                        line_no.to_string(),
                        FontId::monospace(12.0),
                        if cur { Color32::from_rgb(0x20, 0x20, 0x20) } else { Color32::from_rgb(0x90, 0x90, 0x90) },
                    );
                }
                new_line = prow.ends_with_newline;
                if Some(ri) == cur_row {
                    let r = Rect::from_min_size(
                        pos2(text_rect.min.x, out.galley_pos.y + prow.pos.y),
                        vec2(text_rect.width().min(4000.0), prow.row.size.y),
                    );
                    ui.painter().set(
                        hl_idx,
                        egui::Shape::rect_filled(r, CornerRadius::ZERO, Color32::from_rgba_unmultiplied(0xff, 0xe9, 0x6a, 0x30)),
                    );
                }
            }
            ui.painter().line_segment(
                [pos2(gutter.max.x, gutter.min.y), pos2(gutter.max.x, gutter.min.y + full_h)],
                Stroke::new(1.0, Color32::from_rgb(0xb0, 0xad, 0xa4)),
            );
            if let Some(range) = out.cursor_range {
                let (l, c) = line_col(&self.st.logic, range.primary.index.0);
                self.cursor_text = format!("{}: {}", l, c);
                if goto.is_some() {
                    let r = out.galley.pos_from_cursor(range.primary);
                    let target = r.translate(out.galley_pos.to_vec2()).expand2(vec2(0.0, 60.0));
                    ui.scroll_to_rect(target, Some(Align::Center));
                }
            }
        });
    }

    fn modals(&mut self, ctx: &egui::Context) {
        // Zeilen-Editor (Items / Lines)
        if let Some(Modal::Items { text, list }) = &mut self.modal {
            let mut done: Option<bool> = None;
            let is_list = *list;
            egui::Window::new("Zeilen-Editor")
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(if is_list { "Einträge (eine Zeile pro Eintrag):" } else { "Zeilen des Textfelds:" });
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(text).font(FontId::monospace(13.0)).desired_rows(12).desired_width(340.0));
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("    OK    ").clicked() {
                            done = Some(true);
                        }
                        if ui.button("Abbrechen").clicked() {
                            done = Some(false);
                        }
                    });
                });
            if let Some(ok) = done {
                let value = if is_list {
                    text.lines().map(|l| l.trim().replace('|', "/")).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("|")
                } else {
                    text.replace("\r\n", "\n")
                };
                self.modal = None;
                if ok {
                    if let Err(e) = self.st.set_prop("text_raw", &value) {
                        self.modal = Some(Modal::Info(e));
                    }
                }
            }
            return;
        }
        let Some(modal) = &self.modal else { return };
        enum Choice {
            None,
            Yes,
            No,
            Cancel,
        }
        let mut choice = Choice::None;
        let (kind_save, kind_conflict, text): (bool, bool, String) = match modal {
            Modal::Info(t) => (false, false, t.clone()),
            Modal::Items { .. } => (false, false, String::new()),
            Modal::SaveChanges(_) => (true, false, "Änderungen speichern?".into()),
            Modal::LogicConflict(_) => (
                false,
                true,
                "Die Logik-Datei existiert bereits oder wurde außerhalb von Pyphi7 geändert.\n\n\
                 Ja = mit dem Editor-Inhalt überschreiben\n\
                 Nein = vorhandene Datei laden (Editor-Inhalt verwerfen)\n\
                 Abbrechen = nicht speichern"
                    .into(),
            ),
        };
        egui::Window::new(APP_NAME)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.label(&text);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if kind_save || kind_conflict {
                        if ui.button("   Ja   ").clicked() {
                            choice = Choice::Yes;
                        }
                        if ui.button("  Nein  ").clicked() {
                            choice = Choice::No;
                        }
                        if ui.button("Abbrechen").clicked() {
                            choice = Choice::Cancel;
                        }
                    } else if ui.button("    OK    ").clicked() {
                        choice = Choice::Yes;
                    }
                });
            });
        if matches!(choice, Choice::None) {
            return;
        }
        let modal = self.modal.take().unwrap();
        match (modal, choice) {
            (Modal::SaveChanges(then), Choice::Yes) => {
                self.persist(ctx, Action::Save, false, LogicPolicy::Ask);
                if self.modal.is_none() && !self.st.is_dirty() {
                    self.perform(ctx, then);
                }
            }
            (Modal::SaveChanges(then), Choice::No) => {
                // Verwerfen: als sauber markieren, dann ausfuehren
                self.st.dirty = false;
                self.st.logic_dirty = false;
                self.perform(ctx, then);
            }
            (Modal::LogicConflict(what), Choice::Yes) => self.persist(ctx, what, false, LogicPolicy::Overwrite),
            (Modal::LogicConflict(what), Choice::No) => self.persist(ctx, what, false, LogicPolicy::LoadExisting),
            _ => {}
        }
    }
}

fn hide_console(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    {
        let _ = cmd;
    }
}

fn parse_hex(s: &str) -> Option<Color32> {
    let h = s.trim().trim_start_matches('#');
    let h = if h.len() == 3 { h.chars().flat_map(|c| [c, c]).collect::<String>() } else { h.to_string() };
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(&h, 16).ok()?;
    Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

fn put(p: &Painter, pos: egui::Pos2, al: Align2, t: &str, f: &FontId, c: Color32, bold: bool) {
    p.text(pos, al, t, f.clone(), c);
    if bold {
        p.text(pos + vec2(1.0, 0.0), al, t, f.clone(), c);
    }
}

fn dotted_rect(p: &Painter, x1: f32, y1: f32, x2: f32, y2: f32, c: Color32) {
    let mut x = x1;
    while x <= x2 {
        fill(p, x, y1, x, y1, c);
        fill(p, x, y2, x, y2, c);
        x += 2.0;
    }
    let mut y = y1;
    while y <= y2 {
        fill(p, x1, y, x1, y, c);
        fill(p, x2, y, x2, y, c);
        y += 2.0;
    }
}

fn draw_widget(p: &Painter, org: egui::Pos2, wd: &model::Widget, base: &FontId) {
    let x1 = (org.x + wd.x as f32).round();
    let y1 = (org.y + wd.y as f32).round();
    let (w, h) = (wd.w as f32, wd.h as f32);
    let (x2, y2) = (x1 + w - 1.0, y1 + h - 1.0);
    let cy = ((y1 + y2) / 2.0).round();
    let (cx, _) = (((x1 + x2) / 2.0).round(), 0.0);
    // Schrift wie im Objektinspektor eingestellt (Font.Size / Bold / Color)
    let size = if wd.wtype.has_font() { wd.prop_int("font_size") } else { 0 };
    let font = if size > 0 { FontId::proportional(size as f32 * 1.333) } else { base.clone() };
    let bold = wd.wtype.has_font() && wd.prop_bool("font_bold");
    let col = if wd.wtype.has_font() { parse_hex(&wd.prop("font_color")).unwrap_or(BLACK) } else { BLACK };
    let fg = if wd.enabled { col } else { GRAY };
    let lh = p.layout_no_wrap("Ag".into(), font.clone(), BLACK).size().y;
    let font = &font;
    match wd.wtype {
        WType::Button => {
            bevel(p, x1, y1, x2, y2, false, FACE);
            let t = fit(p, wd.text.lines().next().unwrap_or(""), font, w - 8.0);
            put(p, pos2(cx, cy), Align2::CENTER_CENTER, &t, font, fg, bold);
        }
        WType::Label => {
            let max_lines = ((h / lh) as usize).max(1);
            let al = wd.prop("alignment");
            for (i, line) in wd.text.split('\n').take(max_lines).enumerate() {
                let t = fit(p, line, font, w);
                let yy = y1 + i as f32 * lh;
                match al.as_str() {
                    "taCenter" => put(p, pos2(cx, yy), Align2::CENTER_TOP, &t, font, fg, bold),
                    "taRightJustify" => put(p, pos2(x2 + 1.0, yy), Align2::RIGHT_TOP, &t, font, fg, bold),
                    _ => put(p, pos2(x1, yy), Align2::LEFT_TOP, &t, font, fg, bold),
                }
            }
        }
        WType::Entry => {
            bevel(p, x1, y1, x2, y2, true, WHITE);
            let shown = match wd.prop("password").chars().next() {
                Some(ch) => ch.to_string().repeat(wd.text.chars().count()),
                None => wd.text.clone(),
            };
            put(p, pos2(x1 + 4.0, cy), Align2::LEFT_CENTER, &fit(p, &shown, font, w - 8.0), font, fg, bold);
        }
        WType::Checkbox => {
            bevel(p, x1, cy - 6.0, x1 + 12.0, cy + 6.0, true, WHITE);
            if wd.prop_bool("checked") {
                let (a, m, e) = (pos2(x1 + 3.0, cy), pos2(x1 + 5.0, cy + 3.0), pos2(x1 + 9.5, cy - 3.0));
                p.line_segment([a, m], Stroke::new(2.0, BLACK));
                p.line_segment([m, e], Stroke::new(2.0, BLACK));
            }
            put(p, pos2(x1 + 19.0, cy), Align2::LEFT_CENTER, &fit(p, &wd.text, font, w - 20.0), font, fg, bold);
        }
        WType::Radiobutton => {
            let c = pos2(x1 + 6.5, cy);
            p.circle_filled(c, 6.5, WHITE);
            p.circle_stroke(c, 6.5, Stroke::new(1.0, GRAY));
            p.circle_stroke(c, 5.5, Stroke::new(1.0, DARK));
            if wd.prop_bool("checked") {
                p.circle_filled(c, 2.5, BLACK);
            }
            put(p, pos2(x1 + 19.0, cy), Align2::LEFT_CENTER, &fit(p, &wd.text, font, w - 20.0), font, fg, bold);
        }
        WType::Listbox | WType::Text | WType::Combobox => {
            bevel(p, x1, y1, x2, y2, true, WHITE);
            let lines: Vec<String> = if wd.wtype == WType::Text {
                wd.text.split('\n').map(|s| s.to_string()).collect()
            } else {
                model::items_of(wd)
            };
            if wd.wtype == WType::Combobox {
                bevel(p, x2 - 17.0, y1 + 2.0, x2 - 2.0, y2 - 2.0, false, FACE);
                let a = pos2(x2 - 10.0, cy);
                p.add(egui::Shape::convex_polygon(vec![a + vec2(-3.5, -1.5), a + vec2(3.5, -1.5), a + vec2(0.0, 2.5)], BLACK, Stroke::NONE));
                let idx = wd.prop_int("index");
                if let Some(first) = lines.get(idx.max(0) as usize).filter(|_| idx >= 0) {
                    put(p, pos2(x1 + 4.0, cy), Align2::LEFT_CENTER, &fit(p, first, font, w - 28.0), font, fg, bold);
                }
            } else {
                let max_lines = (((h - 6.0) / lh) as usize).max(1);
                let sel = if wd.wtype == WType::Listbox { wd.prop_int("index") } else { -1 };
                for (i, line) in lines.iter().take(max_lines).enumerate() {
                    let yy = y1 + 3.0 + i as f32 * lh;
                    let mut c = fg;
                    if i as i32 == sel {
                        fill(p, x1 + 2.0, yy, x2 - 2.0, yy + lh - 1.0, NAVY);
                        c = WHITE;
                    }
                    put(p, pos2(x1 + 4.0, yy), Align2::LEFT_TOP, &fit(p, line, font, w - 8.0), font, c, bold);
                }
            }
        }
        WType::Groupbox => {
            let top = y1 + (lh / 2.0).round();
            let r = Rect::from_min_max(pos2(x1, top), pos2(x2, y2));
            p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, GRAY), egui::StrokeKind::Inside);
            p.rect_stroke(r.translate(vec2(1.0, 1.0)), CornerRadius::ZERO, Stroke::new(1.0, WHITE), egui::StrokeKind::Inside);
            let t = fit(p, &wd.text, font, w - 20.0);
            if !t.is_empty() {
                let tw = p.layout_no_wrap(t.clone(), font.clone(), BLACK).size().x;
                fill(p, x1 + 7.0, y1, x1 + 13.0 + tw, y1 + lh, FACE);
                put(p, pos2(x1 + 10.0, y1), Align2::LEFT_TOP, &t, font, fg, bold);
            }
        }
        WType::Panel => {
            let c = parse_hex(&wd.prop("color")).unwrap_or(FACE);
            match wd.prop("bevel").as_str() {
                "bvNone" => fill(p, x1, y1, x2, y2, c),
                "bvLowered" => bevel(p, x1, y1, x2, y2, true, c),
                _ => bevel(p, x1, y1, x2, y2, false, c),
            }
            put(p, pos2(cx, cy), Align2::CENTER_CENTER, &fit(p, &wd.text, font, w - 8.0), font, fg, bold);
        }
        WType::Progressbar => {
            bevel(p, x1, y1, x2, y2, true, FACE);
            let (mn, mx) = (wd.prop_int("min"), wd.prop_int("max").max(wd.prop_int("min") + 1));
            let frac = ((wd.prop_int("pos").clamp(mn, mx) - mn) as f32 / (mx - mn) as f32).clamp(0.0, 1.0);
            let inner = (w - 6.0) * frac;
            let mut x = x1 + 3.0;
            while x + 6.0 <= x1 + 3.0 + inner {
                fill(p, x, y1 + 3.0, x + 5.0, y2 - 3.0, NAVY);
                x += 8.0;
            }
        }
        WType::Trackbar => {
            let (mn, mx) = (wd.prop_int("min"), wd.prop_int("max").max(wd.prop_int("min") + 1));
            let frac = ((wd.prop_int("pos").clamp(mn, mx) - mn) as f32 / (mx - mn) as f32).clamp(0.0, 1.0);
            if wd.prop("orient") == "trVertical" {
                bevel(p, cx - 2.0, y1 + 6.0, cx + 2.0, y2 - 6.0, true, FACE);
                let ty = (y1 + 6.0 + (h - 22.0) * frac).round();
                bevel(p, cx - 10.0, ty, cx + 10.0, ty + 10.0, false, FACE);
            } else {
                bevel(p, x1 + 6.0, cy - 2.0, x2 - 6.0, cy + 2.0, true, FACE);
                let tx = (x1 + 6.0 + (w - 22.0) * frac).round();
                bevel(p, tx, cy - 10.0, tx + 10.0, cy + 10.0, false, FACE);
                let n = (mx - mn).min(20);
                for i in 0..=n {
                    let tx = x1 + 11.0 + (w - 22.0) * i as f32 / n as f32;
                    vline(p, tx.round(), cy + 12.0, cy + 15.0, BLACK);
                }
            }
        }
        WType::Spinedit => {
            bevel(p, x1, y1, x2, y2, true, WHITE);
            let (mn, mx) = (wd.prop_int("min"), wd.prop_int("max").max(wd.prop_int("min")));
            let v = wd.prop_int("pos").clamp(mn, mx).to_string();
            put(p, pos2(x1 + 4.0, cy), Align2::LEFT_CENTER, &v, font, fg, bold);
            let bx = x2 - 15.0;
            bevel(p, bx, y1 + 2.0, x2 - 2.0, cy, false, FACE);
            bevel(p, bx, cy, x2 - 2.0, y2 - 2.0, false, FACE);
            let a = pos2(bx + 7.0, (y1 + 2.0 + cy) / 2.0);
            p.add(egui::Shape::convex_polygon(vec![a + vec2(-3.0, 1.5), a + vec2(3.0, 1.5), a + vec2(0.0, -1.5)], BLACK, Stroke::NONE));
            let b = pos2(bx + 7.0, (cy + y2 - 2.0) / 2.0);
            p.add(egui::Shape::convex_polygon(vec![b + vec2(-3.0, -1.5), b + vec2(3.0, -1.5), b + vec2(0.0, 1.5)], BLACK, Stroke::NONE));
        }
        WType::Image => {
            dotted_rect(p, x1, y1, x2, y2, DARK);
            let pic = wd.prop("picture");
            let label = if pic.is_empty() { "(kein Bild)".to_string() } else { pic };
            p.line_segment([pos2(x1 + 3.0, y1 + 3.0), pos2(x2 - 3.0, y2 - 3.0)], Stroke::new(1.0, Color32::from_rgb(0xa0, 0xa0, 0xa0)));
            p.line_segment([pos2(x2 - 3.0, y1 + 3.0), pos2(x1 + 3.0, y2 - 3.0)], Stroke::new(1.0, Color32::from_rgb(0xa0, 0xa0, 0xa0)));
            let t = fit(p, &label, base, w - 8.0);
            let tw = p.layout_no_wrap(t.clone(), base.clone(), BLACK).size();
            fill(p, cx - tw.x / 2.0 - 2.0, cy - tw.y / 2.0, cx + tw.x / 2.0 + 2.0, cy + tw.y / 2.0, FACE);
            p.text(pos2(cx, cy), Align2::CENTER_CENTER, t, base.clone(), BLACK);
        }
        WType::Database => {
            bevel(p, x1, y1, x2, y2, false, FACE);
            let (ex, ey) = (7.0, 3.0);
            let c = pos2(cx + 0.5, cy + 0.5);
            fill(p, c.x - ex, c.y - 5.0, c.x + ex, c.y + 5.0, Color32::from_rgb(0xc8, 0xd8, 0xf0));
            vline(p, c.x - ex, c.y - 5.0, c.y + 5.0, DARK);
            vline(p, c.x + ex, c.y - 5.0, c.y + 5.0, DARK);
            for dy in [-5.0f32, 0.0, 5.0] {
                let top = pos2(c.x, c.y + dy);
                let pts: Vec<_> = (0..=12)
                    .map(|i| {
                        let a = std::f32::consts::PI * (i as f32) / 12.0;
                        top + vec2(-ex * a.cos(), if dy < 0.0 { -ey * a.sin() } else { ey * a.sin() })
                    })
                    .collect();
                p.add(egui::Shape::line(pts, Stroke::new(1.0, DARK)));
            }
            let top: Vec<_> = (0..16)
                .map(|i| {
                    let a = std::f32::consts::TAU * (i as f32) / 16.0;
                    pos2(c.x + ex * a.cos(), c.y - 5.0 + ey * a.sin())
                })
                .collect();
            p.add(egui::Shape::closed_line(top, Stroke::new(1.0, DARK)));
        }
        WType::Datagrid => {
            bevel(p, x1, y1, x2, y2, true, WHITE);
            let cols = {
                let c = model::items_of(wd);
                if c.is_empty() { vec!["Spalte1".to_string(), "Spalte2".to_string()] } else { c }
            };
            let hh = lh + 3.0;
            fill(p, x1 + 2.0, y1 + 2.0, x2 - 2.0, y1 + 2.0 + hh, FACE);
            hline(p, x1 + 2.0, x2 - 2.0, y1 + 2.0 + hh, GRAY);
            let cw = ((w - 4.0) / cols.len() as f32).max(20.0);
            for (i, name) in cols.iter().enumerate() {
                let cx0 = x1 + 2.0 + i as f32 * cw;
                if cx0 + 6.0 > x2 {
                    break;
                }
                if i > 0 {
                    vline(p, cx0, y1 + 2.0, y1 + 2.0 + hh, GRAY);
                }
                put(p, pos2(cx0 + 4.0, y1 + 2.0 + hh / 2.0), Align2::LEFT_CENTER, &fit(p, name, font, (cw - 8.0).min(x2 - cx0 - 6.0)), font, BLACK, true);
            }
            let mut yy = y1 + 3.0 + hh + lh;
            while yy < y2 - 2.0 {
                hline(p, x1 + 2.0, x2 - 2.0, yy, Color32::from_rgb(0xe0, 0xe0, 0xe0));
                yy += lh + 2.0;
            }
        }
        WType::Timer => {
            bevel(p, x1, y1, x2, y2, false, FACE);
            let c = pos2(cx + 0.5, cy + 0.5);
            p.circle_filled(c, 8.0, WHITE);
            p.circle_stroke(c, 8.0, Stroke::new(1.0, BLACK));
            p.line_segment([c, c + vec2(0.0, -6.0)], Stroke::new(1.0, BLACK));
            p.line_segment([c, c + vec2(4.0, 1.5)], Stroke::new(1.0, BLACK));
        }
    }
}

fn setup_style(ctx: &egui::Context) {
    // Windows: Tahoma/Segoe UI wie zu Delphi-Zeiten, sonst die eingebaute Schrift
    let mut fonts = egui::FontDefinitions::default();
    for (name, path) in [
        ("tahoma", r"C:\Windows\Fonts\tahoma.ttf"),
        ("segoeui", r"C:\Windows\Fonts\segoeui.ttf"),
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.into(), Arc::new(egui::FontData::from_owned(bytes)));
            fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, name.into());
            break;
        }
    }
    for (name, path) in [("consolas", r"C:\Windows\Fonts\consola.ttf"), ("cour", r"C:\Windows\Fonts\cour.ttf")] {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.into(), Arc::new(egui::FontData::from_owned(bytes)));
            fonts.families.entry(egui::FontFamily::Monospace).or_default().insert(0, name.into());
            break;
        }
    }
    ctx.set_fonts(fonts);

    let mut v = egui::Visuals::light();
    v.panel_fill = FACE;
    v.window_fill = FACE;
    v.extreme_bg_color = WHITE;
    v.faint_bg_color = FACE;
    v.override_text_color = Some(BLACK);
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.window_shadow = egui::Shadow::NONE;
    v.popup_shadow = egui::Shadow::NONE;
    v.window_stroke = Stroke::new(1.0, BLACK);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = CornerRadius::ZERO;
    }
    v.widgets.noninteractive.bg_fill = FACE;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, GRAY);
    v.widgets.inactive.weak_bg_fill = FACE;
    v.widgets.inactive.bg_fill = FACE;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, GRAY);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xe4, 0xe0, 0xd8);
    v.widgets.hovered.bg_fill = Color32::from_rgb(0xe4, 0xe0, 0xd8);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, DARK);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0xc4, 0xc0, 0xb8);
    v.widgets.active.bg_fill = Color32::from_rgb(0xc4, 0xc0, 0xb8);
    v.selection.bg_fill = Color32::from_rgb(0xa8, 0xc0, 0xe8);
    v.selection.stroke = Stroke::new(1.0, BLACK);
    ctx.set_visuals(v);
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.hint.clear();

        // Meldungen der laufenden Programme
        let mut got = false;
        while let Ok((line, kind)) = self.rx.try_recv() {
            if line.starts_with("[pip beendet") {
                self.pip_busy = false;
                self.libs_state = LibsState::Unknown; // Liste neu einlesen
            }
            self.st.log(line, kind);
            got = true;
        }
        while let Ok(res) = self.lib_rx.try_recv() {
            match res {
                Ok(l) => {
                    self.libs = l;
                    self.libs_state = LibsState::Loaded;
                }
                Err(e) => {
                    self.st.log(format!("Bibliotheksliste nicht lesbar: {}", e), LogKind::Err);
                    self.libs_state = LibsState::Failed;
                }
            }
            got = true;
        }
        if got || !self.procs.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // Klick ausserhalb des Inspektors beendet eine laufende Eingabe
        if ctx.input(|i| i.pointer.primary_pressed()) {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !self.insp_rect.contains(pos) {
                    self.commit_edit();
                }
            }
        }

        // Fenster schliessen
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.st.is_dirty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.modal = Some(Modal::SaveChanges(Action::Quit));
            } else {
                self.stop_all();
            }
        }

        // Tastenkuerzel
        if self.modal.is_none() {
            let typing = ctx.egui_wants_keyboard_input();
            let mut act: Option<Action> = None;
            ctx.input_mut(|i| {
                if i.consume_key(Modifiers::COMMAND, Key::S) {
                    act = Some(Action::Save);
                } else if i.consume_key(Modifiers::COMMAND, Key::N) {
                    act = Some(Action::New);
                } else if i.consume_key(Modifiers::COMMAND, Key::O) {
                    act = Some(Action::Open);
                } else if i.consume_key(Modifiers::COMMAND, Key::F9) {
                    act = Some(Action::Generate);
                } else if i.consume_key(Modifiers::COMMAND, Key::F2) {
                    act = Some(Action::Stop);
                } else if i.consume_key(Modifiers::NONE, Key::F9) {
                    act = Some(Action::Run);
                } else if i.consume_key(Modifiers::NONE, Key::F12) {
                    act = Some(Action::ToggleView);
                } else if !typing && self.st.view == View::Form {
                    if i.consume_key(Modifiers::COMMAND, Key::Z) {
                        act = Some(Action::Undo);
                    } else if i.consume_key(Modifiers::COMMAND, Key::Y) {
                        act = Some(Action::Redo);
                    } else if i.consume_key(Modifiers::COMMAND, Key::D) {
                        act = Some(Action::Duplicate);
                    } else if i.consume_key(Modifiers::NONE, Key::Delete) {
                        act = Some(Action::Delete);
                    } else if i.consume_key(Modifiers::NONE, Key::Escape) {
                        self.st.tool = None;
                        self.st.select(None);
                    }
                }
            });
            if let Some(a) = act {
                self.queue.push(a);
            }
            if !typing && self.st.view == View::Form {
                let mut nudge: Option<(i32, i32, bool, bool)> = None;
                ctx.input_mut(|i| {
                    for (key, dx, dy) in [(Key::ArrowLeft, -1, 0), (Key::ArrowRight, 1, 0), (Key::ArrowUp, 0, -1), (Key::ArrowDown, 0, 1)] {
                        if i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, key) {
                            nudge = Some((dx, dy, true, true));
                        } else if i.consume_key(Modifiers::COMMAND, key) {
                            nudge = Some((dx, dy, false, true));
                        } else if i.consume_key(Modifiers::SHIFT, key) {
                            nudge = Some((dx, dy, true, false));
                        } else if i.consume_key(Modifiers::NONE, key) {
                            nudge = Some((dx, dy, false, false));
                        }
                    }
                });
                if let Some((dx, dy, big, resize)) = nudge {
                    self.st.cmd_nudge(dx, dy, big, resize);
                }
            }
        }

        // ---- Layout
        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::top("toolbar")
            .frame(egui::Frame::NONE.fill(FACE).inner_margin(egui::Margin::same(4)))
            .show(ui, |ui| self.toolbar(ui));

        egui::Panel::bottom("status")
            .frame(egui::Frame::NONE.fill(FACE).inner_margin(egui::Margin::symmetric(2, 2)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                    let pos = if self.st.view == View::Code { self.cursor_text.clone() } else { "1: 1".into() };
                    let modified = if self.st.is_dirty() { "Geändert" } else { "" };
                    for (text, w) in [(pos.as_str(), 70.0), (modified, 80.0), ("Einfügen", 72.0)] {
                        let (r, _) = ui.allocate_exact_size(vec2(w, 20.0), Sense::hover());
                        bevel(ui.painter(), r.min.x, r.min.y, r.max.x - 1.0, r.max.y - 1.0, true, FACE);
                        ui.painter().text(r.center(), Align2::CENTER_CENTER, text, FontId::proportional(13.0), BLACK);
                    }
                    ui.add_space(8.0);
                    ui.label(&self.hint);
                });
            });

        egui::Panel::bottom("messages")
            .resizable(true)
            .default_size(110.0)
            .min_size(50.0)
            .frame(egui::Frame::NONE.fill(FACE).inner_margin(egui::Margin::same(4)))
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Meldungen").strong());
                egui::ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    for (line, kind) in &self.st.log {
                        let col = match kind {
                            LogKind::Normal => BLACK,
                            LogKind::Info => Color32::from_rgb(0x00, 0x00, 0x80),
                            LogKind::Err => Color32::from_rgb(0xb0, 0x00, 0x00),
                        };
                        ui.label(egui::RichText::new(line).monospace().color(col));
                    }
                });
            });

        let ir = egui::Panel::left("inspector")
            .resizable(true)
            .default_size(290.0)
            .min_size(220.0)
            .frame(egui::Frame::NONE.fill(FACE).inner_margin(egui::Margin::same(3)))
            .show(ui, |ui| self.inspector(ui));
        self.insp_rect = ir.response.rect;

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(FACE).inner_margin(egui::Margin::same(2)))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let code_name = self.st.stem().map(|s| format!("{}_logic.py", s)).unwrap_or_else(|| "Unit1.py".into());
                    let form_name = if self.st.proj.title.is_empty() { FORM_CLASS.to_string() } else { self.st.proj.title.clone() };
                    for (view, name) in [(View::Form, form_name), (View::Code, code_name)] {
                        if ui.selectable_label(self.st.view == view, name).clicked() {
                            self.st.view = view;
                        }
                    }
                });
                match self.st.view {
                    View::Form => self.form_view(ui),
                    View::Code => self.code_view(ui),
                }
            });

        // ---- Fenster "Bibliotheken" (pip)
        if self.libs_open {
            let mut open = true;
            egui::Window::new("Bibliotheken")
                .open(&mut open)
                .collapsible(false)
                .default_size([660.0, 560.0])
                .default_pos([260.0, 90.0])
                .show(&ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| self.libs_view(ui));
                });
            if !open {
                self.libs_open = false;
            }
        }

        // ---- Aktionen und Dialoge
        let actions: Vec<Action> = std::mem::take(&mut self.queue);
        for a in actions {
            self.request(&ctx, a);
        }
        self.modals(&ctx);

        // Fenstertitel
        let name = self.st.path.as_ref().and_then(|p| p.file_name()).and_then(|s| s.to_str()).unwrap_or("Projekt1");
        let title = format!("{} - {}{}", APP_NAME, name, if self.st.is_dirty() { "*" } else { "" });
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }
}
