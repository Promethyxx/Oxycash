"""
Oxycash 2026 — Flet 0.82+
Run:   python main.py
Build: flet build windows / linux / apk
"""
from __future__ import annotations
import flet as ft

from core.model   import MONTHS, MNAMES, ALL_TABS, detect_budget_month
from core.storage import Storage
from core.theme   import Theme
from core.i18n    import T, set_lang
from views        import (
    build_month_view, build_dettes_view, build_epargne_view,
    build_frais_view, build_viabilite_view, build_config_view,
    build_charts_view,
)

SPECIAL_TABS_KEYS = ['Debts','Savings','Expenses','Viability','Charts','Config']

def get_tab_labels():
    return T.months_short + [
        T['tab_debts'], T['tab_savings'], T['tab_expenses'],
        T['tab_viability'], T['tab_charts'], T['tab_config'],
    ]

def get_all_tabs():
    return MONTHS + SPECIAL_TABS_KEYS

P = ft.Padding
B = ft.Border
BS = ft.BorderSide


def main(page: ft.Page):
    page.title   = 'Oxycash'
    page.bgcolor = '#0D0D0D'
    page.padding = 0
    page.fonts   = {
        'Playfair Display': 'https://fonts.gstatic.com/s/playfairdisplay/v37/nuFiD-vYSZviVYUb_rj3ij__anPXDTzYgEM86xRbAA.ttf',
        'DM Sans': 'https://fonts.gstatic.com/s/dmsans/v14/rP2Hp2ywxg089UriCZa4ET-DNl0.ttf',
    }

    # ── state ──
    storage  = Storage()
    theme    = Theme(dark=True)
    state    = {'tab': detect_budget_month()}

    def c(k): return theme.c(k)

    # ── toast (SnackBar via overlay — validated on Android) ─────────────────
    _snack = ft.SnackBar(ft.Text('', size=13, color='#1a1a1a', font_family='DM Sans',
                                  weight=ft.FontWeight.W_500),
                          bgcolor='#7BC47F', duration=2500)
    page.overlay.append(_snack)

    def show_toast(msg: str):
        _snack.content.value = msg
        _snack.open = True
        page.update()

    # ── badge ───────────────────────────────────────────────────────────────
    badge_txt = ft.Text('💾 Local', size=10, font_family='DM Sans',
                         weight=ft.FontWeight.W_600, color='#F2D388')
    badge = ft.Container(badge_txt,
                         padding=P.symmetric(horizontal=8, vertical=3),
                         border_radius=8, bgcolor='rgba(242,211,136,0.15)')

    def update_badge():
        s = storage.status()
        if s == 'dav':
            badge_txt.value  = '☁️ WebDAV';     badge_txt.color = '#7BC47F'
            badge.bgcolor    = 'rgba(123,196,127,0.15)'
        elif s == 'dav_err':
            badge_txt.value  = '⚠️ Déconnecté'; badge_txt.color = '#E05555'
            badge.bgcolor    = 'rgba(224,85,85,0.15)'
        else:
            badge_txt.value  = '💾 Local';      badge_txt.color = '#F2D388'
            badge.bgcolor    = 'rgba(242,211,136,0.15)'
        page.update()

    # ── content area ────────────────────────────────────────────────────────
    content_col = ft.Column(expand=True, scroll=ft.ScrollMode.AUTO)

    def on_save():
        def _do():
            storage.save()
            update_badge()
        page.run_thread(_do)

    def render():
        theme.scale = font_scale[0]
        tab = state['tab']

        # rebuild tabs
        tabs = []
        for i, key in enumerate(get_all_tabs()):
            is_act  = key == tab
            is_spec = i >= len(MONTHS)
            act_bg  = c('teal') if is_spec else c('gold')
            norm_fg = c('teal') if is_spec else c('text3')
            def make_tap(k):
                def _tap(e):
                    state['tab'] = k
                    render()
                return _tap
            tabs.append(ft.Container(
                ft.Text(get_tab_labels()[i], size=11, weight=ft.FontWeight.W_600,
                        font_family='DM Sans', no_wrap=True,
                        color='#1a1a1a' if is_act else norm_fg),
                padding=P.symmetric(horizontal=14, vertical=6),
                bgcolor=act_bg if is_act else 'transparent',
                border_radius=8,
                on_click=make_tap(key), ink=True,
            ))

        tab_row.controls.clear()
        tab_row.controls.extend(tabs)
        profile_text.value = storage.active_profile['name']
        tab_row.update()

        # rebuild content
        mi = MONTHS.index(tab) if tab in MONTHS else -1
        if mi >= 0:
            view = build_month_view(tab, storage.data.months[tab], theme, on_save, show_toast, all_months=storage.data.months, page=page, frais=storage.data.frais)
        elif tab == 'Debts':
            view = build_dettes_view(storage.data, theme, on_save, show_toast)
        elif tab == 'Savings':
            view = build_epargne_view(storage.data, theme, on_save, show_toast)
        elif tab == 'Expenses':
            view = build_frais_view(storage.data, theme, on_save, show_toast)
        elif tab == 'Viability':
            view = build_viabilite_view(storage.data, theme, on_save, show_toast)
        elif tab == 'Charts':
            view = build_charts_view(storage.data, theme, storage.currency)
        elif tab == 'Config':
            view = build_config_view(storage, theme, on_save, show_toast, render,
                                     lambda: (theme.toggle(), _apply_theme(), render()),
                                     page=page, on_badge=update_badge)
        else:
            view = ft.Text('?')

        content_col.controls.clear()
        content_col.controls.append(
            ft.Container(view, padding=P.symmetric(horizontal=14, vertical=12), expand=True)
        )
        content_col.update()
        page.update()

    def _apply_theme():
        page.bgcolor = c('bg')
        top_bar.bgcolor = c('bg2')
        page.update()

    # ── profile switcher ────────────────────────────────────────────────────
    def _profile_btn():
        prof = storage.active_profile
        def _go_config(e):
            state['tab'] = 'Config'
            render()
        return ft.Container(
            ft.Text(prof['name'], size=10, font_family='DM Sans',
                    weight=ft.FontWeight.W_600, color=c('teal'),
                    no_wrap=True),
            padding=P.symmetric(horizontal=8, vertical=4),
            border=B.all(1, c('teal')), border_radius=6,
            on_click=_go_config, ink=True,
        )

    # ── font scale ──────────────────────────────────────────────────────────
    font_scale = [0]   # offset in px, persisted in config

    font_scale[0] = storage.cfg.get('font_scale', 0)

    # ── lang toggle ─────────────────────────────────────────────────────────
    def _lang_display():
        from core.i18n import get_lang
        return 'FR' if get_lang() == 'en' else 'EN'

    # ── theme toggle ────────────────────────────────────────────────────────
    theme_icon = ft.Text('🌙', size=16)
    def toggle_theme(e):
        theme.toggle()
        theme_icon.value = '☀️' if not theme.is_dark else '🌙'
        _apply_theme()
        render()

    # ── top bar — all widgets are persistent (never recreated) ─────────────
    tab_row    = ft.Row([], spacing=2, scroll=ft.ScrollMode.AUTO)
    lang_text  = ft.Text(_lang_display(), size=10, font_family='DM Sans',
                         weight=ft.FontWeight.W_700, color=c('gold'))
    profile_text = ft.Text(storage.active_profile['name'], size=10,
                           font_family='DM Sans', weight=ft.FontWeight.W_600,
                           color=c('teal'), no_wrap=True)

    def _do_scale(delta):
        def _h(e):
            font_scale[0] = max(-6, min(8, font_scale[0] + delta))
            storage.cfg['font_scale'] = font_scale[0]
            from core.storage import save_config
            save_config(storage.cfg)
            render()
            show_toast(f'Police {"+" if delta > 0 else ""}{delta}')
        return _h

    def _do_lang(e):
        from core.i18n import toggle_lang, get_lang
        toggle_lang()
        storage.set_lang(get_lang())
        lang_text.value = _lang_display()
        render()
        show_toast('🌐 EN' if get_lang() == 'en' else '🌐 FR')

    def _do_profile(e):
        state['tab'] = 'Config'
        render()

    top_bar = ft.Container(
        ft.Column([
            ft.Row([
                ft.Row([
                    ft.Text('Oxycash', size=18, weight=ft.FontWeight.W_700,
                            font_family='Playfair Display', color=c('text')),
                    badge,
                    ft.Container(
                        profile_text,
                        padding=P.symmetric(horizontal=8, vertical=4),
                        border=B.all(1, c('teal')), border_radius=6,
                        on_click=_do_profile, ink=True,
                    ),
                ], spacing=6, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                ft.Row([
                    ft.Container(
                        ft.Text('A-', size=11, font_family='DM Sans',
                                weight=ft.FontWeight.W_700, color=c('text2')),
                        width=36, height=36,
                        border=B.all(1, c('card_border')), border_radius=6,
                        alignment=ft.Alignment(0, 0),
                        on_click=_do_scale(-2), ink=True,
                    ),
                    ft.Container(
                        ft.Text('A+', size=11, font_family='DM Sans',
                                weight=ft.FontWeight.W_700, color=c('text2')),
                        width=36, height=36,
                        border=B.all(1, c('card_border')), border_radius=6,
                        alignment=ft.Alignment(0, 0),
                        on_click=_do_scale(+2), ink=True,
                    ),
                    ft.Container(
                        lang_text,
                        padding=P.symmetric(horizontal=10, vertical=8),
                        border=B.all(1, c('gold')), border_radius=8,
                        on_click=_do_lang, ink=True,
                    ),
                    ft.Container(
                        theme_icon, width=36, height=36,
                        border_radius=18,
                        border=B.all(1, c('card_border')),
                        alignment=ft.Alignment(0, 0),
                        on_click=toggle_theme, ink=True,
                    ),
                ], spacing=6),
            ], alignment=ft.MainAxisAlignment.SPACE_BETWEEN,
               vertical_alignment=ft.CrossAxisAlignment.CENTER),
            ft.Container(tab_row, padding=P.only(bottom=10, top=10)),
        ], spacing=0),
        bgcolor=c('bg2'),
        padding=P.only(left=12, right=12, top=10, bottom=6),
        border=B.only(bottom=BS(1, c('card_border'))),
    )
    top_bar_container = top_bar

    # ── build initial content BEFORE page.add (validated pattern) ───────────
    # On Android, controls must have content when added to the page.
    # render() fills tab_row and content_col.
    theme.scale = font_scale[0]
    tab = state['tab']
    # Build tabs
    _init_tabs = []
    for i, key in enumerate(get_all_tabs()):
        is_act  = key == tab
        is_spec = i >= len(MONTHS)
        act_bg  = c('teal') if is_spec else c('gold')
        norm_fg = c('teal') if is_spec else c('text3')
        def _make_init_tap(k):
            def _tap(e):
                state['tab'] = k
                render()
            return _tap
        _init_tabs.append(ft.Container(
            ft.Text(get_tab_labels()[i], size=11, weight=ft.FontWeight.W_600,
                    font_family='DM Sans', no_wrap=True,
                    color='#1a1a1a' if is_act else norm_fg),
            padding=P.symmetric(horizontal=14, vertical=6),
            bgcolor=act_bg if is_act else 'transparent',
            border_radius=8,
            on_click=_make_init_tap(key), ink=True,
        ))
    tab_row.controls.extend(_init_tabs)

    # Build initial placeholder (real content after load)
    content_col.controls.append(
        ft.Container(
            ft.Column([
                ft.Text('Oxycash', size=20, weight=ft.FontWeight.W_700,
                        font_family='Playfair Display', color=c('text')),
                ft.Text('Chargement…', size=13, color=c('text3')),
            ], spacing=8),
            padding=P.symmetric(horizontal=14, vertical=12), expand=True,
        )
    )

    # ── root ────────────────────────────────────────────────────────────────
    page.add(ft.SafeArea(
        ft.Column([
            top_bar_container,
            ft.Container(content_col, expand=True),
        ], expand=True, spacing=0),
        expand=True,
    ))

    # ── load data then refresh (run_task AFTER page.add) ──────────────────
    async def _start():
        import asyncio
        await asyncio.get_event_loop().run_in_executor(None, storage.load)
        storage.load_lang()
        font_scale[0] = storage.cfg.get('font_scale', 0)
        update_badge()
        # Now render with real data — controls are already on page
        render()

    page.run_task(_start)


if __name__ == '__main__':
    ft.run(main)