# -*- coding: utf-8 -*-
"""Kontakte: SQLite-Beispiel fuer Pyphi7 (Datenbank-Komponente + Datentabelle)."""


def aktualisieren(ui):
    such = "%" + ui.get_text("EdSearch").strip() + "%"
    ui.show_query("Grid1", "SELECT id, name, telefon FROM kontakte WHERE name LIKE ? ORDER BY name", (such,))
    n = ui.db_value("SELECT COUNT(*) FROM kontakte", default=0)
    ui.set_text("LblStatus", "%d Kontakte" % n)
    ui.set_enabled("BtnDelete", False)


def FormCreate(ui):
    ui.db_script("""
        CREATE TABLE IF NOT EXISTS kontakte (
            id      INTEGER PRIMARY KEY AUTOINCREMENT,
            name    TEXT NOT NULL,
            telefon TEXT
        );
    """)
    aktualisieren(ui)


def BtnAddClick(ui):
    name = ui.get_text("EdName").strip()
    if not name:
        ui.message("Bitte einen Namen eingeben.")
        return
    ui.db_execute("INSERT INTO kontakte (name, telefon) VALUES (?, ?)", (name, ui.get_text("EdTel").strip()))
    ui.set_text("EdName", "")
    ui.set_text("EdTel", "")
    aktualisieren(ui)


def BtnDeleteClick(ui):
    row = ui.get_row("Grid1")
    if row is None:
        return
    if ui.ask_yes_no("'%s' wirklich löschen?" % row[1]):
        ui.db_execute("DELETE FROM kontakte WHERE id = ?", (row[0],))
        aktualisieren(ui)


def EdSearchChange(ui):
    aktualisieren(ui)


def Grid1Click(ui):
    ui.set_enabled("BtnDelete", ui.get_row("Grid1") is not None)


def Grid1DblClick(ui):
    row = ui.get_row("Grid1")
    if row:
        ui.set_text("EdName", row[1])
        ui.set_text("EdTel", row[2] or "")
