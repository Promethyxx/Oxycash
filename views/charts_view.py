"""Oxycash – Annual overview (Flet 0.82+)
Same 6 summary cards as monthly view but with annual totals + monthly breakdown table."""
from __future__ import annotations
import flet as ft
from core.model import AppData, MONTHS, fmt, fmt_sign
from core.i18n import T

P  = ft.Padding
M  = ft.Margin
B  = ft.Border
BS = ft.BorderSide


def build_charts_view(data: AppData, t, currency: str) -> ft.Column:
    def c(k): return t.c(k)
    sc = t.scale

    def txt(s, size=12, weight=ft.FontWeight.NORMAL, col='text',
            family='DM Sans', align=ft.TextAlign.LEFT, expand=False):
        return ft.Text(str(s), size=max(6, size + sc), weight=weight,
                       font_family=family, color=c(col),
                       text_align=align, expand=expand if expand else None)

    # ── Compute annual totals — same logic as monthly summary() ───────────
    tot_rev = 0.0
    tot_paye = 0.0
    tot_a_payer = 0.0
    tot_ret = 0.0
    tot_prev = 0.0
    tot_solde = 0.0

    month_rows = []

    for mk in MONTHS:
        m = data.months.get(mk)
        if m is None:
            month_rows.append({'rev': 0, 'dep': 0, 'prev': 0})
            continue

        rev_banque = sum(l.banque for l in m.revenus)
        rev_cash   = sum(l.cash   for l in m.revenus)
        rev_total  = rev_banque + rev_cash

        ret_a_retirer = sum(l.banque for l in m.retraits)
        ret_retire    = sum(l.etat() for l in m.retraits)

        all_dep    = m.fixes + m.variables
        dep_banque = sum(l.banque for l in all_dep)
        dep_cash   = sum(l.cash   for l in all_dep)

        rev_recu_b = sum(l.etat() for l in m.revenus if l.banque > 0.01)
        rev_recu_c = sum(l.etat() for l in m.revenus if l.cash   > 0.01)
        paye_banque = sum(l.etat() for l in all_dep if l.banque > 0.01) + ret_retire
        paye_cash   = sum(l.etat() for l in all_dep if l.cash > 0.01)
        paye_total  = paye_banque + paye_cash

        a_payer_b = dep_banque - sum(l.etat() for l in all_dep if l.banque > 0.01)
        a_payer_c = dep_cash - sum(l.etat() for l in all_dep if l.cash > 0.01)

        prev_banque = rev_banque - dep_banque - ret_a_retirer
        prev_cash   = rev_cash + ret_a_retirer - dep_cash
        prev_total  = prev_banque + prev_cash

        solde_banque = rev_recu_b - paye_banque
        solde_cash   = ret_retire + rev_recu_c - paye_cash
        solde_total  = solde_banque + solde_cash

        tot_rev     += rev_total
        tot_ret     += ret_a_retirer
        tot_paye    += paye_total
        tot_a_payer += a_payer_b + a_payer_c
        tot_prev    += prev_total
        tot_solde   += solde_total

        month_rows.append({
            'rev': rev_total,
            'dep': dep_banque + dep_cash,
            'prev': prev_total,
        })

    def cc(n): return 'green' if n > 0.01 else ('danger' if n < -0.01 else 'text')

    # ── 6 summary cards ───────────────────────────────────────────────────
    def card(label, value, col_key) -> ft.Container:
        return ft.Container(
            ft.Column([
                txt(label, size=9, weight=ft.FontWeight.W_600, col='text3'),
                txt(f"{fmt_sign(value)} {currency}", size=16,
                    weight=ft.FontWeight.W_700, col=col_key,
                    family='Playfair Display'),
            ], spacing=4),
            padding=14, bgcolor=c('card'),
            border=B.all(1, c('card_border')), border_radius=10,
            expand=True,
        )

    cards_row1 = ft.Row([
        card(T['card_income'], tot_rev, cc(tot_rev)),
        card(T['card_paid'], tot_paye, 'teal'),
    ], spacing=8)

    cards_row2 = ft.Row([
        card(T['card_to_pay'], tot_a_payer, cc(-tot_a_payer)),
        card(T['card_withdrawals'], tot_ret, 'amber'),
    ], spacing=8)

    cards_row3 = ft.Row([
        card(T['card_forecast'], tot_prev, cc(tot_prev)),
        card(T['card_balance'], tot_solde, cc(tot_solde)),
    ], spacing=8)

    # ── Monthly breakdown table ───────────────────────────────────────────
    month_labels = T.months_short

    def hdr(s, width=None):
        ct = txt(s, size=9, weight=ft.FontWeight.W_600, col='text3',
                 align=ft.TextAlign.CENTER)
        return ft.Container(ct, width=width) if width else ft.Container(ct, expand=True)

    header = ft.Row([
        hdr('', width=36),
        hdr(T['card_income']),
        hdr(T.get('chart_exp', 'Dep.')),
        hdr(T['card_forecast']),
    ], spacing=4)

    tbl_rows = [header, ft.Divider(height=1, color=c('card_border'))]
    cumul = 0.0
    for i, md in enumerate(month_rows):
        cumul += md['prev']
        pc = cc(md['prev'])
        tbl_rows.append(ft.Row([
            ft.Container(txt(month_labels[i], size=10, col='text3',
                             align=ft.TextAlign.CENTER), width=36),
            ft.Container(txt(fmt(md['rev']), size=10, col='teal',
                             align=ft.TextAlign.RIGHT), expand=True),
            ft.Container(txt(fmt(md['dep']), size=10, col='danger',
                             align=ft.TextAlign.RIGHT), expand=True),
            ft.Container(txt(fmt_sign(md['prev']), size=10, col=pc,
                             weight=ft.FontWeight.W_700,
                             align=ft.TextAlign.RIGHT), expand=True),
        ], spacing=4))

    tbl_rows.append(ft.Divider(height=1, color=c('card_border')))
    cumul_col = cc(cumul)
    tbl_rows.append(ft.Row([
        ft.Container(txt(T.get('col_total', 'Total'), size=10,
                         weight=ft.FontWeight.W_700, col='text'), width=36),
        ft.Container(txt(fmt(tot_rev), size=10, weight=ft.FontWeight.W_700,
                         col='teal', align=ft.TextAlign.RIGHT), expand=True),
        ft.Container(txt(fmt(tot_rev - tot_prev), size=10,
                         weight=ft.FontWeight.W_700, col='danger',
                         align=ft.TextAlign.RIGHT), expand=True),
        ft.Container(txt(fmt_sign(tot_prev), size=10, weight=ft.FontWeight.W_700,
                         col=cc(tot_prev), align=ft.TextAlign.RIGHT), expand=True),
    ], spacing=4))

    # Cumul annuel
    cumul_card = ft.Container(
        ft.Row([
            txt(T.get('chart_annual_cumul', 'Annual cumulative'), size=13,
                weight=ft.FontWeight.W_600, col='text2', expand=True),
            txt(f"{fmt_sign(cumul)} {currency}", size=20,
                weight=ft.FontWeight.W_700, col=cumul_col,
                family='Playfair Display', align=ft.TextAlign.RIGHT),
        ]),
        padding=14, bgcolor=c('card'),
        border=B.all(1, c('card_border')), border_radius=10,
    )

    table = ft.Container(
        ft.Column(tbl_rows, spacing=4),
        padding=12, bgcolor=c('card'),
        border=B.all(1, c('card_border')), border_radius=10,
    )

    return ft.Column([
        txt(T['tab_charts'], size=20, weight=ft.FontWeight.W_700,
            family='Playfair Display', col='text'),
        cards_row1,
        cards_row2,
        cards_row3,
        cumul_card,
        ft.Container(height=8),
        table,
        ft.Container(height=40),
    ], spacing=12, scroll=ft.ScrollMode.AUTO, expand=True)