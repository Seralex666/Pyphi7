# -*- coding: utf-8 -*-
"""Logik fuer das Beispiel "Ausgaben".

Waehle links eine Kategorie, tippe einen Betrag ein und klicke auf "+"
(oder druecke Enter im Betragsfeld): Die Ausgabe landet im Memo, unten
steht die Summe. Laeuft unveraendert mit Tkinter und mit Qt.
"""
from decimal import Decimal, InvalidOperation

summe = Decimal("0")


def euro(betrag):
    """Decimal -> '1.234,50' (deutsches Format)."""
    text = f"{betrag:,.2f}"
    return text.replace(",", "#").replace(".", ",").replace("#", ".")


def betrag_lesen(text):
    """'12,50' / '12.50' / '1.234,50' -> Decimal, sonst None."""
    text = text.strip().replace("€", "").replace(" ", "")
    if "," in text:                       # deutsches Format: Punkt = Tausender
        text = text.replace(".", "").replace(",", ".")
    try:
        wert = Decimal(text)
    except InvalidOperation:
        return None
    return wert if wert > 0 else None


def FormCreate(ui):
    ui.set_text("Memo1", "")
    ui.set_text("Label4", "Summe: 0,00 €")


def ListBox1Click(ui):
    # nach der Auswahl direkt ins Betragsfeld, damit man gleich tippen kann
    ui.widget("Edit1").focus_set() if hasattr(ui.widget("Edit1"), "focus_set") else ui.widget("Edit1").setFocus()


def Edit1KeyPress(ui):
    if ui.key == "Enter":
        Button1Click(ui)


def Button1Click(ui):
    global summe
    index = ui.selected_index("ListBox1")
    if index < 0:
        ui.message("Bitte zuerst links eine Kategorie auswählen.")
        return
    betrag = betrag_lesen(ui.get_text("Edit1"))
    if betrag is None:
        ui.message("Bitte einen Betrag größer 0 eingeben, z. B. 12,50.")
        return
    kategorie = ui.get_items("ListBox1")[index]
    zeile = f"{kategorie:<12}{euro(betrag):>12} €"
    alt = ui.get_text("Memo1")
    ui.set_text("Memo1", (alt + "\n" if alt else "") + zeile)
    summe += betrag
    ui.set_text("Label4", f"Summe: {euro(summe)} €")
    ui.set_text("Edit1", "")


def Button2Click(ui):
    global summe
    if ui.ask_yes_no("Wirklich alle Ausgaben löschen?"):
        summe = Decimal("0")
        FormCreate(ui)
