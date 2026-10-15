# -*- coding: utf-8 -*-
"""Deine Logik fuer hallo.

Diese Datei gehoert dir: PyRAD ueberschreibt sie NIE. Doppelklick auf eine
Komponente im Designer haengt hier nur eine leere Funktion an (wie in Delphi).
Die gleiche Logik laeuft mit Tkinter UND mit Qt.

Jede Funktion bekommt das Fenster als 'ui'. Das kannst du damit tun:

    ui.get_text("Edit1")              Text lesen (Edit, Memo, Label, Button, ...)
    ui.set_text("Label1", "Hallo")    Text setzen
    ui.is_checked("CheckBox1")        Haken lesen
    ui.set_checked("CheckBox1", True) Haken setzen
    ui.set_items("ListBox1", ["a", "b"])  Eintraege der Liste/Auswahl ersetzen
    ui.get_items("ListBox1")          Eintraege lesen
    ui.selected_index("ListBox1")     gewaehlter Eintrag (-1 = keiner)
    ui.set_enabled("Button1", False)  Komponente sperren / freigeben
    ui.set_visible("Button1", False)  Komponente verstecken / zeigen
    ui.message("Text")                Meldungsfenster
    ui.close()                        Fenster schliessen
    ui.widget("Button1")              das echte Tk-/Qt-Widget (Spezialfaelle)
"""


def FormCreate(ui):
    ui.set_text("Label2", "Gib deinen Namen ein und klicke auf Begrüßen.")


def Button1Click(ui):
    name = ui.get_text("Edit1").strip() or "Unbekannter"
    gruss = ui.get_text("ListBox1") or "Hallo"
    text = "%s, %s!" % (gruss, name)
    if ui.is_checked("CheckBox1"):
        text = text.upper()
    ui.set_text("Label2", text)


def ListBox1Click(ui):
    # Gruss wählen -> gleich neu anzeigen, wenn schon ein Name da ist
    if ui.get_text("Edit1").strip():
        Button1Click(ui)


def Button2Click(ui):
    ui.close()
