# Pyphi7 – Funktionsreferenz

Deine Programmlogik steht in `<Projekt>_logic.py`. Jede Ereignis-Funktion bekommt das Fenster als
Parameter `ui` und läuft unverändert mit **Tkinter** und **Qt (PySide6)**.

```python
def Button1Click(ui):
    name = ui.get_text("Edit1")
    ui.set_text("Label1", "Hallo " + name)
```

Komponenten werden immer über ihren **Namen als Text** angesprochen (`"Edit1"`, `"ListBox1"` …).
Der Name ist die Eigenschaft *Name* im Objektinspektor.

## Inhalt
1. [Die ui-Funktionen](#die-ui-funktionen)
2. [Komponenten: Eigenschaften und Ereignisse](#komponenten)
3. [Formular](#formular)
4. [Ereignis-Funktionen](#ereignis-funktionen)
5. [Tastennamen (ui.key)](#tastennamen)
6. [Eigene Funktionen und Module](#eigene-funktionen)
7. [Welche Funktion passt zu welcher Komponente](#übersicht)

---

## Die ui-Funktionen

### Text
| Funktion | Beschreibung |
|---|---|
| `ui.get_text(name)` | Text lesen. Edit/ComboBox: Eingabe · Memo: gesamter Text · Label/Button/CheckBox/RadioButton/GroupBox/Panel: Beschriftung · ListBox: markierter Eintrag (`""` wenn keiner) · SpinEdit: Zahl als Text · ProgressBar/TrackBar/Image/Timer: `""` |
| `ui.set_text(name, text)` | Text setzen (der Wert wird mit `str()` umgewandelt). Bei Edit löst es genau ein `OnChange` aus. Bei Memo wird der alte Inhalt ersetzt. |

```python
ui.set_text("Label1", "Summe: 12,50 €")
zeilen = ui.get_text("Memo1").splitlines()
```

### Zahlen
| Funktion | Beschreibung |
|---|---|
| `ui.get_value(name)` | Zahl aus **ProgressBar**, **TrackBar** oder **SpinEdit** (ganzzahlig). Bei anderen Komponenten `ValueError`. |
| `ui.set_value(name, wert)` | Zahl setzen. SpinEdit begrenzt auf Min/Max. |

```python
ui.set_value("ProgressBar1", ui.get_value("TrackBar1") * 10)
```

### Haken und Optionen
| Funktion | Beschreibung |
|---|---|
| `ui.is_checked(name)` | `True`/`False` für **CheckBox** und **RadioButton**. |
| `ui.set_checked(name, wert=True)` | Haken setzen oder entfernen. RadioButtons mit gleicher *GroupIndex* schließen sich gegenseitig aus. |

### Listen (ListBox, ComboBox)
| Funktion | Beschreibung |
|---|---|
| `ui.set_items(name, liste)` | Alle Einträge ersetzen. |
| `ui.add_item(name, text)` | Eintrag am Ende anhängen. |
| `ui.get_items(name)` | Alle Einträge als Python-Liste. |
| `ui.selected_index(name)` | Index des gewählten Eintrags, `-1` = keiner. |
| `ui.set_selected_index(name, index)` | Eintrag wählen, `-1` = Auswahl aufheben. |

```python
ui.set_items("ListBox1", ["Miete", "Strom", "Essen"])
i = ui.selected_index("ListBox1")
if i >= 0:
    eintrag = ui.get_items("ListBox1")[i]
```

### Aussehen und Verhalten
| Funktion | Beschreibung |
|---|---|
| `ui.set_enabled(name, flag=True)` | Komponente sperren/freigeben. Bei einem **Timer** stoppt/startet es ihn. |
| `ui.set_visible(name, flag=True)` | Komponente verstecken/zeigen (Timer: ohne Wirkung). |
| `ui.set_interval(name, ms)` | Takt eines **Timers** in Millisekunden ändern. |
| `ui.set_image(name, pfad)` | Bild in einer **Image**-Komponente laden. Relativer Pfad = relativ zum Programmordner. PNG/GIF immer, JPG u. a. mit Pillow (`pip install Pillow`). |

```python
ui.set_enabled("Button1", False)        # Button grau
ui.set_enabled("Timer1", True)          # Timer starten
ui.set_interval("Timer1", 250)          # schneller takten
ui.set_image("Image1", "bild.png")
```

### Dialoge und Fenster
| Funktion | Beschreibung |
|---|---|
| `ui.message(text, titel="Hinweis")` | Meldungsfenster mit OK. |
| `ui.ask_yes_no(text, titel="Frage")` | Ja/Nein-Frage, liefert `True`/`False`. |
| `ui.close()` | Fenster schließen (ruft vorher `FormClose` auf). |
| `ui.key` | Letzte Taste (nur in `OnKeyPress`-Funktionen sinnvoll), siehe [Tastennamen](#tastennamen). |
| `ui.widget(name)` | Das echte Tk- bzw. Qt-Widget für Spezialfälle (toolkit-spezifisch!). |

```python
if ui.ask_yes_no("Wirklich löschen?"):
    ui.set_items("ListBox1", [])
```

---

## Komponenten

Jede Komponente hat zusätzlich zu den allgemeinen Eigenschaften **Name, Left, Top, Width, Height,
Enabled, Visible, Hint** (Tooltip) und – wo sinnvoll – **Caption/Text** sowie die Schrift
(**Font.Size, Font.Bold, Font.Color**) die unten genannten Eigenschaften und Ereignisse.
Ereignisse legst du per **Doppelklick** auf die Komponente an (Standard-Ereignis) oder im
Objektinspektor → Reiter *Ereignisse*. Die Funktion heißt `<Name><Endung>`, z. B. `Button1Click`.

### Button (TButton)
Schaltfläche. Eigenschaft: **Caption**.
Ereignis: **OnClick** → `Button1Click(ui)`

### Label (TLabel)
Beschriftung. Eigenschaften: **Caption**, **Alignment** (`taLeftJustify`, `taCenter`, `taRightJustify`).
Kein Ereignis. Text ändern mit `ui.set_text`.

### Edit (TEdit)
Einzeiliges Eingabefeld. Eigenschaften: **Text**, **ReadOnly**, **PasswordChar** (z. B. `*`).
Ereignisse: **OnChange** (`Edit1Change`), **OnEnter** (`Edit1Enter`, Fokus bekommen),
**OnExit** (`Edit1Exit`, Fokus verlieren), **OnKeyPress** (`Edit1KeyPress`, Taste in `ui.key`).
Funktionen: `get_text`, `set_text`.

### CheckBox (TCheckBox)
Kontrollkästchen. Eigenschaften: **Caption**, **Checked**.
Ereignis: **OnClick** (`CheckBox1Click`). Funktionen: `is_checked`, `set_checked`.

### RadioButton (TRadioButton)
Optionsfeld. Eigenschaften: **Caption**, **Checked**, **GroupIndex** (RadioButtons mit gleicher Zahl
bilden eine Gruppe, innerhalb derselben ist nur einer gewählt).
Ereignis: **OnClick** (`RadioButton1Click`). Funktionen: `is_checked`, `set_checked`.

### ListBox (TListBox)
Liste. Eigenschaften: **Items** (eine Zeile pro Eintrag, Button „…“ öffnet den Zeilen-Editor),
**ItemIndex** (Startauswahl, `-1` = keine).
Ereignisse: **OnClick** (`ListBox1Click`), **OnDblClick** (`ListBox1DblClick`), **OnEnter**, **OnExit**, **OnKeyPress**.
Funktionen: `set_items`, `add_item`, `get_items`, `selected_index`, `set_selected_index`, `get_text` (gewählter Eintrag).

### ComboBox (TComboBox)
Auswahlfeld. Eigenschaften: **Items**, **ItemIndex**, **Style** (`csDropDownList` = nur Auswahl,
`csDropDown` = auch freie Eingabe).
Ereignisse: **OnChange** (`ComboBox1Change`), **OnEnter**, **OnExit**.
Funktionen: `set_items`, `add_item`, `get_items`, `selected_index`, `set_selected_index`, `get_text`, `set_text`.

### Memo (TMemo)
Mehrzeiliges Textfeld. Eigenschaften: **Lines** (Startinhalt), **ReadOnly**, **WordWrap**.
Ereignisse: **OnChange** (`Memo1Change`), **OnEnter**, **OnExit**, **OnKeyPress**.
Funktionen: `get_text` (gesamter Text), `set_text`. Zeile anhängen:

```python
ui.set_text("Memo1", ui.get_text("Memo1") + "neue Zeile\n")
```

### GroupBox (TGroupBox) und Panel (TPanel)
Container. Beim Verschieben nehmen sie die darauf liegenden Komponenten mit.
GroupBox: **Caption**. Panel: **Caption**, **BevelOuter** (`bvNone`, `bvRaised`, `bvLowered`), **Color**.
Kein Ereignis. `set_text`, `set_enabled`, `set_visible` funktionieren.

### ProgressBar (TProgressBar)
Fortschrittsbalken. Eigenschaften: **Min**, **Max**, **Position**.
Kein Ereignis. Funktionen: `get_value`, `set_value`.

### TrackBar (TTrackBar)
Schieberegler. Eigenschaften: **Min**, **Max**, **Position**, **Orientation** (`trHorizontal`, `trVertical`).
Ereignis: **OnChange** (`TrackBar1Change`). Funktionen: `get_value`, `set_value`.

### SpinEdit (TSpinEdit)
Zahlenfeld mit Pfeilen. Eigenschaften: **MinValue**, **MaxValue**, **Value**.
Ereignis: **OnChange** (`SpinEdit1Change`). Funktionen: `get_value`, `set_value`, `get_text`.

### Image (TImage)
Bild. Eigenschaften: **Picture** (Dateipfad, relativ zum Projekt), **Stretch** (an Komponentengröße anpassen).
Kein Ereignis. Funktion: `set_image`.

### Timer (TTimer)
Nicht sichtbar, ruft in festem Takt eine Funktion auf. Eigenschaft: **Interval** (Millisekunden).
Ereignis: **OnTimer** (`Timer1Timer`). Funktionen: `set_enabled(name, True/False)` startet/stoppt,
`set_interval`. Ein Timer läuft ab Programmstart, wenn *Enabled* gesetzt ist.

```python
zaehler = 0
def Timer1Timer(ui):
    global zaehler
    zaehler += 1
    ui.set_text("Label1", str(zaehler))
```

---

## Formular

Eigenschaften des Formulars (Objektinspektor → `Form1`): **Caption**, **ClientWidth**, **ClientHeight**,
**Color**, **Position** (`poScreenCenter` zentriert das Fenster beim Start).
Die Größe lässt sich auch an den Kanten und Ecken des Formulars ziehen.

Ereignisse des Formulars:

| Ereignis | Funktion | Wann |
|---|---|---|
| **OnCreate** | `FormCreate(ui)` | einmal beim Start, nachdem alles aufgebaut ist – z. B. Listen füllen |
| **OnClose** | `FormClose(ui)` | beim Schließen. `return False` verhindert das Schließen |

```python
def FormClose(ui):
    if not ui.ask_yes_no("Programm wirklich beenden?"):
        return False
```

---

## Ereignis-Funktionen

* Name: `<Komponentenname><Endung>`. Endungen: `Click`, `Change`, `DblClick`, `Enter`, `Exit`,
  `KeyPress`, `Timer`; beim Formular `FormCreate`, `FormClose`.
* Umbenennen einer Komponente im Objektinspektor benennt die zugehörige Funktion in `_logic.py` mit um.
* Die Datei `<Projekt>_logic.py` wird von Pyphi7 **nie überschrieben**, nur fehlende Funktionen werden ergänzt.
* Neu angelegte Funktionen enthalten bei `…Click` eine Beispiel-Meldung, sonst `pass`.
* Im Code-Editor zeigt die linke Übersicht alle Objekte mit ihren Funktionen; ein Klick springt hin.

## Tastennamen

In `OnKeyPress`-Funktionen enthält `ui.key` entweder das getippte Zeichen (`"a"`, `"7"`, `","`) oder
einen Namen: `"Enter"`, `"Escape"`, `"Backspace"`, `"Tab"`, `"Up"`, `"Down"`, `"Left"`, `"Right"` u. a.

```python
def Edit1KeyPress(ui):
    if ui.key == "Enter":
        ui.set_text("Label1", ui.get_text("Edit1"))
```

## Eigene Funktionen

In `_logic.py` darfst du beliebig eigenen Python-Code schreiben: Hilfsfunktionen, globale Variablen,
`import`s anderer Module. Alle Funktionen, die kein Ereignis sind, nimmt Pyphi7 unverändert mit.
Zusätzliche Bibliotheken installierst du im Tab **Bibliotheken**.

```python
from decimal import Decimal

def euro(x):
    return f"{x:.2f} €".replace(".", ",")

def Button1Click(ui):
    betrag = Decimal(ui.get_text("Edit1").replace(",", "."))
    ui.add_item("ListBox1", euro(betrag))
```

---

## Übersicht

| Komponente | Wert lesen | Wert setzen | Ereignisse |
|---|---|---|---|
| Button | `get_text` | `set_text` | OnClick |
| Label | `get_text` | `set_text` | – |
| Edit | `get_text` | `set_text` | OnChange, OnEnter, OnExit, OnKeyPress |
| CheckBox | `is_checked` | `set_checked` | OnClick |
| RadioButton | `is_checked` | `set_checked` | OnClick |
| ListBox | `get_items`, `selected_index` | `set_items`, `add_item`, `set_selected_index` | OnClick, OnDblClick, OnEnter, OnExit, OnKeyPress |
| ComboBox | `get_text`, `get_items`, `selected_index` | `set_text`, `set_items`, `add_item`, `set_selected_index` | OnChange, OnEnter, OnExit |
| Memo | `get_text` | `set_text` | OnChange, OnEnter, OnExit, OnKeyPress |
| GroupBox / Panel | `get_text` | `set_text` | – |
| ProgressBar | `get_value` | `set_value` | – |
| TrackBar | `get_value` | `set_value` | OnChange |
| SpinEdit | `get_value` | `set_value` | OnChange |
| Image | – | `set_image` | – |
| Timer | – | `set_enabled`, `set_interval` | OnTimer |

Für alle sichtbaren Komponenten gelten außerdem `set_enabled` und `set_visible`.
