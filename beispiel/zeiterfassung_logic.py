# -*- coding: utf-8 -*-
"""Zeiterfassung: Einstempeln / Ausstempeln, gespeichert in SQLite."""
from datetime import datetime


def jetzt():
    return datetime.now().strftime("%Y-%m-%d %H:%M:%S")


def anzeigen(ui):
    # Die Spaltennamen der Tabelle kommen aus den AS-Namen der Abfrage
    ui.show_query("Grid1", """
        SELECT name AS Name,
               strftime('%d.%m. %H:%M', kommt) AS Kommt,
               COALESCE(strftime('%d.%m. %H:%M', geht), '') AS Geht,
               CASE WHEN geht IS NULL THEN 'läuft'
                    ELSE printf('%.2f h', (julianday(geht) - julianday(kommt)) * 24)
               END AS Dauer
        FROM zeiten ORDER BY id DESC
    """)
    offen = ui.db_value("SELECT COUNT(*) FROM zeiten WHERE geht IS NULL", default=0)
    ui.set_text("LblStatus", "%d eingestempelt" % offen)


def FormCreate(ui):
    ui.db_script("""
        CREATE TABLE IF NOT EXISTS zeiten (
            id    INTEGER PRIMARY KEY AUTOINCREMENT,
            name  TEXT NOT NULL,
            kommt TEXT NOT NULL,
            geht  TEXT
        );
    """)
    anzeigen(ui)


def BtnInClick(ui):
    name = ui.get_text("EdName").strip()
    if not name:
        ui.message("Bitte erst deinen Namen eingeben.")
        return
    if ui.db_value("SELECT id FROM zeiten WHERE name = ? AND geht IS NULL", (name,)):
        ui.message(name + " ist schon eingestempelt.")
        return
    ui.db_execute("INSERT INTO zeiten (name, kommt) VALUES (?, ?)", (name, jetzt()))
    anzeigen(ui)


def BtnOutClick(ui):
    name = ui.get_text("EdName").strip()
    geaendert = ui.db_execute("UPDATE zeiten SET geht = ? WHERE name = ? AND geht IS NULL", (jetzt(), name))
    if geaendert == 0:
        ui.message(name + " ist nicht eingestempelt.")
    anzeigen(ui)
