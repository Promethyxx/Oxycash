"""
Oxycash – Design system
Mirrors the CSS variables from the HTML.
"""

# ─── Palette ──────────────────────────────────────────────────────────────────

DARK = {
    'bg':          '#0D0D0D',
    'bg2':         '#161614',
    'card':        '#1E1E1B',
    'card_border': '#2A2A26',
    'text':        '#E8E4DE',
    'text2':       '#9A9690',
    'text3':       '#4A4744',
    'red':         '#D96459',
    'teal':        '#85CDCA',
    'gold':        '#E8A87C',
    'amber':       '#F2D388',
    'brown':       '#C7B198',
    'green':       '#7BC47F',
    'danger':      '#E05555',
    'blue':        '#6FA8DC',
    'purple':      '#B794D6',
}

LIGHT = {
    'bg':          '#F5F4F0',
    'bg2':         '#ECEAE4',
    'card':        '#E4E2DC',
    'card_border': '#D0CEC8',
    'text':        '#2A2520',
    'text2':       '#6A6560',
    'text3':       '#A0A098',
    'red':         '#B8453A',
    'teal':        '#4A8E8A',
    'gold':        '#C97B4B',
    'amber':       '#C9A84C',
    'brown':       '#8E7E6A',
    'green':       '#4A9E4E',
    'danger':      '#C0453A',
    'blue':        '#4A7DB8',
    'purple':      '#8B5CA6',
}


class Theme:
    def __init__(self, dark: bool = True):
        self._dark  = dark
        self._c     = DARK if dark else LIGHT
        self.scale  = 0    # font size offset in px

    @property
    def is_dark(self): return self._dark

    def toggle(self):
        self._dark = not self._dark
        self._c    = DARK if self._dark else LIGHT

    def __getattr__(self, name):
        if name.startswith('_'):
            raise AttributeError(name)
        return self._c.get(name, '#FF00FF')   # magenta = missing key

    # shorthand helpers for Flet color args
    def c(self, key: str) -> str:
        return self._c.get(key, '#FF00FF')