"""Oxycash – Special views (Flet 0.82+) — fully editable"""
from __future__ import annotations
import sys, pathlib; sys.path.insert(0, str(pathlib.Path(__file__).parent.parent))
import flet as ft
from core.model import AppData, fmt, MONTHS
from core.i18n import T

P  = ft.Padding
M  = ft.Margin
BR = ft.BorderRadius
B  = ft.Border
BS = ft.BorderSide


_font_scale = [0]  # set by build_*_view via theme.scale

def _t(s, size=12, weight=ft.FontWeight.NORMAL, col='', family='DM Sans',
       align=ft.TextAlign.LEFT, expand=False, overflow=None, no_wrap=False):
    kw = dict(size=max(6, size + _font_scale[0]), weight=weight,
              font_family=family, text_align=align)
    if col:      kw['color']    = col
    if expand:   kw['expand']   = True
    if overflow: kw['overflow'] = overflow
    if no_wrap:  kw['no_wrap']  = True
    return ft.Text(str(s), **kw)


def _tf(value, on_change=None, on_blur=None, width=None, expand=False,
        num=False, col='', hint='', password=False, c=None):
    kw = dict(
        value=str(value), text_size=12, color=col,
        bgcolor='transparent', border_color=c('card_border'),
        focused_border_color=c('gold'),
        content_padding=P.symmetric(horizontal=8, vertical=6),
        hint_text=hint, password=password, can_reveal_password=password,
    )
    if width:  kw['width']  = width
    if expand: kw['expand'] = True
    if num:    kw['keyboard_type'] = ft.KeyboardType.NUMBER
    if on_change: kw['on_change'] = on_change
    if on_blur:   kw['on_blur']   = on_blur
    return ft.TextField(**kw)


def _sec_hdr(icon, label, total_str, col_key, c):
    return ft.Container(
        ft.Row([_t(icon, size=16, col=c('text')),
                _t(label, size=14, weight=ft.FontWeight.W_600,
                   family='Playfair Display', col=c('text'), expand=True),
                _t(total_str, size=13, weight=ft.FontWeight.W_700, col=c(col_key))],
               spacing=8),
        padding=P.only(bottom=6, top=4),
        border=B.only(bottom=BS(1, c('card_border'))),
    )


def _add_btn(label, on_click, c):
    return ft.Container(
        ft.Row([ft.Icon(ft.Icons.ADD, size=14, color=c('text3')),
                _t(label, size=12, col='text3')], spacing=6),
        padding=P.symmetric(horizontal=12, vertical=8),
        border=B.all(1, c('card_border')), border_radius=10,
        on_click=on_click, ink=True,
    )


def _del_btn(on_click, c):
    return ft.IconButton(
        ft.Icons.DELETE_OUTLINE, icon_size=16, icon_color=c('danger'),
        on_click=on_click, style=ft.ButtonStyle(padding=P.all(2)),
    )


# ═══ DETTES ════════════════════════════════════════════════════════

def build_dettes_view(data: AppData, t, on_save, on_toast, on_reload=None):
    def c(k): return t.c(k)
    _font_scale[0] = t.scale
    col = ft.Column([], spacing=8, scroll=ft.ScrollMode.AUTO, expand=True)
    dette_expanded = {}  # persisté entre rebuilds

    def rebuild():
        col.controls.clear()
        col.controls.extend(_build())
        col.update()

    def _build():
        total_du = sum(d.solde   for d in data.dettes)
        total_ok = sum(d.soldeOk for d in data.dettes)
        paid     = sum(1 for d in data.dettes if d.soldeOk >= d.solde)

        summary = ft.Container(
            ft.Row([
                ft.Column([_t(T['deb_total_due'], size=9, weight=ft.FontWeight.W_600, col=c('text3')),
                           _t(f"{fmt(total_du)} CHF", size=16, weight=ft.FontWeight.W_700, col=c('danger'))],
                          expand=True),
                ft.Column([_t(T['deb_negotiated'], size=9, weight=ft.FontWeight.W_600, col=c('text3')),
                           _t(f"{fmt(total_ok)} CHF", size=16, weight=ft.FontWeight.W_700, col=c('green'))],
                          expand=True),
                ft.Column([_t(T['deb_settled'], size=9, weight=ft.FontWeight.W_600, col=c('text3')),
                           _t(f"{paid}/{len(data.dettes)}", size=16, weight=ft.FontWeight.W_700, col=c('teal'))],
                          expand=True),
            ]),
            padding=14, bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=10,
        )

        cards = []
        for i, d in enumerate(data.dettes):
            is_paid  = d.soldeOk >= d.solde
            accent   = c('green') if is_paid else c('danger')
            is_open  = dette_expanded.get(i, False)

            def upd_str(field, i=i):
                def _h(e): setattr(data.dettes[i], field, e.control.value); on_save()
                return _h
            def upd_num(field, i=i):
                def _h(e):
                    try: setattr(data.dettes[i], field, float(e.control.value.replace(',','.'))); on_save(); rebuild()
                    except ValueError: pass
                return _h
            def del_dette(e, i=i):
                data.dettes.pop(i); on_save(); rebuild(); on_toast(T['toast_deleted'])
            def toggle_dette(e, i=i):
                dette_expanded[i] = not dette_expanded.get(i, False)
                rebuild()

            # ── ligne titre ──
            title_row = ft.Row([
                _tf(d.creancier, on_blur=upd_str('creancier'), expand=True, col=c('text'), c=c),
                ft.IconButton(
                    ft.Icons.EXPAND_MORE if is_open else ft.Icons.ADD,
                    icon_size=16,
                    icon_color=c('danger') if is_open else c('text3'),
                    on_click=toggle_dette,
                    style=ft.ButtonStyle(padding=P.all(2)),
                ),
                _tf(fmt(d.solde), on_blur=upd_num('solde'), num=True, width=82, col=c('danger'), c=c),
                _t('/', size=13, weight=ft.FontWeight.W_700, col=c('text3')),
                _tf(fmt(d.soldeOk), on_blur=upd_num('soldeOk'), num=True, width=82, col=c('green'), c=c),
                _del_btn(del_dette, c),
            ], spacing=4, vertical_alignment=ft.CrossAxisAlignment.CENTER)

            detail_parts = []
            if is_open:
                detail_parts = [
                    ft.Divider(height=1, color=c('card_border')),
                    ft.Row([
                        ft.Column([_t(T['deb_representative'], size=9, col=c('text3')),
                                   _tf(d.rep, on_blur=upd_str('rep'), col=c('text2'), c=c)],
                                  expand=True, spacing=2),
                        ft.Column([_t(T['deb_pursuit'], size=9, col=c('text3')),
                                   _tf(d.poursuite, on_blur=upd_str('poursuite'), col=c('text2'), c=c)],
                                  expand=True, spacing=2),
                    ], spacing=8),
                    ft.Row([
                        ft.Column([_t(T['deb_due_chf'], size=9, col=c('text3')),
                                   _tf(fmt(d.solde), on_blur=upd_num('solde'), num=True, col=c('danger'), c=c)],
                                  expand=True, spacing=2),
                        ft.Column([_t(T['deb_neg_chf'], size=9, col=c('text3')),
                                   _tf(fmt(d.soldeOk), on_blur=upd_num('soldeOk'), num=True, col=c('green'), c=c)],
                                  expand=True, spacing=2),
                    ], spacing=8),
                    ft.Row([
                        ft.Column([_t(T['deb_status'], size=9, col=c('text3')),
                                   _tf(d.etat, on_blur=upd_str('etat'), col=c('text2'), c=c)],
                                  expand=True, spacing=2),
                        ft.Column([_t(T['deb_date'], size=9, col=c('text3')),
                                   _tf(d.date, on_blur=upd_str('date'), col=c('text3'), c=c)],
                                  expand=True, spacing=2),
                    ], spacing=8),
                    ft.Row([
                        ft.Column([_t(T.get('deb_ref', 'Référence'), size=9, col=c('text3')),
                                   _tf(d.ref, on_blur=upd_str('ref'), col=c('text2'), c=c)],
                                  expand=True, spacing=2),
                    ], spacing=8),
                    ft.Column([
                        _t(T.get('deb_note', 'Note'), size=9, col=c('text3')),
                        ft.TextField(
                            value=d.note, multiline=True, min_lines=2, max_lines=4,
                            text_size=12, color=c('text2'), bgcolor='transparent',
                            border_color=c('card_border'), focused_border_color=c('gold'),
                            content_padding=P.symmetric(horizontal=8, vertical=6),
                            on_blur=upd_str('note'),
                        ),
                    ], spacing=2),
                ]

            cards.append(ft.Container(
                ft.Column([title_row, *detail_parts], spacing=6),
                padding=14, bgcolor=c('card'),
                border=B.only(left=BS(3, accent), top=BS(1, c('card_border')),
                              right=BS(1, c('card_border')), bottom=BS(1, c('card_border'))),
                border_radius=10, opacity=0.75 if is_paid else 1.0,
            ))

        def add_dette(e):
            from core.model import Dette
            data.dettes.append(Dette(rep='', creancier='Nouveau', poursuite='',
                                     solde=0, soldeOk=0, etat='', date='', ref='', note=''))
            on_save(); rebuild()

        return [
            _t(T['deb_title'], size=20, weight=ft.FontWeight.W_700, family='Playfair Display', col=c('text')),
            _t(T['deb_subtitle'], size=12, col=c('text2')),
            summary, *cards,
            _add_btn(T['deb_add'], add_dette, c),
            ft.Container(height=40),
        ]

    col.controls.extend(_build())
    return col


# ═══ EPARGNE ════════════════════════════════════════════════════════

def build_epargne_view(data: AppData, t, on_save, on_toast, on_reload=None):
    def c(k): return t.c(k)
    _font_scale[0] = t.scale
    col = ft.Column([], spacing=6, scroll=ft.ScrollMode.AUTO, expand=True)

    def rebuild():
        col.controls.clear()
        col.controls.extend(_build())
        col.update()

    def _build():
        ep = data.epargne
        # ── Projets épargne ──
        def _progress_row(pct, done):
            bar_col = c('green') if done else c('teal')
            return ft.Row([
                ft.Container(height=6, bgcolor=bar_col,
                             border_radius=3, expand=max(1, pct)),
                ft.Container(height=6, bgcolor=c('card_border'),
                             border_radius=3, expand=max(1, 100 - pct)),
            ], spacing=0, expand=True)

        savings_blocks = []
        for si, sv in enumerate(ep.get('savings', [])):
            # tri alpha des rows
            sv.setdefault('rows', []).sort(key=lambda r: r.get('name', '').lower())

            sv_total      = sum(r.get('montant', 0) for r in sv['rows'])
            rows_cible    = sum(r.get('cible', 0) for r in sv['rows'])
            has_rows      = len(sv['rows']) > 0
            # objectif : somme des entrées si elles ont des objectifs, sinon valeur manuelle
            sv_cible      = rows_cible if (has_rows and rows_cible > 0.01) else sv.get('cible', 0)
            sv_reste      = sv_cible - sv_total if sv_cible > 0.01 else 0
            sv_pct        = min(100, int(sv_total / sv_cible * 100)) if sv_cible > 0.01 else 0
            sv_done       = sv_cible > 0.01 and sv_reste <= 0.01
            sv_ck         = 'green' if sv_done else 'gold'
            is_open       = sv.get('open', True)

            def upd_sv_label(e, si=si):
                ep['savings'][si]['label'] = e.control.value; on_save()
            def upd_sv_montant(e, si=si):
                try: ep['savings'][si]['montant'] = float(e.control.value.replace(',','.')); on_save(); rebuild()
                except ValueError: pass
            def upd_sv_cible(e, si=si):
                try: ep['savings'][si]['cible'] = float(e.control.value.replace(',','.')); on_save(); rebuild()
                except ValueError: pass
            def del_sv(e, si=si):
                ep['savings'].pop(si); on_save(); rebuild()
            def toggle_sv(e, si=si):
                ep['savings'][si]['open'] = not ep['savings'][si].get('open', True)
                on_save(); rebuild()

            # ── ligne titre (nom + montant / objectif + expand + del) ──
            # si pas d'entrées : champ éditable ; sinon : read-only = somme des entrées
            montant_display = fmt(sv_total) if has_rows else fmt(sv.get('montant', 0))
            montant_widget = ft.TextField(
                value=montant_display,
                width=82, text_size=12,
                text_align=ft.TextAlign.RIGHT,
                color=c('teal'), bgcolor='transparent',
                border_color=c('card_border') if not has_rows else 'transparent',
                focused_border_color=c('gold'),
                content_padding=P.symmetric(horizontal=6, vertical=2),
                read_only=has_rows,
                keyboard_type=ft.KeyboardType.NUMBER,
                on_blur=upd_sv_montant if not has_rows else None,
                hint_text='Épargné',
            )
            title_row = ft.Row([
                _tf(sv.get('label', 'Projet'), on_blur=upd_sv_label,
                    expand=True, col=c('text'), c=c),
                ft.IconButton(
                    ft.Icons.EXPAND_MORE if is_open else ft.Icons.ADD,
                    icon_size=16,
                    icon_color=c('gold') if is_open else c('text3'),
                    on_click=toggle_sv,
                    style=ft.ButtonStyle(padding=P.all(2)),
                ),
                montant_widget,
                _t('/', size=14, weight=ft.FontWeight.W_700, col=c('text3')),
                ft.TextField(
                    value=fmt(sv_cible) if sv_cible > 0 else '',
                    width=82, text_size=12,
                    text_align=ft.TextAlign.RIGHT,
                    color=c('gold'), bgcolor='transparent',
                    border_color=c('card_border') if not (has_rows and rows_cible > 0.01) else 'transparent',
                    focused_border_color=c('gold'),
                    content_padding=P.symmetric(horizontal=6, vertical=2),
                    read_only=has_rows and rows_cible > 0.01,
                    keyboard_type=ft.KeyboardType.NUMBER,
                    on_blur=upd_sv_cible if not (has_rows and rows_cible > 0.01) else None,
                    hint_text='Objectif',
                ),
                _del_btn(del_sv, c),
            ], spacing=4, vertical_alignment=ft.CrossAxisAlignment.CENTER)

            body_parts = []
            if is_open:
                if sv_cible > 0.01:
                    body_parts += [
                        ft.Divider(height=1, color=c('card_border')),
                        ft.Row([
                            _progress_row(sv_pct, sv_done),
                            _t(f"{sv_pct}%", size=10, weight=ft.FontWeight.W_700, col=c(sv_ck)),
                        ], spacing=8, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                    ]

                # ── lignes ──
                row_widgets = []
                for ri, r in enumerate(sv['rows']):
                    r_montant = r.get('montant', 0)
                    r_cible   = r.get('cible', 0)
                    r_pct     = min(100, int(r_montant / r_cible * 100)) if r_cible > 0.01 else 0
                    r_done    = r_cible > 0.01 and r_montant >= r_cible
                    r_ck      = 'green' if r_done else 'teal'

                    def upd_rname(e, si=si, ri=ri):
                        ep['savings'][si]['rows'][ri]['name'] = e.control.value
                        ep['savings'][si]['rows'].sort(key=lambda r: r.get('name','').lower())
                        on_save(); rebuild()
                    def upd_rmontant(e, si=si, ri=ri):
                        try: ep['savings'][si]['rows'][ri]['montant'] = float(e.control.value.replace(',','.')); on_save(); rebuild()
                        except ValueError: pass
                    def upd_rcible(e, si=si, ri=ri):
                        try: ep['savings'][si]['rows'][ri]['cible'] = float(e.control.value.replace(',','.')); on_save(); rebuild()
                        except ValueError: pass
                    def del_row(e, si=si, ri=ri):
                        ep['savings'][si]['rows'].pop(ri); on_save(); rebuild()

                    line_parts = [
                        ft.Row([
                            _tf(r.get('name', ''), on_blur=upd_rname,
                                expand=True, col=c('text'), c=c),
                            _tf(fmt(r_montant), on_blur=upd_rmontant, num=True,
                                width=82, col=c('teal'), hint='Épargné', c=c),
                            _t('/', size=11, col=c('text3')),
                            _tf(fmt(r_cible) if r_cible > 0 else '', on_blur=upd_rcible,
                                num=True, width=82, col=c('gold'), hint='Objectif', c=c),
                            _del_btn(del_row, c),
                        ], spacing=4, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                    ]
                    if r_cible > 0.01:
                        line_parts.append(
                            ft.Row([
                                _progress_row(r_pct, r_done),
                                _t(f"{r_pct}%", size=9, weight=ft.FontWeight.W_700, col=c(r_ck)),
                            ], spacing=6, vertical_alignment=ft.CrossAxisAlignment.CENTER)
                        )
                    row_widgets.append(ft.Container(
                        ft.Column(line_parts, spacing=4),
                        padding=P.symmetric(horizontal=12, vertical=8),
                        bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=8,
                    ))

                def add_sv_row(e, si=si):
                    ep['savings'][si]['rows'].append({'name': 'Nouveau', 'montant': 0, 'cible': 0})
                    ep['savings'][si]['rows'].sort(key=lambda r: r.get('name','').lower())
                    on_save(); rebuild()

                body_parts += [
                    ft.Divider(height=1, color=c('card_border')),
                    *row_widgets,
                    _add_btn(T.get('sav_add_saving', '+ Entrée'), add_sv_row, c),
                ]

            savings_blocks.append(ft.Container(
                ft.Column([title_row, *body_parts], spacing=6),
                padding=14, bgcolor=c('card'),
                border=B.all(1, c('gold') if is_open else c('card_border')), border_radius=10,
            ))

        def add_savings(e):
            ep.setdefault('savings', []).append({'label': 'Projet', 'cible': 0, 'rows': []})
            on_save(); rebuild()

        return [
            _t(T['sav_title'], size=20, weight=ft.FontWeight.W_700, family='Playfair Display', col=c('text')),
            *savings_blocks,
            _add_btn(T.get('sav_add_project', '+ Projet'), add_savings, c),
            ft.Container(height=40),
        ]

    col.controls.extend(_build())
    return col


# ═══ FRAIS ══════════════════════════════════════════════════════════

def build_frais_view(data: AppData, t, on_save, on_toast, on_reload=None):
    def c(k): return t.c(k)
    _font_scale[0] = t.scale
    col = ft.Column([], spacing=12, scroll=ft.ScrollMode.AUTO, expand=True)
    frais_expanded = {'fixes': True, 'ponctuels': True, 'retraits': True}

    def rebuild():
        col.controls.clear()
        col.controls.extend(_build())
        col.update()

    def _build():
        def table(cat, label, col_key):
            lines  = sorted(data.frais.get(cat, []), key=lambda l: l.name.lower())
            # réordonner la liste source aussi pour que les index soient cohérents
            data.frais[cat] = lines
            totals = [sum(l.monthly[mi] for l in lines) for mi in range(12)]
            grand  = sum(totals)

            def hdr(s, ck, width=None):
                ct = _t(s, size=9, weight=ft.FontWeight.W_600, col=c(ck), align=ft.TextAlign.CENTER)
                return ft.Container(ct, width=width) if width else ft.Container(ct, expand=True)

            header = ft.Row(
                [hdr(T['exp_post'], 'text3', width=72)] +
                [hdr(MONTHS[mi][:3], 'text3') for mi in range(12)] +
                [hdr(T['exp_total'], col_key, width=52), ft.Container(width=28)],
                spacing=2,
            )
            rows = [header, ft.Divider(height=1, color=c('card_border'))]

            for li, fl in enumerate(lines):
                yr = sum(fl.monthly)

                def upd_name(e, li=li, cat=cat):
                    data.frais[cat][li].name = e.control.value; on_save()

                month_cells = []
                for mi in range(12):
                    def upd_m(e, li=li, mi=mi, cat=cat):
                        try:
                            data.frais[cat][li].monthly[mi] = float(e.control.value.replace(',','.') or '0')
                            on_save(); rebuild()
                        except ValueError: pass
                    month_cells.append(ft.Container(
                        ft.TextField(
                            value=fmt(fl.monthly[mi]) if fl.monthly[mi] else '',
                            hint_text='0', text_size=9,
                            text_align=ft.TextAlign.CENTER,
                            color=c('text'), bgcolor='transparent',
                            border_color=c('card_border'),
                            focused_border_color=c('gold'),
                            content_padding=P.symmetric(horizontal=2, vertical=4),
                            keyboard_type=ft.KeyboardType.NUMBER,
                            on_blur=upd_m,
                        ), expand=True,
                    ))

                def del_line(e, li=li, cat=cat):
                    data.frais[cat].pop(li); on_save(); rebuild()

                rows.append(ft.Row(
                    [ft.Container(
                        ft.TextField(value=fl.name, text_size=10, color=c('text2'),
                                     bgcolor='transparent', border_color='transparent',
                                     focused_border_color=c('gold'),
                                     content_padding=P.symmetric(horizontal=4, vertical=4),
                                     on_blur=upd_name),
                        width=72)] +
                    month_cells +
                    [ft.Container(_t(fmt(yr), size=10, weight=ft.FontWeight.W_700,
                                      col=c(col_key), align=ft.TextAlign.RIGHT), width=52),
                     _del_btn(del_line, c)],
                    spacing=2,
                ))

            rows.append(ft.Divider(height=1, color=c('card_border')))
            rows.append(ft.Row(
                [ft.Container(_t(T['col_total'], size=10, weight=ft.FontWeight.W_700, col=c('text')), width=72)] +
                [ft.Container(_t(fmt(totals[mi]) if totals[mi] else '-', size=9,
                                  weight=ft.FontWeight.W_700, col=c(col_key),
                                  align=ft.TextAlign.CENTER), expand=True)
                 for mi in range(12)] +
                [ft.Container(_t(fmt(grand), size=10, weight=ft.FontWeight.W_700,
                                  col=c(col_key), align=ft.TextAlign.RIGHT), width=52),
                 ft.Container(width=28)],
                spacing=2,
            ))

            def add_line(e, cat=cat):
                from core.model import FraisLine
                data.frais[cat].append(FraisLine('Nouveau', [0.0]*12)); on_save(); rebuild()

            is_open = frais_expanded.get(cat, True)

            def toggle_table(e, cat=cat):
                frais_expanded[cat] = not frais_expanded.get(cat, True)
                rebuild()

            title_row = ft.Container(
                ft.Row([
                    _t(label, size=13, weight=ft.FontWeight.W_600,
                       family='Playfair Display', col=c('text'), expand=True),
                    _t(fmt(grand), size=12, weight=ft.FontWeight.W_700, col=c(col_key)),
                    ft.IconButton(
                        ft.Icons.EXPAND_MORE if is_open else ft.Icons.ADD,
                        icon_size=16,
                        icon_color=c(col_key) if is_open else c('text3'),
                        on_click=toggle_table,
                        style=ft.ButtonStyle(padding=P.all(2)),
                    ),
                ], spacing=8, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                padding=P.only(bottom=6, top=4),
                border=B.only(bottom=BS(1, c('card_border'))),
            )

            body = []
            if is_open:
                body = [
                    ft.Container(ft.Column(rows, spacing=4), padding=12,
                                 bgcolor=c('card'), border=B.all(1, c('card_border')),
                                 border_radius=10, clip_behavior=ft.ClipBehavior.HARD_EDGE),
                    _add_btn(T['exp_add_line'], add_line, c),
                ]

            return ft.Column([title_row, *body], spacing=6)

        return [
            _t(T['exp_title'], size=20, weight=ft.FontWeight.W_700,
               family='Playfair Display', col=c('text')),
            table('fixes',     T['exp_fixed'],       'blue'),
            table('ponctuels', T['exp_occasional'],  'amber'),
            table('retraits',  T['exp_withdrawals'], 'teal'),
            ft.Container(height=40),
        ]

    col.controls.extend(_build())
    return col


# ═══ VIABILITE ══════════════════════════════════════════════════════

def build_viabilite_view(data: AppData, t, on_save, on_toast, on_reload=None):
    def c(k): return t.c(k)
    _font_scale[0] = t.scale
    col = ft.Column([], spacing=10, scroll=ft.ScrollMode.AUTO, expand=True)

    def rebuild():
        col.controls.clear()
        col.controls.extend(_build())
        col.update()

    def _next_val(val, dt, dv):
        """Applique un delta (fixed ou pct) à une valeur."""
        if dt == 'pct':
            return round(val * (1 + dv / 100), 2)
        return round(val + dv, 2)

    def _compute_solde(vc, palier):
        s = 0.0
        for ci, col_def in enumerate(vc.colonnes):
            v = palier.valeurs[ci] if ci < len(palier.valeurs) else 0.0
            s += v if col_def.is_income else -v
        return s

    def _build():
        from core.model import ViabiliteColonne, ViabilitePalier, ViabiliteConfig
        vc = data.viabilite  # ViabiliteConfig

        # ── Section config colonnes ──
        # pct_bases : dict ci -> liste de ci-income servant de base (stocké sur vc)
        if not hasattr(vc, 'pct_bases') or not isinstance(getattr(vc, 'pct_bases', None), dict):
            vc.pct_bases = {}

        col_rows = []
        # Pour le panneau de sélection de base % : on garde une ref au container expandable par colonne
        pct_panels = {}  # ci -> ft.Column (le sous-panneau)

        income_cols = [(ci, cd) for ci, cd in enumerate(vc.colonnes) if cd.is_income]

        for ci, col_def in enumerate(vc.colonnes):
            def upd_cname(e, ci=ci):
                vc.colonnes[ci].name = e.control.value; on_save()
            def upd_ctype(e, ci=ci):
                was_pct = vc.colonnes[ci].delta_type == 'pct'
                vc.colonnes[ci].delta_type = 'fixed' if was_pct else 'pct'
                on_save(); rebuild()
            def del_col(e, ci=ci):
                vc.colonnes.pop(ci)
                for p in vc.paliers:
                    if ci < len(p.valeurs): p.valeurs.pop(ci)
                # nettoyer pct_bases
                vc.pct_bases.pop(str(ci), None)
                on_save(); rebuild()
            def toggle_income(e, ci=ci):
                vc.colonnes[ci].is_income = not vc.colonnes[ci].is_income
                on_save(); rebuild()

            # ── bouton +/− (is_income) ──
            income_btn = ft.Container(
                _t('+' if col_def.is_income else '−', size=12,
                   weight=ft.FontWeight.W_700,
                   col=c('teal') if col_def.is_income else c('danger')),
                width=24, height=24,
                border=B.all(1, c('teal') if col_def.is_income else c('danger')),
                border_radius=4,
                alignment=ft.Alignment(0, 0),
                on_click=toggle_income, ink=True,
            )

            # ── bouton +/% devant le 1er champ (valeur_type : montant fixe ou % d'une base) ──
            if not hasattr(col_def, 'valeur_type'):
                col_def.valeur_type = 'fixed'

            def upd_cvaleur(e, ci=ci):
                try: vc.colonnes[ci].valeur = float(e.control.value.replace(',','.') or '0'); on_save()
                except ValueError: pass

            def toggle_valeur_type(e, ci=ci):
                cd = vc.colonnes[ci]
                if not hasattr(cd, 'valeur_type'): cd.valeur_type = 'fixed'
                cd.valeur_type = 'pct' if cd.valeur_type == 'fixed' else 'fixed'
                on_save(); rebuild()

            # Clic sur le champ valeur en mode % : ouvre/ferme sous-panneau bases
            if not hasattr(vc, '_val_open'):
                vc._val_open = set()

            def open_val_panel(e, ci=ci):
                if ci in vc._val_open:
                    vc._val_open.discard(ci)
                else:
                    vc._val_open.add(ci)
                rebuild()

            vtype_is_pct = getattr(col_def, 'valeur_type', 'fixed') == 'pct'
            valeur_type_btn = ft.Container(
                _t('%' if vtype_is_pct else '+', size=11, weight=ft.FontWeight.W_700,
                   col=c('gold') if vtype_is_pct else c('text3'),
                   align=ft.TextAlign.CENTER),
                width=24, height=24,
                border=B.all(1, c('gold') if vtype_is_pct else c('card_border')),
                border_radius=4,
                alignment=ft.Alignment(0, 0),
                on_click=toggle_valeur_type, ink=True,
            )

            valeur_hint = '%' if vtype_is_pct else (T.get('via_income_lbl', 'Actuel') if col_def.is_income else T.get('via_charge_lbl', 'Augm.'))

            if vtype_is_pct:
                def make_focus(ci=ci):
                    def _h(e, ci=ci):
                        if ci not in vc._val_open:
                            vc._val_open.add(ci)
                            rebuild()
                    return _h
                valeur_field = ft.TextField(
                    value=fmt(col_def.valeur), text_size=12,
                    color=c('teal') if col_def.is_income else c('text2'),
                    bgcolor='transparent', border_color=c('card_border'),
                    focused_border_color=c('gold'),
                    content_padding=P.symmetric(horizontal=8, vertical=6),
                    hint_text=valeur_hint, width=80,
                    keyboard_type=ft.KeyboardType.NUMBER,
                    on_blur=upd_cvaleur,
                    on_focus=make_focus(ci),
                )
            else:
                valeur_field = _tf(fmt(col_def.valeur), on_blur=upd_cvaleur, num=True,
                                   width=80, col=c('teal') if col_def.is_income else c('text2'),
                                   hint=valeur_hint, c=c)

            # ── bouton delta +/% (type d'augmentation) — toujours bascule ──
            def make_type_click(ci=ci):
                def _h(e, ci=ci):
                    vc.colonnes[ci].delta_type = 'fixed' if vc.colonnes[ci].delta_type == 'pct' else 'pct'
                    on_save(); rebuild()
                return _h

            delta_hint = '%' if col_def.delta_type == 'pct' else '+'
            type_btn = ft.Container(
                _t('%' if col_def.delta_type == 'pct' else '+',
                   size=11, weight=ft.FontWeight.W_700, col=c('gold')),
                width=24, height=24,
                border=B.all(1, c('gold')), border_radius=4,
                alignment=ft.Alignment(0, 0),
                on_click=make_type_click(ci), ink=True,
            )

            def upd_cdelta(e, ci=ci):
                try: vc.colonnes[ci].delta_val = float(e.control.value.replace(',','.') or '0'); on_save()
                except ValueError: pass

            main_row = ft.Row([
                income_btn,
                _tf(col_def.name, on_blur=upd_cname, expand=True, col=c('text'), c=c),
                valeur_type_btn,
                valeur_field,
                type_btn,
                _tf(fmt(col_def.delta_val), on_blur=upd_cdelta, num=True,
                    width=64, col=c('gold'), hint=delta_hint, c=c),
                _del_btn(del_col, c),
            ], spacing=6, vertical_alignment=ft.CrossAxisAlignment.CENTER)

            # ── sous-panneau bases pour valeur_type='pct' ──
            col_parts = [main_row]
            if vtype_is_pct and ci in getattr(vc, '_val_open', set()):
                bases_key = str(ci)
                selected = set(vc.pct_bases.get(bases_key, []))

                def make_toggle_base(ci=ci, bci=0):
                    def _h(e, ci=ci, bci=bci):
                        key = str(ci)
                        sel = set(vc.pct_bases.get(key, []))
                        if bci in sel: sel.discard(bci)
                        else: sel.add(bci)
                        vc.pct_bases[key] = list(sel)
                        on_save(); rebuild()
                    return _h

                base_checks = []
                for bci, bcd in income_cols:
                    is_sel = bci in selected
                    base_checks.append(ft.Container(
                        ft.Row([
                            ft.Container(
                                _t('✓' if is_sel else '○', size=11,
                                   col=c('gold') if is_sel else c('text3'),
                                   weight=ft.FontWeight.W_700),
                                width=20,
                            ),
                            _t(bcd.name, size=11, col=c('text2') if is_sel else c('text3')),
                        ], spacing=6),
                        on_click=make_toggle_base(ci, bci),
                        ink=True,
                        padding=P.symmetric(horizontal=8, vertical=4),
                        border_radius=6,
                        bgcolor=c('card_border') if is_sel else 'transparent',
                    ))

                pct_panel = ft.Container(
                    ft.Column([
                        _t(T.get('via_pct_base', 'Base du calcul %'), size=9,
                           col=c('text3'), weight=ft.FontWeight.W_600),
                        *base_checks,
                    ], spacing=2),
                    padding=P.only(left=48, top=4, bottom=4, right=8),
                    border=B.only(left=BS(2, c('gold'))),
                )
                col_parts.append(pct_panel)

            col_rows.append(ft.Column(col_parts, spacing=4))

        def add_col(e):
            vc.colonnes.append(ViabiliteColonne('Nouveau', 0, False, 'fixed', 0))
            for p in vc.paliers:
                p.valeurs.append(0.0)
            on_save(); rebuild()

        config_card = ft.Container(
            ft.Column([
                ft.Row([
                    _t(T.get('via_columns', 'Colonnes'), size=12,
                       weight=ft.FontWeight.W_600, col=c('text2'), expand=True),
                    _t(T.get('via_income_legend', 'Actuel / Augmentation'), size=9, col=c('text3')),
                ]),
                ft.Divider(height=1, color=c('card_border')),
                *col_rows,
                _add_btn(T.get('via_add_col', '+ Colonne'), add_col, c),
            ], spacing=6),
            padding=14, bgcolor=c('card'),
            border=B.all(1, c('card_border')), border_radius=10,
        )

        # ── Générateur ──
        def upd_npal(e):
            try: vc.n_paliers = max(1, int(e.control.value.replace(',','.') or '1')); on_save()
            except ValueError: pass

        def generate(e):
            if not hasattr(vc, 'pct_bases') or not isinstance(getattr(vc, 'pct_bases', None), dict):
                vc.pct_bases = {}

            def _get_base_sum(ci, val_map):
                """Somme des colonnes cochées comme base pour ci."""
                bases = vc.pct_bases.get(str(ci), [])
                if bases:
                    return sum(val_map.get(bci, 0.0) for bci in bases)
                # fallback : première colonne is_income
                return next((val_map.get(bci, 0.0)
                             for bci, cd in enumerate(vc.colonnes)
                             if cd.is_income and bci != ci), 0.0)

            # taux_courant : pour chaque colonne valeur_type='pct', le taux évolue palier par palier
            # initialisé depuis cd.valeur
            taux_courant = {ci: getattr(cd, 'valeur', 0.0)
                            for ci, cd in enumerate(vc.colonnes)
                            if getattr(cd, 'valeur_type', 'fixed') == 'pct'}

            # ── Premier palier si vide ──
            if not vc.paliers:
                val_map = {}
                # passe 1 : colonnes montant fixe
                for bci, cd in enumerate(vc.colonnes):
                    if getattr(cd, 'valeur_type', 'fixed') == 'fixed':
                        val_map[bci] = cd.valeur
                # passe 2 : colonnes taux (valeur = % des bases)
                for bci, cd in enumerate(vc.colonnes):
                    if getattr(cd, 'valeur_type', 'fixed') == 'pct':
                        base = _get_base_sum(bci, val_map)
                        val_map[bci] = round(base * taux_courant[bci] / 100, 2)
                vc.paliers.append(ViabilitePalier(
                    valeurs=[val_map.get(i, 0.0) for i in range(len(vc.colonnes))]))

            # ── Paliers suivants ──
            last = vc.paliers[-1]
            for _ in range(vc.n_paliers):
                prev_map = {i: (last.valeurs[i] if i < len(last.valeurs) else 0.0)
                            for i in range(len(vc.colonnes))}
                new_map = {}

                # passe 1 : colonnes montant fixe — delta_type='fixed' → ajoute delta_val
                for bci, cd in enumerate(vc.colonnes):
                    if getattr(cd, 'valeur_type', 'fixed') == 'fixed':
                        new_map[bci] = _next_val(prev_map[bci], 'fixed', cd.delta_val)

                # passe 2 : colonnes taux (valeur_type='pct')
                # le taux évolue selon delta_type :
                #   delta_type='fixed' → taux fixe (inchangé), montant = base_new * taux / 100
                #   delta_type='pct'   → taux += delta_val, montant = base_new * taux / 100
                for bci, cd in enumerate(vc.colonnes):
                    if getattr(cd, 'valeur_type', 'fixed') == 'pct':
                        if cd.delta_type == 'pct':
                            taux_courant[bci] = round(taux_courant[bci] + cd.delta_val, 10)
                        # base = nouvelles valeurs fixed déjà calculées
                        base = _get_base_sum(bci, new_map)
                        new_map[bci] = round(base * taux_courant[bci] / 100, 2)

                new_p = ViabilitePalier(
                    valeurs=[new_map.get(i, 0.0) for i in range(len(vc.colonnes))])
                vc.paliers.append(new_p)
                last = new_p
            on_save(); rebuild()

        def clear_all(e):
            vc.paliers.clear(); on_save(); rebuild()

        n_field = ft.TextField(
            value=str(vc.n_paliers), width=60, height=34, text_size=12,
            text_align=ft.TextAlign.CENTER,
            bgcolor='transparent', border_color=c('card_border'),
            focused_border_color=c('gold'),
            content_padding=P.symmetric(horizontal=6, vertical=2),
            keyboard_type=ft.KeyboardType.NUMBER,
            on_blur=upd_npal,
        )
        gen_card = ft.Container(
            ft.Row([
                _t(T.get('via_generate', 'Générer'), size=12,
                   weight=ft.FontWeight.W_600, col=c('text2'), expand=True),
                n_field,
                _t(T.get('via_paliers', 'paliers'), size=11, col=c('text3')),
                ft.ElevatedButton(
                    T.get('via_gen_btn', '▶ Go'), height=34,
                    on_click=generate,
                    style=ft.ButtonStyle(
                        bgcolor=c('teal'), color='#1a1a1a',
                        shape=ft.RoundedRectangleBorder(radius=8),
                        padding=P.symmetric(horizontal=14),
                    ),
                ),
                ft.ElevatedButton(
                    T.get('via_clear_all', 'Tout supprimer'), height=34,
                    on_click=clear_all,
                    style=ft.ButtonStyle(
                        bgcolor='transparent', color=c('danger'),
                        side=ft.BorderSide(1, c('danger')),
                        shape=ft.RoundedRectangleBorder(radius=8),
                        padding=P.symmetric(horizontal=10),
                    ),
                ),
            ], spacing=8, vertical_alignment=ft.CrossAxisAlignment.CENTER),
            padding=14, bgcolor=c('card'),
            border=B.all(1, c('card_border')), border_radius=10,
        )

        # ── Tableau des paliers ──
        def hdr(s, ck='text3'):
            return ft.Container(
                _t(s, size=9, weight=ft.FontWeight.W_600, col=c(ck),
                   align=ft.TextAlign.RIGHT),
                expand=True,
            )

        header_cells = [hdr(cd.name, 'teal' if cd.is_income else 'text3')
                        for cd in vc.colonnes]
        header_cells += [hdr(T.get('via_balance', 'Solde'), 'green'),
                         ft.Container(width=28)]
        rows = [ft.Row(header_cells, spacing=4),
                ft.Divider(height=1, color=c('card_border'))]

        for pi, palier in enumerate(vc.paliers):
            solde = _compute_solde(vc, palier)
            sc    = 'green' if solde >= 0 else 'danger'

            def upd_val(e, pi=pi, ci_=None):
                # ci_ capturé dans la closure ci-dessous
                try:
                    vc.paliers[pi].valeurs[ci_] = float(e.control.value.replace(',','.') or '0')
                    on_save(); rebuild()
                except ValueError: pass

            def del_p(e, pi=pi):
                vc.paliers.pop(pi); on_save(); rebuild()

            cells = []
            for ci, col_def in enumerate(vc.colonnes):
                v = palier.valeurs[ci] if ci < len(palier.valeurs) else 0.0
                def _upd(e, pi=pi, ci=ci):
                    try:
                        vc.paliers[pi].valeurs[ci] = float(e.control.value.replace(',','.') or '0')
                        on_save(); rebuild()
                    except ValueError: pass
                ck = 'teal' if col_def.is_income else 'text2'
                cells.append(ft.Container(
                    ft.TextField(
                        value=fmt(v), text_size=10, text_align=ft.TextAlign.RIGHT,
                        color=c(ck), bgcolor='transparent', border_color='transparent',
                        focused_border_color=c('gold'),
                        content_padding=P.symmetric(horizontal=4, vertical=4),
                        keyboard_type=ft.KeyboardType.NUMBER, on_blur=_upd,
                    ), expand=True,
                ))

            cells.append(ft.Container(
                _t(fmt(solde), size=11, weight=ft.FontWeight.W_700,
                   col=c(sc), align=ft.TextAlign.RIGHT),
                expand=True,
            ))
            cells.append(_del_btn(del_p, c))
            rows.append(ft.Row(cells, spacing=4))


        table_card = ft.Container(
            ft.Column(rows, spacing=2), padding=12,
            bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=10,
        ) if vc.paliers else ft.Container(
            _t(T.get('via_empty', 'Aucun palier — cliquez ▶ Go pour générer.'),
               size=11, col=c('text3')),
            padding=14,
        )

        return [
            _t(T['via_title'], size=20, weight=ft.FontWeight.W_700,
               family='Playfair Display', col=c('text')),
            config_card,
            gen_card,
            table_card,
            ft.Container(height=40),
        ]

    col.controls.extend(_build())
    return col


# ═══ CONFIG ═════════════════════════════════════════════════════════

def _lang_switch_card(storage, on_reload, c, T_ref):
    from core.i18n import toggle_lang, get_lang
    lang_lbl = ft.Text(
        'EN | FR', size=13, font_family='DM Sans',
        weight=ft.FontWeight.W_600, color=c('text'),
    )
    en_indicator = ft.Container(
        ft.Text('EN', size=11, font_family='DM Sans', weight=ft.FontWeight.W_700,
                color='#1a1a1a' if get_lang()=='en' else c('text2')),
        padding=P.symmetric(horizontal=10, vertical=5),
        bgcolor=c('gold') if get_lang()=='en' else 'transparent',
        border=B.all(1, c('gold') if get_lang()=='en' else c('card_border')),
        border_radius=ft.BorderRadius(6,0,0,6),
    )
    fr_indicator = ft.Container(
        ft.Text('FR', size=11, font_family='DM Sans', weight=ft.FontWeight.W_700,
                color='#1a1a1a' if get_lang()=='fr' else c('text2')),
        padding=P.symmetric(horizontal=10, vertical=5),
        bgcolor=c('gold') if get_lang()=='fr' else 'transparent',
        border=B.all(1, c('gold') if get_lang()=='fr' else c('card_border')),
        border_radius=ft.BorderRadius(0,6,6,0),
    )

    def switch(e):
        toggle_lang()
        storage.set_lang(get_lang())
        on_reload()

    return ft.Container(
        ft.Column([
            _t(T_ref['cfg_lang'], size=11, weight=ft.FontWeight.W_600, col=c('text3')),
            ft.Container(height=6),
            ft.Container(
                ft.Row([en_indicator, fr_indicator], spacing=0),
                on_click=switch, ink=True,
            ),
        ], spacing=2),
        padding=14, bgcolor=c('card'),
        border=B.all(1, c('card_border')), border_radius=12,
    )


def build_config_view(storage, t, on_save, on_toast, on_reload, on_theme_toggle, page=None, on_badge=None):
    def c(k): return t.c(k)
    _font_scale[0] = t.scale
    cfg = storage.cfg

    def _badge():
        if on_badge: on_badge()

    def field(label, value, hint='', password=False):
        lbl = _t(label, size=11, col=c('text2'))
        tf  = ft.TextField(
            value=value, hint_text=hint, password=password,
            can_reveal_password=password,
            bgcolor=c('card'), border_color=c('card_border'),
            focused_border_color=c('gold'), color=c('text'), text_size=13,
            content_padding=P.symmetric(horizontal=10, vertical=8),
        )
        return lbl, tf

    status_txt   = ft.Text('', size=12, font_family='DM Sans')
    import_status = ft.Text('', size=11, font_family='DM Sans')

    # Import path field — shown after file picker or typed manually
    import_path_tf = ft.TextField(
        value='', hint_text='Chemin vers oxycash.json (ex: C:\\Users\\...\\oxycash.json)',
        bgcolor=c('card'), border_color=c('card_border'),
        focused_border_color=c('gold'), color=c('text'), text_size=12,
        content_padding=P.symmetric(horizontal=10, vertical=8),
    )

    def abtn(label, bg, on_click, danger=False, expand=False):
        if danger:
            style = ft.ButtonStyle(bgcolor='transparent', color=c('danger'),
                                   side=ft.BorderSide(1, c('danger')),
                                   shape=ft.RoundedRectangleBorder(radius=8))
        else:
            fg = '#1a1a1a' if bg in ('gold','teal','green') else c('text')
            style = ft.ButtonStyle(bgcolor=c(bg), color=fg,
                                   shape=ft.RoundedRectangleBorder(radius=8),
                                   padding=P.symmetric(horizontal=14))
        btn = ft.ElevatedButton(label, on_click=on_click, height=36, style=style)
        if expand: btn.expand = True
        return btn

    def do_export(e):
        if page is None:
            on_toast('Export non disponible'); page.update(); return

        async def _do_export():
            import datetime
            fname = f"oxycash-{datetime.date.today()}.json"
            json_data = storage.export_json()
            try:
                # Flet 0.82: save_file returns path (desktop) or saves directly (mobile)
                result = await ft.FilePicker().save_file(
                    file_name=fname,
                    allowed_extensions=['json'],
                    src_bytes=json_data.encode('utf-8'),
                )
                if result:
                    on_toast(f'Exporté: {fname} ✓')
                else:
                    on_toast(f'Exporté: {fname} ✓')
            except Exception:
                # Fallback: write to home directory (desktop)
                try:
                    import pathlib
                    path = pathlib.Path.home() / fname
                    path.write_text(json_data, encoding='utf-8')
                    on_toast(T.fmt('cfg_exported', name=path.name))
                except Exception as ex2:
                    on_toast(f'Erreur export: {str(ex2)[:40]}')
            page.update()

        page.run_task(_do_export)

    # ── File picker (Flet 0.82+ Service API) ────────────────────────────────
    _picked_content = [None]

    def do_browse(e):
        """Pick a JSON file using Flet 0.82 FilePicker Service API."""
        if page is None:
            on_toast(T['cfg_import_na']); page.update(); return

        async def _do_pick():
            import asyncio
            try:
                files = await ft.FilePicker().pick_files(
                    allowed_extensions=['json'],
                    allow_multiple=False,
                    with_data=True,
                )
                if files and len(files) > 0:
                    picked = files[0]
                    import_path_tf.value = picked.name or 'oxycash.json'
                    # with_data=True gives us file content directly
                    if picked.data:
                        _picked_content[0] = picked.data.decode('utf-8') if isinstance(picked.data, bytes) else picked.data
                        on_toast('Fichier chargé ✓')
                    elif picked.path:
                        # Fallback: read from path (desktop)
                        import pathlib
                        try:
                            _picked_content[0] = pathlib.Path(picked.path).read_text(encoding='utf-8')
                            on_toast('Fichier chargé ✓')
                        except Exception:
                            _picked_content[0] = None
                            on_toast('Fichier sélectionné — cliquer Importer')
                    else:
                        _picked_content[0] = None
                        on_toast('Fichier sélectionné — cliquer Importer')
                else:
                    on_toast('Aucun fichier sélectionné')
            except Exception as ex:
                on_toast(f'Erreur: {str(ex)[:50]}')
            page.update()

        page.run_task(_do_pick)

    def do_import(e):
        # First try content already loaded by picker
        raw = _picked_content[0]
        if raw is None:
            path = import_path_tf.value.strip()
            if not path:
                on_toast(T['cfg_import_nopath']); page.update()
                return
            import pathlib
            try:
                raw = pathlib.Path(path).read_text(encoding='utf-8')
            except FileNotFoundError:
                on_toast(T['cfg_import_notfound']); page.update()
                return
            except Exception as ex:
                on_toast(f'Erreur: {str(ex)[:40]}'); page.update()
                return
        try:
            ok = storage.import_json(raw)
            if ok:
                import_status.value = 'Import réussi ✓'
                import_status.color = c('green')
                _picked_content[0] = None
                on_reload()
                on_toast('Import réussi !')
            else:
                import_status.value = T['cfg_import_invalid']
                import_status.color = c('danger')
                on_toast(T['cfg_import_invalid'])
        except Exception as ex:
            import_status.value = str(ex)[:60]
            import_status.color = c('danger')
            on_toast('Erreur import')
        page.update()

    def do_reset(e):
        storage.reset(); on_reload(); on_toast(T['cfg_reset_done']); page.update()

    def mk_card(title, *children):
        return ft.Container(
            ft.Column([_t(title, size=11, weight=ft.FontWeight.W_600, col=c('text3')),
                       ft.Container(height=6), *children], spacing=2),
            padding=14, bgcolor=c('card'), border=B.all(1, c('card_border')), border_radius=12,
        )

    # ── profile management ───────────────────────────────────────────────────
    def profile_card():
        profiles = storage.profiles
        active   = storage.active_slug

        rows = []
        for p in profiles:
            is_act = p['slug'] == active

            def switch(e, slug=p['slug']):
                storage.switch_profile(slug)
                on_reload()

            def del_p(e, slug=p['slug'], name=p['name']):
                if len(storage.profiles) <= 1:
                    on_toast('Cannot delete last profile')
                    return
                storage.delete_profile(slug)
                on_reload(); page.update()

            name_ref = ft.Ref[ft.TextField]()

            def rename(e, slug=p['slug']):
                tf = name_ref.current
                if tf and tf.value.strip():
                    storage.rename_profile(slug, tf.value.strip())
                    on_reload()

            rows.append(ft.Container(
                ft.Row([
                    ft.Container(
                        width=8, height=8,
                        bgcolor=c('teal') if is_act else 'transparent',
                        border_radius=4,
                        border=B.all(1, c('teal')),
                    ),
                    ft.TextField(
                        ref=name_ref,
                        value=p['name'], expand=True, height=32, text_size=12,
                        bgcolor='transparent', border_color='transparent',
                        focused_border_color=c('gold'), color=c('text'),
                        content_padding=P.symmetric(horizontal=4, vertical=2),
                        on_blur=rename,
                    ),
                    ft.Container(
                        _t('✓' if is_act else T['cfg_switch'], size=10,
                           col=c('teal') if is_act else c('text2')),
                        padding=P.symmetric(horizontal=6, vertical=4),
                        border=B.all(1, c('teal') if is_act else c('card_border')),
                        border_radius=6,
                        on_click=switch, ink=True,
                    ),
                    ft.IconButton(
                        ft.Icons.DELETE_OUTLINE, icon_size=14,
                        icon_color=c('danger'),
                        on_click=del_p,
                        style=ft.ButtonStyle(padding=P.all(2)),
                    ),
                ], spacing=6, vertical_alignment=ft.CrossAxisAlignment.CENTER),
                padding=P.symmetric(horizontal=4, vertical=4),
                bgcolor=c('card') if is_act else 'transparent',
                border_radius=8,
            ))

        new_name_tf = ft.TextField(
            value='', hint_text=T['cfg_profile_hint'],
            bgcolor=c('card'), border_color=c('card_border'),
            focused_border_color=c('gold'), color=c('text'), text_size=12,
            content_padding=P.symmetric(horizontal=10, vertical=6),
            height=36,
        )

        def add_profile(e):
            name = new_name_tf.value.strip()
            if not name:
                on_toast(T['toast_label_req']); return
            slug = storage.add_profile(name)
            storage.switch_profile(slug)
            new_name_tf.value = ''
            on_toast(f'Profil « {name} » créé ✓')
            on_reload(); page.update()

        return mk_card(T['cfg_profiles'],
                       *rows,
                       ft.Container(height=4),
                       ft.Column([
                           new_name_tf,
                           ft.ElevatedButton(T['cfg_add_profile'], on_click=add_profile,
                                             height=36,
                                             style=ft.ButtonStyle(
                                                 bgcolor=c('teal'), color='#1a1a1a',
                                                 shape=ft.RoundedRectangleBorder(radius=8),
                                                 padding=P.symmetric(horizontal=12))),
                       ], spacing=6))

    # ── WebDAV per active profile ─────────────────────────────────────────────
    prof    = storage.active_profile
    url_lbl, url_tf = field(T['cfg_url'],      prof.get('dav_url',''),  'https://…/remote.php/dav/files/user/Oxy/')
    usr_lbl, usr_tf = field(T['cfg_user'],     prof.get('dav_user',''), 'user@email.com')
    pw_lbl,  pw_tf  = field(T['cfg_password'], prof.get('dav_pass',''), '••••••••', password=True)

    def save_cfg_profile(e):
        storage.save_profile_dav(storage.active_slug,
                                  url_tf.value.strip(),
                                  usr_tf.value.strip(),
                                  pw_tf.value)
        # Auto-test connection so badge reflects real state
        ok, msg = storage.test_dav()
        if ok:
            storage.dav_ok = True
            status_txt.value = T['cfg_saved'] + ' — ' + msg; status_txt.color = c('green')
        else:
            status_txt.value = T['cfg_saved']; status_txt.color = c('green')
        on_toast(T['cfg_saved']); _badge(); page.update()

    def test_cfg_profile(e):
        storage.save_profile_dav(storage.active_slug,
                                  url_tf.value.strip(),
                                  usr_tf.value.strip(),
                                  pw_tf.value)
        ok, msg = storage.test_dav()
        status_txt.value = msg
        status_txt.color = c('green') if ok else c('danger')
        on_toast(f'\u2713 {msg}' if ok else f'\u2717 {msg}'); _badge(); page.update()

    def clear_cfg(e):
        storage.clear_config()
        url_tf.value = ''; usr_tf.value = ''; pw_tf.value = ''
        status_txt.value = T['cfg_cleared']; status_txt.color = c('text2')
        on_toast(T['cfg_cleared']); _badge(); page.update()

    # ── Currency ──────────────────────────────────────────────────────────────
    currency_tf = ft.TextField(
        value=storage.currency, width=80, height=36, text_size=13,
        bgcolor=c('card'), border_color=c('card_border'),
        focused_border_color=c('gold'), color=c('text'),
        content_padding=P.symmetric(horizontal=10, vertical=6),
    )
    def save_currency(e):
        storage.set_currency(currency_tf.value.strip() or 'CHF')
        on_toast(T['cfg_saved'])

    currency_card = mk_card(T['cfg_currency'],
        ft.Row([currency_tf,
                abtn(T['cfg_save'], 'gold', save_currency)], spacing=8))

    # ── Theme toggle card ─────────────────────────────────────────────────
    theme_card = mk_card(T['cfg_theme'],
        ft.Container(
            ft.Row([
                _t('🌙' if t.is_dark else '☀️', size=18),
                _t(T['cfg_dark'] if t.is_dark else T['cfg_light'],
                   size=13, weight=ft.FontWeight.W_600, col=c('text')),
            ], spacing=8),
            on_click=lambda e: on_theme_toggle(),
            ink=True, padding=P.symmetric(horizontal=8, vertical=8),
            border=B.all(1, c('card_border')), border_radius=8,
        ))

    # ── Lang switch card ──────────────────────────────────────────────────
    lang_card = _lang_switch_card(storage, on_reload, c, T)

    return ft.Column([
        _t(T['cfg_title'], size=20, weight=ft.FontWeight.W_700,
           family='Playfair Display', col=c('text')),
        profile_card(),
        currency_card,
        theme_card,
        lang_card,
        mk_card(T['cfg_webdav'],
                url_lbl, url_tf, ft.Container(height=2),
                usr_lbl, usr_tf, ft.Container(height=2),
                pw_lbl,  pw_tf,  ft.Container(height=6),
                ft.Row([abtn(T['cfg_save'],'gold',save_cfg_profile),
                        abtn(T['cfg_test'],'teal',test_cfg_profile),
                        abtn(T['cfg_clear'],'card',clear_cfg)],
                       spacing=8, wrap=True),
                status_txt),
        mk_card(T['cfg_export'],
                abtn(T['cfg_export_lbl'], 'card', do_export),
                _t(T['cfg_export_sub'], size=10, col=c('text3'))),
        mk_card(T['cfg_import'],
                _t(T['cfg_import_sub'], size=10, col=c('text3')),
                ft.Container(height=4),
                import_path_tf,
                ft.Container(height=6),
                ft.Row([abtn(T['cfg_browse'], 'card', do_browse),
                        abtn(T['cfg_import_btn'], 'teal', do_import)],
                       spacing=8, wrap=True),
                import_status),
        mk_card(T['cfg_data'], abtn(T['cfg_reset'],'',do_reset,danger=True)),
        ft.Container(height=40),
    ], spacing=12, scroll=ft.ScrollMode.AUTO, expand=True)