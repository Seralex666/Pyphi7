# -*- coding: utf-8 -*-
# AUTOMATISCH ERZEUGT von Pyphi7 (Tkinter) - nicht von Hand aendern!
# Diese Datei wird bei jedem "Code erzeugen" neu geschrieben.
# Deine eigenen Funktionen gehoeren nach ausgaben_logic.py
import os
import tkinter as tk
from tkinter import ttk, messagebox

import ausgaben_logic as logic

_HERE = os.path.dirname(os.path.abspath(__file__))
_KEYNAMES = {"Return": "Enter", "KP_Enter": "Enter", "BackSpace": "Backspace",
             "Prior": "PageUp", "Next": "PageDown"}
_MODKEYS = ("Shift_L", "Shift_R", "Control_L", "Control_R", "Alt_L", "Alt_R",
            "Caps_Lock", "Meta_L", "Meta_R", "Super_L", "Super_R", "ISO_Level3_Shift")


class _Tip:
    """Kleiner Tooltip (Eigenschaft Hint)."""

    def __init__(self, widget, text):
        self.widget, self.text, self.tw, self.job = widget, text, None, None
        widget.bind("<Enter>", self._enter, add="+")
        widget.bind("<Leave>", self._leave, add="+")
        widget.bind("<ButtonPress>", self._leave, add="+")

    def _enter(self, _e):
        self._leave()
        self.job = self.widget.after(500, self._show)

    def _show(self):
        self.job = None
        x = self.widget.winfo_rootx() + 12
        y = self.widget.winfo_rooty() + self.widget.winfo_height() + 4
        self.tw = tw = tk.Toplevel(self.widget)
        tw.wm_overrideredirect(True)
        tw.wm_geometry("+%d+%d" % (x, y))
        tk.Label(tw, text=self.text, background="#ffffe1", foreground="black",
                 relief="solid", borderwidth=1, padx=3).pack()

    def _leave(self, _e=None):
        if self.job:
            self.widget.after_cancel(self.job)
            self.job = None
        if self.tw:
            self.tw.destroy()
            self.tw = None


class _Timer:
    """Timer-Komponente: ruft alle 'ms' Millisekunden deine Funktion auf."""

    def __init__(self, win, ms, enabled, fname):
        self.win, self.ms, self.fname = win, ms, fname
        self.on, self.job = False, None
        if enabled:
            self.start()

    def start(self):
        self.on = True
        if self.job is None:
            self.job = self.win.root.after(self.ms, self._tick)

    def stop(self):
        self.on = False
        if self.job is not None:
            self.win.root.after_cancel(self.job)
            self.job = None

    def _tick(self):
        self.job = None
        if not self.on:
            return
        self.job = self.win.root.after(self.ms, self._tick)
        if self.fname:
            self.win._call(self.fname)


def _fit(img, w, h):
    """Bild ohne Pillow grob auf die Komponente skalieren (ganzzahlig)."""
    iw, ih = img.width(), img.height()
    if iw <= 0 or ih <= 0:
        return img
    if iw > w or ih > h:
        f = max(-(-iw // max(w, 1)), -(-ih // max(h, 1)))
        return img.subsample(f, f)
    f = min(w // iw, h // ih)
    return img.zoom(f, f) if f > 1 else img


class Window:
    """Das Fenster. Deine Funktionen bekommen es als 'ui' uebergeben."""

    def __init__(self):
        self.root = tk.Tk()
        self.root.title('Ausgaben')
        self.root.geometry("480x300")
        self.root.resizable(False, False)
        self.w = {}        # Name -> Tk-Widget
        self.kinds = {}    # Name -> Typ
        self._geo = {}     # Name -> Position/Groesse
        self._vars = {}    # Variablen (Checkbox, Radiobutton, Edit, SpinEdit)
        self._groups = {}  # RadioButton-Gruppen
        self._min = {}     # Untergrenze der ProgressBar
        self._ro = set()   # schreibgeschuetzte Memos
        self._images = {}  # Bild-Referenzen (sonst verschwinden sie)
        self._stretch = {}
        self._fonts = []   # Font-Objekte am Leben halten
        self._on_close_fn = ""
        self._in_close = False
        self.key = ""      # zuletzt gedrueckte Taste (fuer OnKeyPress)
        self.root.protocol("WM_DELETE_WINDOW", self.close)

        # Label1 (Label)
        _w = ttk.Label(self.root, text='Kategorie:', anchor='nw', justify='left')
        self._add('Label1', 'Label', _w, 16, 16, 120, 17)

        # ListBox1 (Listbox)
        _w = tk.Listbox(self.root, exportselection=False)
        for _i in ['Miete', 'Essen', 'Auto', 'Freizeit']:
            _w.insert('end', _i)
        _w.selection_set(0)
        _w.bind('<<ListboxSelect>>', lambda e: self._call('ListBox1Click'))
        self._add('ListBox1', 'Listbox', _w, 16, 36, 144, 96)

        # Label2 (Label)
        _w = ttk.Label(self.root, text='Betrag in Euro:', anchor='nw', justify='left')
        self._add('Label2', 'Label', _w, 16, 148, 120, 17)

        # Edit1 (Entry)
        _sv = tk.StringVar(value='')
        self._vars['Edit1'] = _sv   # Referenz halten, sonst verfaellt der Trace
        _w = ttk.Entry(self.root, textvariable=_sv)
        _w.bind('<KeyPress>', lambda e: self._key(e, 'Edit1KeyPress'), add='+')
        _Tip(_w, 'z. B. 12,50  (Enter = hinzufuegen)')
        self._add('Edit1', 'Entry', _w, 16, 168, 96, 21)

        # Button1 (Button)
        _w = ttk.Button(self.root, text='+', command=lambda: self._call('Button1Click'), style=self._style('Button1', 'TButton', font=self._font(12, True)))
        _Tip(_w, 'Ausgabe hinzufuegen')
        self._add('Button1', 'Button', _w, 120, 166, 40, 25)

        # Button2 (Button)
        _w = ttk.Button(self.root, text='Alles leeren', command=lambda: self._call('Button2Click'))
        self._add('Button2', 'Button', _w, 16, 252, 144, 25)

        # Label3 (Label)
        _w = ttk.Label(self.root, text='Ausgaben:', anchor='nw', justify='left')
        self._add('Label3', 'Label', _w, 176, 16, 120, 17)

        # Memo1 (Text)
        _w = tk.Text(self.root, wrap='none')
        self._ro.add('Memo1')
        _w.configure(state='disabled')
        self._add('Memo1', 'Text', _w, 176, 36, 288, 196)

        # Label4 (Label)
        _w = ttk.Label(self.root, text='Summe: 0,00 €', anchor='ne', justify='right', font=self._font(12, True))
        self._add('Label4', 'Label', _w, 176, 244, 288, 22)

        self.root.update_idletasks()
        self.root.geometry("480x300+%d+%d" % ((self.root.winfo_screenwidth() - 480) // 2, (self.root.winfo_screenheight() - 300) // 2))

        self._call('FormCreate')

    # ---- Hilfsfunktionen fuer deine Logik ------------------------------
    def _add(self, name, kind, widget, x, y, width, height):
        self.w[name] = widget
        self.kinds[name] = kind
        self._geo[name] = dict(x=x, y=y, width=width, height=height)
        widget.place(**self._geo[name])

    def _timer(self, name, ms, enabled, fname):
        self.w[name] = _Timer(self, ms, enabled, fname)
        self.kinds[name] = "Timer"

    def _group(self, index):
        return self._groups.setdefault(index, tk.StringVar(value=""))

    def _font(self, size=0, bold=False):
        """Schrift passend zum Standardfont (size in Punkten, 0 = Standard)."""
        import tkinter.font as tkfont
        f = tkfont.nametofont("TkDefaultFont").copy()
        if size:
            f.configure(size=size)
        if bold:
            f.configure(weight="bold")
        self._fonts.append(f)
        return f

    def _style(self, name, base, font=None, fg=None):
        """Eigener ttk-Stil fuer Schrift und Farbe einer einzelnen Komponente."""
        sname = "%s.%s" % (name, base)
        opts = {}
        if font:
            opts["font"] = font
        if fg:
            opts["foreground"] = fg
        ttk.Style(self.root).configure(sname, **opts)
        return sname

    def _call(self, fname):
        fn = getattr(logic, fname, None)
        if fn is None:
            messagebox.showerror("Funktion fehlt",
                                 "In ausgaben_logic.py fehlt: def %s(ui)" % fname)
            return None
        return fn(self)

    def _key(self, e, fname):
        if e.keysym in _MODKEYS:
            return
        ch = e.char
        self.key = ch if ch and ch.isprintable() else _KEYNAMES.get(e.keysym, e.keysym)
        self._call(fname)

    def _modified(self, wd, fname):
        if wd.edit_modified():
            wd.edit_modified(False)
            self._call(fname)

    def widget(self, name):
        """Das echte Tk-Widget (fuer Spezialfaelle)."""
        return self.w[name]

    def get_text(self, name):
        wd, kind = self.w[name], self.kinds[name]
        if kind == "Text":
            return wd.get("1.0", "end-1c")
        if kind in ("Entry", "Combobox"):
            return wd.get()
        if kind == "Spinedit":
            return str(self.get_value(name))
        if kind == "Listbox":
            sel = wd.curselection()
            return wd.get(sel[0]) if sel else ""
        if kind in ("Progressbar", "Trackbar", "Image", "Timer"):
            return ""
        return wd.cget("text")

    def set_text(self, name, text):
        wd, kind = self.w[name], self.kinds[name]
        text = str(text)
        if kind == "Text":
            state = str(wd.cget("state"))
            wd.configure(state="normal")
            wd.delete("1.0", "end")
            wd.insert("1.0", text)
            wd.configure(state=state)
        elif kind == "Entry":
            self._vars[name].set(text)     # ein einziges OnChange
        elif kind == "Combobox":
            wd.set(text)
        elif kind == "Spinedit":
            self.set_value(name, int(float(text)))
        elif kind in ("Progressbar", "Trackbar", "Image", "Timer"):
            pass
        else:
            wd.configure(text=text)

    def get_value(self, name):
        """Zahlenwert von ProgressBar, TrackBar oder SpinEdit."""
        wd, kind = self.w[name], self.kinds[name]
        if kind == "Trackbar":
            return int(wd.get())
        if kind == "Progressbar":
            return int(float(wd.cget("value"))) + self._min[name]
        if kind == "Spinedit":
            try:
                return int(float(self._vars[name].get()))
            except ValueError:
                return 0
        raise ValueError("%s hat keinen Zahlenwert" % name)

    def set_value(self, name, value):
        wd, kind = self.w[name], self.kinds[name]
        value = int(value)
        if kind == "Trackbar":
            wd.set(value)
        elif kind == "Progressbar":
            wd.configure(value=value - self._min[name])
        elif kind == "Spinedit":
            lo, hi = int(float(wd.cget("from"))), int(float(wd.cget("to")))
            self._vars[name].set(str(max(lo, min(hi, value))))
        else:
            raise ValueError("%s hat keinen Zahlenwert" % name)

    def is_checked(self, name):
        if self.kinds[name] == "Radiobutton":
            return self._vars[name].get() == name
        return bool(self._vars[name].get())

    def set_checked(self, name, value=True):
        if self.kinds[name] == "Radiobutton":
            var = self._vars[name]
            if value:
                var.set(name)
            elif var.get() == name:
                var.set("")
        else:
            self._vars[name].set(bool(value))

    def set_items(self, name, items):
        wd, kind = self.w[name], self.kinds[name]
        items = [str(i) for i in items]
        if kind == "Listbox":
            state = str(wd.cget("state"))
            wd.configure(state="normal")
            wd.delete(0, "end")
            for i in items:
                wd.insert("end", i)
            wd.configure(state=state)
        elif kind == "Combobox":
            wd.configure(values=items)

    def add_item(self, name, text):
        """Einen Eintrag ans Ende von Liste/Auswahlfeld haengen."""
        wd, kind = self.w[name], self.kinds[name]
        if kind == "Listbox":
            wd.insert("end", str(text))
        elif kind == "Combobox":
            wd.configure(values=list(wd.cget("values")) + [str(text)])

    def get_items(self, name):
        wd, kind = self.w[name], self.kinds[name]
        if kind == "Listbox":
            return list(wd.get(0, "end"))
        return list(wd.cget("values"))

    def selected_index(self, name):
        """Index des gewaehlten Eintrags (Liste/Auswahlfeld) oder -1."""
        wd, kind = self.w[name], self.kinds[name]
        if kind == "Listbox":
            sel = wd.curselection()
            return sel[0] if sel else -1
        return wd.current()

    def set_selected_index(self, name, index):
        wd, kind = self.w[name], self.kinds[name]
        if kind == "Listbox":
            wd.selection_clear(0, "end")
            if index >= 0:
                wd.selection_set(index)
                wd.see(index)
        elif kind == "Combobox":
            if index < 0:
                wd.set("")
            else:
                wd.current(index)

    def set_enabled(self, name, flag=True):
        wd, kind = self.w[name], self.kinds[name]
        if kind == "Timer":
            wd.start() if flag else wd.stop()
        elif kind == "Text":
            wd.configure(state="normal" if flag and name not in self._ro else "disabled")
        elif kind in ("Listbox", "Trackbar", "Panel", "Image"):
            wd.configure(state="normal" if flag else "disabled")
        else:
            wd.state(["!disabled" if flag else "disabled"])

    def set_visible(self, name, flag=True):
        if self.kinds[name] == "Timer":
            return
        if flag:
            self.w[name].place(**self._geo[name])
        else:
            self.w[name].place_forget()

    def set_interval(self, name, ms):
        """Takt eines Timers in Millisekunden aendern."""
        self.w[name].ms = int(ms)

    def set_image(self, name, path):
        """Bild laden (relativ zum Programmordner). PNG/GIF; mit Pillow auch JPG u.a."""
        wd = self.w[name]
        full = path if os.path.isabs(path) else os.path.join(_HERE, path)
        w, h = self._geo[name]["width"], self._geo[name]["height"]
        img = None
        if path and os.path.exists(full):
            try:
                from PIL import Image as _PI, ImageTk as _PT
                im = _PI.open(full)
                if self._stretch.get(name, True):
                    im = im.resize((w, h))
                img = _PT.PhotoImage(im)
            except Exception:
                try:
                    img = tk.PhotoImage(file=full)
                    if self._stretch.get(name, True):
                        img = _fit(img, w, h)
                except tk.TclError:
                    img = None
        self._images[name] = img
        wd.configure(image=img if img else "")

    def message(self, text, title="Hinweis"):
        messagebox.showinfo(title, str(text))

    def ask_yes_no(self, text, title="Frage"):
        return messagebox.askyesno(title, str(text))

    def close(self):
        """Fenster schliessen (ruft vorher FormClose auf; False = nicht schliessen)."""
        if self._in_close:
            return
        self._in_close = True
        try:
            if self._on_close_fn and self._call(self._on_close_fn) is False:
                return
            self.root.destroy()
        finally:
            self._in_close = False

    def run(self):
        self.root.mainloop()


if __name__ == "__main__":
    Window().run()
