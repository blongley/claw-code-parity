"""Build the CS Analytical opportunity-brief deck as a vector PDF + slide PNGs.

Mirrors build_deck_pptx.py one-for-one. Caslon serif system; falls back to
a registered serif on hosts without Caslon installed.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

from reportlab.lib.colors import HexColor
from reportlab.lib.units import inch
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas

ROOT = Path(__file__).resolve().parent.parent
OUT_PDF = ROOT / "02_Deck_PDF" / "CS_Analytical_Opportunity_Brief.pdf"
SLIDES_DIR = ROOT / "03_Deck_Slide_Images"

PAGE_W = 13.333 * inch
PAGE_H = 7.5 * inch

IVORY = HexColor("#FAF7F2")
NAVY = HexColor("#0B2545")
BURGUNDY = HexColor("#6E1B26")
CHARCOAL = HexColor("#1C1C1C")
SLATE = HexColor("#555B66")
GOLD = HexColor("#B58B3A")
RULE = HexColor("#C9BEA8")


# ---------------------------------------------------------------------------
# Fonts — Caslon if available, otherwise registered serif fallback
# ---------------------------------------------------------------------------

def _try_register(name: str, path: str) -> bool:
    p = Path(path)
    if not p.exists():
        return False
    try:
        pdfmetrics.registerFont(TTFont(name, str(p)))
        return True
    except Exception:
        return False


def _setup_fonts():
    candidates = [
        "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    ]
    candidates_b = [
        "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
    ]
    candidates_i = [
        "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Italic.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf",
    ]
    head = body = "Times-Roman"
    head_b = body_b = "Times-Bold"
    head_i = body_i = "Times-Italic"
    for c in candidates:
        if _try_register("CaslonLike", c):
            head = body = "CaslonLike"
            break
    for c in candidates_b:
        if _try_register("CaslonLike-Bold", c):
            head_b = body_b = "CaslonLike-Bold"
            break
    for c in candidates_i:
        if _try_register("CaslonLike-Italic", c):
            head_i = body_i = "CaslonLike-Italic"
            break
    return {"regular": head, "bold": head_b, "italic": head_i}


FONTS = _setup_fonts()


def _fill(c, rgb):
    c.setFillColor(rgb)


def _stroke(c, rgb):
    c.setStrokeColor(rgb)


def rect(c, x, y, w, h, color):
    _fill(c, color)
    c.rect(x, y, w, h, stroke=0, fill=1)


def hairline(c, x, y, w, color=RULE, weight=0.6):
    _stroke(c, color)
    c.setLineWidth(weight)
    c.line(x, y, x + w, y)


def y_from_top(top_inches):
    return PAGE_H - top_inches * inch


def text(c, x_in, top_in, body, *, size, color=CHARCOAL, font=None,
         italic=False, bold=False, align="left", line_spacing=1.15,
         max_width_in=None):
    if font is None:
        font = FONTS["italic"] if italic else (FONTS["bold"] if bold else FONTS["regular"])
    _fill(c, color)
    c.setFont(font, size)
    line_height = size * line_spacing
    x = x_in * inch
    y = y_from_top(top_in) - size

    lines = body.split("\n") if isinstance(body, str) else body
    for line in lines:
        if align == "right" and max_width_in is not None:
            tw = c.stringWidth(line, font, size)
            xx = (x_in + max_width_in) * inch - tw
        elif align == "center" and max_width_in is not None:
            tw = c.stringWidth(line, font, size)
            xx = (x_in + max_width_in / 2) * inch - tw / 2
        else:
            xx = x
        c.drawString(xx, y, line)
        y -= line_height


def page_chrome(c, *, page_no, total, section):
    rect(c, 0, 0, PAGE_W, PAGE_H, IVORY)
    hairline(c, 0.75 * inch, y_from_top(0.55), 11.83 * inch,
             color=RULE, weight=0.6)
    text(c, 0.75, 0.30, section.upper(), size=8.5, color=SLATE,
         font=FONTS["italic"], italic=True)
    text(c, 8.0, 0.30, "TSI-CITADEL  ·  OPPORTUNITY BRIEF",
         size=8.5, color=SLATE, italic=True, align="right",
         max_width_in=4.83)
    hairline(c, 0.75 * inch, y_from_top(7.05), 11.83 * inch,
             color=RULE, weight=0.45)
    text(c, 0.75, 7.12,
         "Prepared for Alan Weiss  ·  CFO, CS Analytical Laboratory  ·  Confidential",
         size=8, color=SLATE, italic=True)
    text(c, 8.0, 7.12, f"{page_no:02d} / {total:02d}",
         size=8, color=SLATE, align="right", max_width_in=4.83)


# ---------------------------------------------------------------------------
# Slides
# ---------------------------------------------------------------------------

def slide_cover(c):
    rect(c, 0, 0, PAGE_W, PAGE_H, IVORY)
    rect(c, 0, 0, 0.6 * inch, PAGE_H, NAVY)
    rect(c, 0.6 * inch, 0, 0.08 * inch, PAGE_H, BURGUNDY)

    text(c, 1.1, 0.85, "CS ANALYTICAL LABORATORY",
         size=13, color=SLATE, italic=True)
    hairline(c, 1.1 * inch, y_from_top(1.35), 3.5 * inch,
             color=BURGUNDY, weight=1.5)

    text(c, 1.1, 1.85,
         "An opportunity brief.\nMarket, growth paths,\nand the agentic-AI question.",
         size=40, color=NAVY, line_spacing=1.1)

    hairline(c, 1.1 * inch, y_from_top(5.55), 2.2 * inch,
             color=NAVY, weight=1.0)
    text(c, 1.1, 5.75, "Prepared by  Bruce Longley  ·  TSI-Citadel",
         size=18, color=CHARCOAL)
    text(c, 1.1, 6.15,
         "For  Alan Weiss  ·  Chief Financial Officer  ·  CS Analytical Laboratory",
         size=12, color=SLATE, italic=True)
    text(c, 1.1, 6.5, "May 2026  ·  bruce@tsicitadel.ai",
         size=10, color=SLATE)


def slide_why(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Why this brief")
    text(c, 0.75, 1.05, "Why we're sending this",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "CS Analytical has been on our\nwatchlist for two years.\nThe last six months made the\nwatchlist a working file.",
         size=32, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(6.0), 2.5 * inch,
             color=BURGUNDY, weight=1.25)
    text(c, 0.75, 6.2,
         "The Clifton expansion, RM Analytical, USP <382>, and\nthe GLP-1 wave all landed in the same quarter.\nThat is the signal.",
         size=15, color=CHARCOAL, italic=True, line_spacing=1.4)


def slide_company(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The company")
    text(c, 0.75, 1.05, "CS Analytical at a glance — outside view",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "The only cGMP-built,\nFDA-regulated lab in the world\ndedicated to container testing.",
         size=24, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(4.65), 2.5 * inch,
             color=BURGUNDY, weight=1.25)

    pillars = [
        ("Where",
         "Clifton, New Jersey.\nExpansion under way to\ndouble the current footprint."),
        ("What",
         "USP <1207> deterministic\nportfolio: helium leak, vacuum\ndecay, HVLD, laser-headspace.\nUSP <382>, Annex 1, microbial."),
        ("Who",
         "Founding-team pedigree on\nthe world's first cGMP CCIT\nlab. PhD-staffed, founder-led,\nclient-facing scientists."),
        ("Recent moves",
         "RM Analytical (raw materials,\nFeb 2026). Micro testing —\nUSP <87>, <788>. USP/EP gas.\nLab doubling (May 2026)."),
    ]
    x = 0.75
    col_w = 2.95
    for title, body in pillars:
        rect(c, x * inch, y_from_top(4.85), (col_w - 0.15) * inch, 2, BURGUNDY)
        text(c, x, 4.95, title, size=14, color=NAVY)
        text(c, x, 5.5, body, size=11, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_market(c, page, total, chart_path):
    page_chrome(c, page_no=page, total=total, section="The market")
    text(c, 0.75, 1.05, "What sits in front of you",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "A $9.5 B host market —\nand a $1.5 B niche moving with it.",
         size=22, color=NAVY, line_spacing=1.12)

    c.drawImage(str(chart_path), 0.75 * inch, y_from_top(6.8),
                width=7.6 * inch, height=3.4 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 8.7 * inch, y_from_top(3.6), 0.6 * inch, color=BURGUNDY)
    text(c, 8.7, 3.85,
         "Pharma analytical testing\noutsourcing: $9.5 B (2025),\n~9% CAGR through 2030.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.7, 4.85,
         "CCIT services carve-out:\n$1.5 B (2025), ~9-10% CAGR.\n$2.4 B by 2030.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.7, 5.85,
         "No CCIT-pure provider holds\nmore than mid-single-digit\nshare. The room is open.",
         size=11, color=CHARCOAL, line_spacing=1.4)

    text(c, 0.75, 6.85,
         "Sources (composite midpoints, 2026): Precedence Research, Market Research Future, Data Horizzon, Roots Analysis.",
         size=7.5, color=SLATE, italic=True)


def slide_tailwinds(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Tailwinds")
    text(c, 0.75, 1.05, "Four forces, one quarter",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Each of these alone would justify a faster\nplan. All four landed at once.",
         size=20, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(3.95), 11.83 * inch, color=RULE)

    forces = [
        ("USP <382>",
         "Effective Dec 1, 2025.\nSystem-level functional\nsuitability testing of\nelastomeric components.\nResponsibility shifts to\nthe drug manufacturer."),
        ("Annex 1 convergence",
         "EU GMP Annex 1 now\nmandates validated\ndeterministic CCIT.\nVisual inspection alone\nis no longer acceptable.\nGlobal alignment is here."),
        ("GLP-1 / PFS wave",
         "Prefilled syringes:\n$9.7 B (2025) → $18.1 B\n(2031). GLP-1 autoinjectors\nat 15.6% CAGR. Shift to\nCOP barrels. Volumes\npunish defect rates."),
        ("Biologics share",
         "Biologics now ~40% of\nFDA novel approvals.\nEvery one ships in a\nsterile primary container.\nGlass-delamination recalls\nare in the news again."),
    ]
    x = 0.75
    col_w = 2.95
    for title, body in forces:
        text(c, x, 4.15, title, size=14, color=NAVY)
        rect(c, x * inch, y_from_top(4.6), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.8, body, size=10.5, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_competitive(c, page, total, chart_path):
    page_chrome(c, page_no=page, total=total, section="The competitive field")
    text(c, 0.75, 1.05, "Who else is in the room",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Generalists are broad.\nSpecialists are few.\nThe corner is open.",
         size=22, color=NAVY, line_spacing=1.1)

    c.drawImage(str(chart_path), 0.75 * inch, y_from_top(6.8),
                width=7.8 * inch, height=3.8 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 8.8 * inch, y_from_top(3.2), 0.6 * inch, color=BURGUNDY)
    text(c, 8.8, 3.4,
         "Eurofins, SGS, Charles\nRiver, WuXi, Pace,\nIntertek, Element —\nbroad, deep pockets,\nshallow on CCIT.",
         size=10.5, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.8, 5.05,
         "Nelson, Boston Analytical,\nWest Pharma Labs —\ncredible specialists, slow\nto modernize.",
         size=10.5, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.8, 6.20,
         "The deep-and-modern\ncorner is empty.\nCS Analytical is the\ncredible occupant.",
         size=10.5, color=CHARCOAL, line_spacing=1.4)


def slide_in_motion(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Already in motion")
    text(c, 0.75, 1.05,
         "What CS Analytical is already doing right",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "This is not a turnaround brief.\nIt is an acceleration brief.",
         size=22, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.95), 11.83 * inch, color=RULE)

    moves = [
        ("RM Analytical launch",
         "Feb 2026.  Raw material\nand excipient testing\nunder a sister brand."),
        ("Clifton expansion",
         "May 2026. Doubling the\nlab footprint. Capacity\nahead of demand."),
        ("Micro & gas testing",
         "USP <87>, USP <788>,\nUSP/EP gas. Cross-sell\ninto the existing book."),
        ("USP <382> readiness",
         "Already running elastomer\nfunctionality. The Dec 2025\nwave finds capacity here."),
    ]
    x = 0.75
    col_w = 2.95
    for title, body in moves:
        text(c, x, 4.15, title, size=13.5, color=NAVY)
        rect(c, x * inch, y_from_top(4.6), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.8, body, size=11, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_organic(c, page, total):
    page_chrome(c, page_no=page, total=total,
                section="Organic growth — no AI required")
    text(c, 0.75, 1.05,
         "Five paths that don't require an AI partner",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Each is buildable on the\nexisting team and footprint.",
         size=22, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.85), 11.83 * inch, color=RULE)

    vectors = [
        ("01", "USP <382> wave capture",
         "Be the named lab for the post-Dec 2025 elastomer rush."),
        ("02", "GLP-1 / PFS HVLD productization",
         "HVLD for COP-barrel autoinjectors. Repeatable, high-margin."),
        ("03", "Annex 1 European mandate",
         "EU GMP Annex 1 opens European sponsors. Reciprocal-recognition."),
        ("04", "Cell, gene & cryo container methods",
         "No incumbent method. Highest margin per study in the market."),
        ("05", "Regulatory consulting carve-out",
         "Productize the advisory work the team gives away today."),
    ]
    y = 4.1
    for num, title, body in vectors:
        text(c, 0.75, y, num, size=22, color=BURGUNDY, italic=True)
        text(c, 1.7, y + 0.08, title, size=14, color=NAVY)
        text(c, 6.3, y + 0.1, body, size=11.5, color=CHARCOAL, italic=True,
             line_spacing=1.35)
        y += 0.55


def slide_agentic(c, page, total):
    page_chrome(c, page_no=page, total=total,
                section="Agentic AI — TSI-Citadel angle")
    text(c, 0.75, 1.05, "Where TSI-Citadel multiplies what's here",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "The healthcare agentic-AI category is\ntracking 9× growth, 2024 → 2030.",
         size=20, color=NAVY, line_spacing=1.1)
    text(c, 0.75, 3.15, "These are the four we'd build first.",
         size=12, color=SLATE, italic=True)
    hairline(c, 0.75 * inch, y_from_top(3.75), 11.83 * inch, color=RULE)

    uses = [
        ("Regulatory intel",
         "Continuous monitoring of\nUSP, EP, JP, PIC/S, FDA\n483s, warning letters.\nResolves changes against\nclient filed methods."),
        ("Market radar",
         "FDA filings, ClinicalTrials.gov,\nconference rosters, container-\nvendor releases. Daily ingest,\nweekly BD shortlist with\nevidence linked."),
        ("BD operations",
         "Outbound drafting,\nqualification, follow-up,\nmeeting prep, post-call\nsummary. Returns founder\nand scientist time."),
        ("Scientific surveillance",
         "Daily distillation of CCIT,\nHVLD, leak-detection,\nvendor white-paper\nliterature. Methods surface\nbefore the client asks."),
    ]
    x = 0.75
    col_w = 2.95
    for title, body in uses:
        text(c, x, 4.0, title, size=13.5, color=NAVY)
        rect(c, x * inch, y_from_top(4.45), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.65, body, size=11, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_roi(c, page, total, chart_path):
    page_chrome(c, page_no=page, total=total, section="ROI shape — for the CFO")
    text(c, 0.75, 1.05, "Three readings of the same 36 months",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Status quo. Organic. Organic + agentic.",
         size=20, color=NAVY, line_spacing=1.1)

    c.drawImage(str(chart_path), 0.75 * inch, y_from_top(6.4),
                width=7.6 * inch, height=3.4 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 8.6 * inch, y_from_top(3.2), 0.6 * inch, color=BURGUNDY)
    text(c, 8.6, 3.4, "PAYBACK ON THE AGENTIC LAYER",
         size=10, color=BURGUNDY, italic=True)
    text(c, 8.6, 3.85,
         "Conservative case: 9-12 months.\nCarried by founder-calendar\nrecovery and quote-cycle\nreduction alone.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.6, 5.10,
         "Aggressive case: 4-6 months.\nDriven by net-new BD\nopportunities the radar surfaces\nand the proposal agent closes.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.6, 6.35,
         "Capital required: low six figures.\nNot a transformation. A bolt-on.",
         size=11, color=CHARCOAL, line_spacing=1.4)


def slide_adjacent(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Adjacent opportunities")
    text(c, 0.75, 1.05, "Where else this team could play",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Adjacencies, not pivots.\nOptional, not required.",
         size=22, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.85), 11.83 * inch, color=RULE)

    items = [
        ("Combination products",
         "Cross-center (CDER + CDRH) submissions. Few incumbent labs\nare ready. Margin profile is favorable."),
        ("Device sterilization validation",
         "ISO 11135 / 11137 adjacency to existing container work.\nNatural cross-sell into the same sponsor base."),
        ("Compounding pharmacy testing",
         "USP <797>, <800>. Different regulatory floor; same testing\nphysics. Fragmented buyer base, less competition."),
        ("Veterinary biologics",
         "Same containers, smaller competitive set, USDA channel.\nUnderpriced relative to human-side work."),
        ("Industry training & certification",
         "The team's reputation is a productizable asset.\nCourse revenue is high margin and is itself a BD funnel."),
    ]
    y = 4.05
    for title, body in items:
        text(c, 0.75, y, title, size=13, color=NAVY)
        text(c, 4.6, y + 0.05, body, size=11.5, color=CHARCOAL,
             line_spacing=1.4)
        y += 0.55


def slide_risks(c, page, total):
    page_chrome(c, page_no=page, total=total,
                section="Risks — named, not glossed")
    text(c, 0.75, 1.05, "What we'd want to disagree about",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6, "Three CFO-relevant risks,\nthree handlings.",
         size=22, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.85), 11.83 * inch, color=RULE)

    risks = [
        ("Capital timing",
         "Lab expansion + RMA build + agentic spend in one fiscal year.\nHandling: sequence agentic spend behind the lab build;\nagentic stack pays for itself in <12 months."),
        ("Margin dilution from adjacencies",
         "Raw-material and micro work carry lower gross margin than CCIT.\nHandling: hold CCIT premium pricing while RMA scales; use\nagentic intel to keep the CCIT pipeline full."),
        ("Agentic-AI hallucination in regulated context",
         "An agent that flags a false USP change damages a sponsor\nrelationship. Handling: human-in-the-loop on every flag that\ntouches a client filed method. Suggested, not authoritative."),
    ]
    y = 4.05
    for title, body in risks:
        text(c, 0.75, y, title, size=13.5, color=NAVY)
        text(c, 4.8, y + 0.05, body, size=11, color=CHARCOAL,
             line_spacing=1.4)
        y += 0.95


def slide_working_together(c, page, total):
    page_chrome(c, page_no=page, total=total,
                section="If you wanted to work with us")
    text(c, 0.75, 1.05, "How TSI-Citadel would engage",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Light footprint. Boxed scope.\nMeasurable from day one.",
         size=22, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.95), 11.83 * inch, color=RULE)

    steps = [
        ("DAYS 1 — 30", "Diagnostic",
         "Sit with the BD, regulatory,\nand operations leads. Map\nthe data feeds, the cycles,\nand the moments. No code\nyet. Written read-out and a\ngo / no-go at day 30."),
        ("DAYS 31 — 60", "Two agents live",
         "Regulatory-intel and market-\nradar agents, wired to real\nfeeds. Daily output to a\nnamed internal user. First\nmeasurable lift inside the\nperiod."),
        ("DAYS 61 — 90", "Decide",
         "Joint readout. Numbers.\nThe three remaining agents\nare built only if the first\ntwo earned them. The CFO\nholds the gate, by design."),
    ]
    x = 0.75
    col_w = 3.95
    for label, title, body in steps:
        text(c, x, 4.15, label, size=10, color=BURGUNDY, italic=True)
        text(c, x, 4.55, title, size=15, color=NAVY)
        rect(c, x * inch, y_from_top(5.05), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 5.25, body, size=11, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_closing(c):
    rect(c, 0, 0, PAGE_W, PAGE_H, NAVY)
    rect(c, 0, 0, 0.08 * inch, PAGE_H, BURGUNDY)

    text(c, 1.1, 1.45, "THE NEXT CONVERSATION",
         size=12, color=GOLD, italic=True)
    hairline(c, 1.1 * inch, y_from_top(2.0), 2.5 * inch,
             color=BURGUNDY, weight=1.5)

    text(c, 1.1, 2.35,
         "Read the long form\non a plane.\nThen tell us where\nwe're wrong.",
         size=36, color=IVORY, line_spacing=1.1)

    text(c, 1.1, 5.6, "Bruce Longley", size=18, color=IVORY)
    text(c, 1.1, 5.95, "TSI-Citadel", size=12, color=GOLD, italic=True)
    text(c, 1.1, 6.3, "bruce@tsicitadel.ai", size=12, color=IVORY)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    OUT_PDF.parent.mkdir(parents=True, exist_ok=True)
    SLIDES_DIR.mkdir(parents=True, exist_ok=True)

    build_dir = ROOT / "_build"
    market_chart = build_dir / "chart_market.png"
    comp_chart = build_dir / "chart_competitive.png"
    roi_chart = build_dir / "chart_roi.png"

    c = canvas.Canvas(str(OUT_PDF), pagesize=(PAGE_W, PAGE_H))
    c.setTitle("CS Analytical Opportunity Brief — for Alan Weiss")
    c.setAuthor("Bruce Longley, TSI-Citadel")
    c.setSubject("Market, organic growth, and agentic-AI opportunity")

    total = 13

    slide_cover(c);                       c.showPage()
    slide_why(c, 2, total);               c.showPage()
    slide_company(c, 3, total);           c.showPage()
    slide_market(c, 4, total, market_chart); c.showPage()
    slide_tailwinds(c, 5, total);         c.showPage()
    slide_competitive(c, 6, total, comp_chart); c.showPage()
    slide_in_motion(c, 7, total);         c.showPage()
    slide_organic(c, 8, total);           c.showPage()
    slide_agentic(c, 9, total);           c.showPage()
    slide_roi(c, 10, total, roi_chart);   c.showPage()
    slide_adjacent(c, 11, total);         c.showPage()
    slide_risks(c, 12, total);            c.showPage()
    slide_working_together(c, 13, total); c.showPage()
    slide_closing(c);                     c.showPage()

    c.save()
    print(f"wrote {OUT_PDF}")

    out_prefix = SLIDES_DIR / "slide"
    subprocess.run(
        ["pdftoppm", "-png", "-r", "180", str(OUT_PDF), str(out_prefix)],
        check=True,
    )
    print(f"wrote PNGs to {SLIDES_DIR}")


if __name__ == "__main__":
    main()
