//! Farbschemata der Entwicklungsumgebung (hell = klassisch Win9x, dunkel = Dark Mode).
//!
//! Die Oberflaeche des *entworfenen* Programms (Formular, Komponenten, Palette-Symbole) bleibt
//! immer hell, weil Tk/Qt-Fenster so aussehen: dafuer gibt es den Wachposten [`Light`].

use eframe::egui::Color32;
use std::cell::Cell;

#[derive(Clone, Copy)]
pub struct Pal {
    /// Flaechen (Panels, Knoepfe)
    pub face: Color32,
    /// Eingabefelder, Editor-Hintergrund
    pub white: Color32,
    /// Mittlerer Schatten / abgeblendeter Text / feine Rahmen
    pub gray: Color32,
    pub dark: Color32,
    /// Normaler Text
    pub black: Color32,
    /// Titelleisten
    pub navy: Color32,
    /// Flaeche hinter dem Formular
    pub workspace: Color32,
    // Win9x-Kanten: Licht, innerer Schatten, aeusserer Schatten
    pub hi: Color32,
    pub mid: Color32,
    pub low: Color32,
    pub out: Color32,
    pub err: Color32,
    pub ok: Color32,
    pub info: Color32,
    pub line1: Color32,
    pub line2: Color32,
    pub hover: Color32,
    pub active: Color32,
    pub select: Color32,
    pub gutter: Color32,
    pub gutter_line: Color32,
    pub gutter_cur: Color32,
    pub gutter_dim: Color32,
    pub curline: Color32,
}

const fn c(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub const LIGHT: Pal = Pal {
    face: c(0xd4, 0xd0, 0xc8),
    white: Color32::WHITE,
    gray: c(0x80, 0x80, 0x80),
    dark: c(0x40, 0x40, 0x40),
    black: Color32::BLACK,
    navy: c(0x0a, 0x24, 0x6a),
    workspace: c(0x7b, 0x7b, 0x7b),
    hi: Color32::WHITE,
    mid: c(0x80, 0x80, 0x80),
    low: c(0x40, 0x40, 0x40),
    out: Color32::BLACK,
    err: c(0xa0, 0x00, 0x00),
    ok: c(0x00, 0x70, 0x00),
    info: c(0x00, 0x00, 0x80),
    line1: c(0xe0, 0xe0, 0xe0),
    line2: c(0xc0, 0xc0, 0xc0),
    hover: c(0xe4, 0xe0, 0xd8),
    active: c(0xc4, 0xc0, 0xb8),
    select: c(0xa8, 0xc0, 0xe8),
    gutter: c(0xec, 0xe9, 0xe0),
    gutter_line: c(0xb0, 0xad, 0xa4),
    gutter_cur: c(0x20, 0x20, 0x20),
    gutter_dim: c(0x90, 0x90, 0x90),
    curline: Color32::from_rgba_premultiplied(0x30, 0x2c, 0x14, 0x30),
};

pub const DARK: Pal = Pal {
    face: c(0x2b, 0x2d, 0x31),
    white: c(0x1e, 0x1f, 0x22),
    gray: c(0x8a, 0x8f, 0x98),
    dark: c(0x9a, 0xa0, 0xaa),
    black: c(0xd8, 0xdb, 0xe2),
    navy: c(0x23, 0x3d, 0x73),
    workspace: c(0x3c, 0x3f, 0x44),
    hi: c(0x4f, 0x53, 0x5a),
    mid: c(0x1a, 0x1b, 0x1e),
    low: c(0x38, 0x3a, 0x3f),
    out: c(0x0c, 0x0d, 0x0f),
    err: c(0xff, 0x7b, 0x72),
    ok: c(0x7e, 0xd6, 0x8a),
    info: c(0x8f, 0xb4, 0xff),
    line1: c(0x33, 0x36, 0x3b),
    line2: c(0x45, 0x48, 0x4f),
    hover: c(0x3a, 0x3d, 0x43),
    active: c(0x4a, 0x4e, 0x56),
    select: c(0x2f, 0x4f, 0x8a),
    gutter: c(0x25, 0x27, 0x2b),
    gutter_line: c(0x3f, 0x42, 0x48),
    gutter_cur: c(0xe0, 0xe2, 0xe8),
    gutter_dim: c(0x6c, 0x71, 0x7a),
    curline: Color32::from_rgba_premultiplied(0x10, 0x10, 0x10, 0x10),
};

thread_local! {
    static DARK_ON: Cell<bool> = const { Cell::new(false) };
    static FORCE_LIGHT: Cell<bool> = const { Cell::new(false) };
}

pub fn set_dark(on: bool) {
    DARK_ON.with(|d| d.set(on));
}

pub fn is_dark() -> bool {
    DARK_ON.with(|d| d.get()) && !FORCE_LIGHT.with(|f| f.get())
}

pub fn pal() -> Pal {
    if is_dark() {
        DARK
    } else {
        LIGHT
    }
}

/// Solange dieser Wachposten lebt, wird hell gemalt (Formular-Inhalt, Palette-Symbole).
pub struct Light(bool);

impl Light {
    pub fn on() -> Light {
        Light(FORCE_LIGHT.with(|f| f.replace(true)))
    }
}

impl Drop for Light {
    fn drop(&mut self) {
        let prev = self.0;
        FORCE_LIGHT.with(|f| f.set(prev));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_guard_overrides_dark() {
        set_dark(true);
        assert_eq!(pal().face, DARK.face);
        {
            let _g = Light::on();
            assert_eq!(pal().face, LIGHT.face);
            {
                let _g2 = Light::on();
                assert_eq!(pal().face, LIGHT.face);
            }
            assert_eq!(pal().face, LIGHT.face);
        }
        assert_eq!(pal().face, DARK.face);
        set_dark(false);
        assert_eq!(pal().face, LIGHT.face);
    }
}
