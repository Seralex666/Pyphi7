# -*- coding: utf-8 -*-
# AUTOMATISCH ERZEUGT von Pyphi7 (Qt / PySide6) - nicht von Hand aendern!
# Diese Datei wird bei jedem "Code erzeugen" neu geschrieben.
# Deine eigenen Funktionen gehoeren nach ausgaben_logic.py
import os
import sys

from PySide6.QtCore import Qt, QTimer, QEvent
from PySide6.QtGui import QColor, QPixmap
from PySide6.QtWidgets import (QApplication, QWidget, QPushButton, QLabel, QLineEdit,
                               QCheckBox, QRadioButton, QButtonGroup, QGroupBox, QFrame,
                               QProgressBar, QSlider, QSpinBox, QListWidget,
                               QPlainTextEdit, QComboBox, QMessageBox)

import ausgaben_logic as logic

_HERE = os.path.dirname(os.path.abspath(__file__))
_K = Qt.Key
_KEYNAMES = {_K.Key_Return: "Enter", _K.Key_Enter: "Enter", _K.Key_Escape: "Escape",
             _K.Key_Backspace: "Backspace", _K.Key_Tab: "Tab", _K.Key_Delete: "Delete",
             _K.Key_Up: "Up", _K.Key_Down: "Down", _K.Key_Left: "Left", _K.Key_Right: "Right",
             _K.Key_Home: "Home", _K.Key_End: "End", _K.Key_PageUp: "PageUp",
             _K.Key_PageDown: "PageDown", _K.Key_Insert: "Insert"}


class Window(QWidget):
    """Das Fenster. Deine Funktionen bekommen es als 'ui' uebergeben."""

    def __init__(self):
        self._app = QApplication.instance() or QApplication(sys.argv)
        super().__init__()
        self.setWindowTitle('Ausgaben')
        self.setFixedSize(480, 300)
        self.w = {}        # Name -> Qt-Widget
        self.kinds = {}    # Name -> Typ
        self._ev = {}      # Widget -> Ereignisse (Fokus, Tasten)
        self._groups = {}  # RadioButton-Gruppen
        self._on_close_fn = ""
        self._in_close = False
        self.key = ""      # zuletzt gedrueckte Taste (fuer OnKeyPress)

        # Label1 (Label)
        _w = QLabel('Kategorie:', self)
        _w.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop)
        self._add('Label1', 'Label', _w, 16, 16, 120, 17)

        # ListBox1 (Listbox)
        _w = QListWidget(self)
        _w.addItems(['Miete', 'Essen', 'Auto', 'Freizeit'])
        _w.setCurrentRow(0)
        _w.itemSelectionChanged.connect(lambda *_: self._call('ListBox1Click'))
        self._add('ListBox1', 'Listbox', _w, 16, 36, 144, 96)

        # Label2 (Label)
        _w = QLabel('Betrag in Euro:', self)
        _w.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop)
        self._add('Label2', 'Label', _w, 16, 148, 120, 17)

        # Edit1 (Entry)
        _w = QLineEdit('', self)
        self._watch(_w, key='Edit1KeyPress')
        _w.setToolTip('z. B. 12,50  (Enter = hinzufuegen)')
        self._add('Edit1', 'Entry', _w, 16, 168, 96, 21)

        # Button1 (Button)
        _w = QPushButton('+', self)
        _w.clicked.connect(lambda *_: self._call('Button1Click'))
        _f = _w.font()
        _f.setPointSize(12)
        _f.setBold(True)
        _w.setFont(_f)
        _w.setToolTip('Ausgabe hinzufuegen')
        self._add('Button1', 'Button', _w, 120, 166, 40, 25)

        # Button2 (Button)
        _w = QPushButton('Alles leeren', self)
        _w.clicked.connect(lambda *_: self._call('Button2Click'))
        self._add('Button2', 'Button', _w, 16, 252, 144, 25)

        # Label3 (Label)
        _w = QLabel('Ausgaben:', self)
        _w.setAlignment(Qt.AlignmentFlag.AlignLeft | Qt.AlignmentFlag.AlignTop)
        self._add('Label3', 'Label', _w, 176, 16, 120, 17)

        # Memo1 (Text)
        _w = QPlainTextEdit('', self)
        _w.setLineWrapMode(QPlainTextEdit.LineWrapMode.NoWrap)
        _w.setReadOnly(True)
        self._add('Memo1', 'Text', _w, 176, 36, 288, 196)

        # Label4 (Label)
        _w = QLabel('Summe: 0,00 €', self)
        _w.setAlignment(Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignTop)
        _f = _w.font()
        _f.setPointSize(12)
        _f.setBold(True)
        _w.setFont(_f)
        self._add('Label4', 'Label', _w, 176, 244, 288, 22)

        _scr = self._app.primaryScreen().availableGeometry()
        self.move(_scr.center() - self.rect().center())

        self._call('FormCreate')

    # ---- Hilfsfunktionen fuer deine Logik ------------------------------
    def _add(self, name, kind, widget, x, y, width, height):
        self.w[name] = widget
        self.kinds[name] = kind
        widget.setGeometry(x, y, width, height)

    def _timer(self, name, ms, enabled, fname):
        t = QTimer(self)
        t.setInterval(ms)
        if fname:
            t.timeout.connect(lambda: self._call(fname))
        self.w[name] = t
        self.kinds[name] = "Timer"
        if enabled:
            t.start()

    def _group(self, index):
        if index not in self._groups:
            self._groups[index] = QButtonGroup(self)
        return self._groups[index]

    def _watch(self, widget, **spec):
        """Fokus-/Tastenereignisse eines Widgets beobachten."""
        self._ev.setdefault(widget, {}).update(spec)
        widget.installEventFilter(self)

    def eventFilter(self, obj, ev):
        spec = self._ev.get(obj)
        if spec:
            t = ev.type()
            if t == QEvent.Type.FocusIn and "enter" in spec:
                self._call(spec["enter"])
            elif t == QEvent.Type.FocusOut and "exit" in spec:
                self._call(spec["exit"])
            elif t == QEvent.Type.KeyPress and "key" in spec:
                self.key = _KEYNAMES.get(ev.key()) or ev.text()
                self._call(spec["key"])
        return False

    def _call(self, fname):
        fn = getattr(logic, fname, None)
        if fn is None:
            QMessageBox.critical(self, "Funktion fehlt",
                                 "In ausgaben_logic.py fehlt: def %s(ui)" % fname)
            return None
        return fn(self)

    def closeEvent(self, ev):
        if self._on_close_fn and not self._in_close:
            self._in_close = True
            try:
                res = self._call(self._on_close_fn)
            finally:
                self._in_close = False
            if res is False:
                ev.ignore()
                return
        ev.accept()

    def widget(self, name):
        """Das echte Qt-Widget (fuer Spezialfaelle)."""
        return self.w[name]

    def get_text(self, name):
        wd, kind = self.w[name], self.kinds[name]
        if kind == "Text":
            return wd.toPlainText()
        if kind == "Combobox":
            return wd.currentText()
        if kind == "Listbox":
            item = wd.currentItem()
            return item.text() if item else ""
        if kind == "Spinedit":
            return str(wd.value())
        if kind in ("Progressbar", "Trackbar", "Image", "Timer"):
            return ""
        return wd.text()

    def set_text(self, name, text):
        wd, kind = self.w[name], self.kinds[name]
        text = str(text)
        if kind == "Text":
            wd.setPlainText(text)
        elif kind == "Combobox":
            if wd.isEditable():
                wd.setCurrentText(text)
            else:
                wd.setCurrentIndex(wd.findText(text))
        elif kind == "Spinedit":
            wd.setValue(int(float(text)))
        elif kind in ("Progressbar", "Trackbar", "Image", "Timer"):
            pass
        elif kind == "Groupbox":
            wd.setTitle(text)
        else:
            wd.setText(text)

    def get_value(self, name):
        """Zahlenwert von ProgressBar, TrackBar oder SpinEdit."""
        if self.kinds[name] not in ("Progressbar", "Trackbar", "Spinedit"):
            raise ValueError("%s hat keinen Zahlenwert" % name)
        return self.w[name].value()

    def set_value(self, name, value):
        if self.kinds[name] not in ("Progressbar", "Trackbar", "Spinedit"):
            raise ValueError("%s hat keinen Zahlenwert" % name)
        self.w[name].setValue(int(value))

    def is_checked(self, name):
        return self.w[name].isChecked()

    def set_checked(self, name, value=True):
        wd = self.w[name]
        grp = wd.group() if self.kinds[name] == "Radiobutton" else None
        if grp is not None and not value:
            grp.setExclusive(False)        # sonst kann man einen Radio nicht abwaehlen
            wd.setChecked(False)
            grp.setExclusive(True)
        else:
            wd.setChecked(bool(value))

    def set_items(self, name, items):
        wd = self.w[name]
        wd.clear()
        wd.addItems([str(i) for i in items])
        if self.kinds[name] == "Combobox":
            wd.setCurrentIndex(-1)

    def add_item(self, name, text):
        """Einen Eintrag ans Ende von Liste/Auswahlfeld haengen."""
        self.w[name].addItem(str(text))

    def get_items(self, name):
        wd = self.w[name]
        if self.kinds[name] == "Listbox":
            return [wd.item(i).text() for i in range(wd.count())]
        return [wd.itemText(i) for i in range(wd.count())]

    def selected_index(self, name):
        """Index des gewaehlten Eintrags (Liste/Auswahlfeld) oder -1."""
        wd = self.w[name]
        if self.kinds[name] == "Listbox":
            return wd.currentRow()
        return wd.currentIndex()

    def set_selected_index(self, name, index):
        wd = self.w[name]
        if self.kinds[name] == "Listbox":
            wd.setCurrentRow(index)
        else:
            wd.setCurrentIndex(index)

    def set_enabled(self, name, flag=True):
        wd = self.w[name]
        if self.kinds[name] == "Timer":
            wd.start() if flag else wd.stop()
        else:
            wd.setEnabled(bool(flag))

    def set_visible(self, name, flag=True):
        if self.kinds[name] != "Timer":
            self.w[name].setVisible(bool(flag))

    def set_interval(self, name, ms):
        """Takt eines Timers in Millisekunden aendern."""
        self.w[name].setInterval(int(ms))

    def set_image(self, name, path):
        """Bild laden (relativ zum Programmordner)."""
        full = path if os.path.isabs(path) else os.path.join(_HERE, path)
        pix = QPixmap(full) if path else QPixmap()
        self.w[name].setPixmap(pix)

    def message(self, text, title="Hinweis"):
        QMessageBox.information(self, title, str(text))

    def ask_yes_no(self, text, title="Frage"):
        return QMessageBox.question(self, title, str(text)) == QMessageBox.StandardButton.Yes

    # close() kommt von Qt selbst: ui.close() schliesst das Fenster (ruft vorher FormClose).

    def run(self):
        self.show()
        return self._app.exec()


if __name__ == "__main__":
    sys.exit(Window().run())
