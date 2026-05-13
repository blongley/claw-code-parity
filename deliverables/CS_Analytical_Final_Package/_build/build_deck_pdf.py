"""Final-package deck — vector PDF + slide PNGs.

Mirrors build_deck_pptx.py one-for-one. Caslon serif system; falls back
to a registered serif when Caslon is not installed on the host.
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
OUT_PDF = ROOT / "01_Deck" / "CS_Analytical_Opportunity_Brief.pdf"
SLIDES_DIR = ROOT / "01_Deck" / "Slide_Images"

PAGE_W = 13.333 * inch
PAGE_H = 7.5 * inch
TOTAL = 17

IVORY = HexColor("#FAF7F2")
NAVY = HexColor("#0B2545")
BURGUNDY = HexColor("#6E1B26")
CHARCOAL = HexColor("#1C1C1C")
SLATE = HexColor("#555B66")
GOLD = HexColor("#B58B3A")
RULE = HexColor("#C9BEA8")


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
    head = body = "Times-Roman"
    head_b = body_b = "Times-Bold"
    head_i = body_i = "Times-Italic"
    for path in [
        "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    ]:
        if _try_register("CaslonLike", path):
            head = body = "CaslonLike"
            break
    for path in [
        "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
    ]:
        if _try_register("CaslonLike-Bold", path):
            head_b = body_b = "CaslonLike-Bold"
            break
    for path in [
        "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Italic.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf",
    ]:
        if _try_register("CaslonLike-Italic", path):
            head_i = body_i = "CaslonLike-Italic"
            break
    return {"regular": head, "bold": head_b, "italic": head_i}


FONTS = _setup_fonts()


def rect(c, x, y, w, h, color):
    c.setFillColor(color)
    c.rect(x, y, w, h, stroke=0, fill=1)


def hairline(c, x, y, w, color=RULE, weight=0.6):
    c.setStrokeColor(color)
    c.setLineWidth(weight)
    c.line(x, y, x + w, y)


def y_from_top(top_inches):
    return PAGE_H - top_inches * inch


def text(c, x_in, top_in, body, *, size, color=CHARCOAL, font=None,
         italic=False, bold=False, align="left", line_spacing=1.15,
         max_width_in=None):
    if font is None:
        font = FONTS["italic"] if italic else (FONTS["bold"] if bold else FONTS["regular"])
    c.setFillColor(color)
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


def page_chrome(c, *, page_no, section):
    rect(c, 0, 0, PAGE_W, PAGE_H, IVORY)
    hairline(c, 0.75 * inch, y_from_top(0.55), 11.83 * inch,
             color=RULE, weight=0.6)
    text(c, 0.75, 0.30, section.upper(), size=8.5, color=SLATE,
         italic=True)
    text(c, 8.0, 0.30, "TSI-CITADEL  ·  OPPORTUNITY BRIEF",
         size=8.5, color=SLATE, italic=True, align="right",
         max_width_in=4.83)
    hairline(c, 0.75 * inch, y_from_top(7.05), 11.83 * inch,
             color=RULE, weight=0.45)
    text(c, 0.75, 7.12,
         "Prepared for Alan Weiss  ·  CFO, CS Analytical Laboratory  ·  Confidential",
         size=8, color=SLATE, italic=True)
    text(c, 8.0, 7.12, f"{page_no:02d} / {TOTAL:02d}",
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


def slide_thesis(c):
    page_chrome(c, page_no=2, section="The thesis")
    text(c, 0.75, 1.05, "Why we're sending this",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Three industry shifts compounded\nin one six-month window.",
         size=28, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(4.15), 11.83 * inch, color=RULE)

    shifts = [
        ("USP <382>  ·  Dec 1 2025",
         "Elastomer functional suitability\nshifts to the drug manufacturer.\nUSP: 'early adoption is encouraged.'"),
        ("FDA agentic AI  ·  Dec 1 2025",
         "All FDA employees — including\ninspectors — receive secure agentic\nAI. Regulator is no longer\nagentically naive."),
        ("Nelson acquisitive  ·  May 2026",
         "Sotera Health Q1 2026: 'exploring\nstrategic acquisitions to enhance\nNelson Labs' pharmaceutical\ncapabilities.' NJ precedent exists."),
    ]
    x = 0.75
    col_w = 3.95
    for label, body in shifts:
        text(c, x, 4.35, label, size=12.5, color=NAVY)
        rect(c, x * inch, y_from_top(4.85), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 5.05, body, size=11, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_company(c):
    page_chrome(c, page_no=3, section="The company")
    text(c, 0.75, 1.05, "CS Analytical at a glance — outside view",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "The only cGMP-built,\nFDA-regulated lab in the world\ndedicated to container testing.",
         size=22, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(4.55), 2.5 * inch,
             color=BURGUNDY, weight=1.25)

    pillars = [
        ("Where & what",
         "Clifton, NJ. Doubling lab\nspace, May 2026. USP <1207>\ndeterministic portfolio plus\nUSP <382>, <87>, <788>,\ngas, micro."),
        ("Who — leadership",
         "Brian Mulhall, CEO.\nAlan Weiss, CFO, leads\nfinance, BD, HR. Brandon\nZurawlow, CSO — PDA\nTR-86 contributor."),
        ("Recent moves",
         "RM Analytical (Feb 2026).\nClifton doubling (May 2026).\nInterphex 2026 showcase:\nIV bag CCIT, <382>, cell-\ntherapy distribution."),
        ("Position",
         "Vendor-neutral. Method-led.\nFounder-accountable on\nevery study. Inspected\nby design, not retrofit."),
    ]
    x = 0.75
    col_w = 2.95
    for title, body in pillars:
        rect(c, x * inch, y_from_top(4.74), (col_w - 0.15) * inch, 2, BURGUNDY)
        text(c, x, 4.85, title, size=13, color=NAVY)
        text(c, x, 5.4, body, size=10.5, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_market(c, chart_path):
    page_chrome(c, page_no=4, section="The market")
    text(c, 0.75, 1.05, "What sits in front of you",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "A $9.5 B host market —\nand a $1.5 B niche moving with it.",
         size=22, color=NAVY, line_spacing=1.12)

    c.drawImage(str(chart_path), 0.75 * inch, y_from_top(6.8),
                width=7.6 * inch, height=3.4 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 8.7 * inch, y_from_top(3.6), 0.6 * inch, color=BURGUNDY)
    text(c, 8.7, 3.85, "Pharma analytical testing\noutsourcing: $9.5 B (2025),\n~9% CAGR.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.7, 4.7, "CCIT services carve-out:\n$1.5 B (2025) → $2.4 B (2030).",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.7, 5.45, "BioPlan 2025: outsourcing\nbudgets surged 11%, U.S.\ncaptured 75% of intentions.\nTop-5 CDMOs hold only 15%.",
         size=11, color=CHARCOAL, line_spacing=1.4)

    text(c, 0.75, 6.85,
         "Sources: Precedence Research, Grand View, MarketsAndMarkets, BioPlan Associates 22nd Annual Report (Jul 2025). Composite midpoints.",
         size=7.5, color=SLATE, italic=True)


def slide_tailwinds(c):
    page_chrome(c, page_no=5, section="Tailwinds")
    text(c, 0.75, 1.05, "Five forces, one quarter",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Each alone would justify a faster plan.\nAll five landed in the same window.",
         size=18, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(3.95), 11.83 * inch, color=RULE)

    forces = [
        ("USP <382>",
         "Dec 1 2025. System-level\nelastomer testing. Burden\nshifts to drug manufacturer.\nUSP: 'early adoption\nencouraged.'"),
        ("Annex 1",
         "EU GMP Annex 1 requires\nvalidated deterministic CCIT.\nGrade A bioContamination\nlimit = zero. Lyo pre-batch\nsterilization enforced."),
        ("GLP-1 / PFS",
         "Prefilled syringes\n$9.7B → $18.1B by 2031.\nGLP-1 autoinjectors at\n15.6% CAGR. COP barrels\nadopted; methods reset."),
        ("503B sterility crisis",
         "GenoGenix recall Jul 2025.\nProRx Oct 2025: 36,000+\nsemaglutide vials. 1,150\nadverse events filed YTD.\n503B is now a CCIT buyer."),
        ("Biologics share",
         "~40% of FDA novel\napprovals. Every one in\na sterile primary container.\nGlass-delamination recalls\nback in 2026 headlines."),
    ]
    x = 0.75
    col_w = 2.40
    for title, body in forces:
        text(c, x, 4.15, title, size=12.5, color=NAVY)
        rect(c, x * inch, y_from_top(4.6), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.8, body, size=10, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_container_wave(c, chart_path):
    page_chrome(c, page_no=6, section="The container-format wave")
    text(c, 0.75, 1.05, "New containers. New methods. No incumbent.",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Each new format is a discrete\nCCIT method-development engagement.",
         size=18, color=NAVY, line_spacing=1.12)

    c.drawImage(str(chart_path), 0.75 * inch, y_from_top(6.8),
                width=8.2 * inch, height=3.4 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 9.2 * inch, y_from_top(3.4), 0.6 * inch, color=BURGUNDY)
    text(c, 9.2, 3.6,
         "None of the generalists\nhas these validated.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 9.2, 4.4,
         "CS Analytical's CSO has\npublicly presented on\ncryogenic CCIT — the\nonly credentialed claim.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 9.2, 5.85,
         "Every launch above is\na billable engagement\nwaiting to be sized.",
         size=11, color=CHARCOAL, line_spacing=1.4)


def slide_regulator_adopted(c):
    page_chrome(c, page_no=7, section="Why now")
    text(c, 0.75, 1.05, "December 1, 2025.  Two things became official.",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "The regulator is no longer agentically naive.",
         size=24, color=NAVY, line_spacing=1.1)

    # Top panel — USP <382> on navy
    rect(c, 0.75 * inch, y_from_top(5.05), 11.83 * inch, 1.85 * inch, NAVY)
    text(c, 1.0, 3.35, "USP <382>", size=13, color=GOLD, italic=True)
    text(c, 1.0, 3.75,
         "Elastomeric Component Functional Suitability — official.\nSystem-level testing. Responsibility shifts from elastomer\nsupplier to drug manufacturer. USP: 'early adoption is encouraged.'",
         size=12, color=IVORY, line_spacing=1.4)

    # Bottom panel — FDA on ivory with burgundy rule
    rect(c, 0.75 * inch, y_from_top(6.85), 11.83 * inch, 1.7 * inch, IVORY)
    rect(c, 0.75 * inch, y_from_top(6.85), 0.04 * inch, 1.7 * inch, BURGUNDY)
    text(c, 1.0, 5.3, "The FDA — same day",
         size=13, color=BURGUNDY, italic=True)
    text(c, 1.0, 5.7,
         "Agentic AI deployed to all FDA employees. Secure GovCloud, no\ntraining on industry submissions. Supports pre-market review,\npost-market surveillance, INSPECTIONS, and compliance.",
         size=12, color=CHARCOAL, line_spacing=1.4)


def slide_competitive(c, chart_path):
    page_chrome(c, page_no=8, section="The competitive field")
    text(c, 0.75, 1.05, "Who else is in the room",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Generalists are broad.\nSpecialists are scaling. The corner is open.",
         size=22, color=NAVY, line_spacing=1.1)

    c.drawImage(str(chart_path), 0.75 * inch, y_from_top(6.8),
                width=7.8 * inch, height=3.8 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 8.8 * inch, y_from_top(3.2), 0.6 * inch, color=BURGUNDY)
    text(c, 8.8, 3.4,
         "Three competitive moves in\nthe last six months:",
         size=10.5, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.8, 4.05,
         "Nelson Labs doubling\ncleanrooms and publicly\nacquisitive (Sotera Q1).",
         size=10.5, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.8, 5.05,
         "West Pharma 165k sq ft\nDublin (Mar 2026) for\nGLP-1 vertical integration.",
         size=10.5, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.8, 6.1,
         "Eurofins adding 157k m²\nof lab space in 2025-2026.",
         size=10.5, color=CHARCOAL, line_spacing=1.4)


def slide_in_motion(c):
    page_chrome(c, page_no=9, section="Already in motion")
    text(c, 0.75, 1.05,
         "What CS Analytical is already doing right",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "This is not a turnaround brief.\nIt is an acceleration brief.",
         size=22, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.95), 11.83 * inch, color=RULE)

    moves = [
        ("RM Analytical  ·  Feb 2026",
         "Raw material and excipient\ntesting under a sister brand."),
        ("Clifton expansion  ·  May 2026",
         "Doubling the lab footprint.\nCapacity ahead of demand."),
        ("Micro, gas, IV bag",
         "USP <87>, <788>, gas.\nIV bag CCIT in 2026 catalog."),
        ("Interphex 2026 showcase",
         "IV bag, USP <382>, cell-\ntherapy distribution testing."),
    ]
    x = 0.75
    col_w = 2.95
    for title, body in moves:
        text(c, x, 4.15, title, size=12.5, color=NAVY)
        rect(c, x * inch, y_from_top(4.65), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.8, body, size=11, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_organic(c):
    page_chrome(c, page_no=10,
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
         "Named lab for the post-Dec 2025 elastomer rush."),
        ("02", "GLP-1 / PFS HVLD productization",
         "HVLD for COP-barrel autoinjectors. High margin."),
        ("03", "Annex 1 European mandate",
         "EU GMP Annex 1 opens European sponsors."),
        ("04", "Cell, gene & cryo container methods",
         "No incumbent method. CSO credentialed for this."),
        ("05", "Regulatory consulting carve-out",
         "Productize the advisory work given away today."),
    ]
    y = 4.1
    for num, title, body in vectors:
        text(c, 0.75, y, num, size=22, color=BURGUNDY, italic=True)
        text(c, 1.7, y + 0.08, title, size=14, color=NAVY)
        text(c, 6.3, y + 0.1, body, size=11.5, color=CHARCOAL,
             italic=True, line_spacing=1.35)
        y += 0.55


def slide_agentic(c):
    page_chrome(c, page_no=11, section="Agentic AI — TSI-Citadel angle")
    text(c, 0.75, 1.05,
         "Where TSI-Citadel multiplies what's already here",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "McKinsey: 35-45% productivity gain\nacross 270 pharma workflows.",
         size=18, color=NAVY, line_spacing=1.1)
    text(c, 0.75, 3.15, "Five agents we would build first.",
         size=11, color=SLATE, italic=True)
    hairline(c, 0.75 * inch, y_from_top(3.75), 11.83 * inch, color=RULE)

    uses = [
        ("USP <382>\nwave-capture",
         "Monitors every 483 / WL\nfor elastomer findings.\nResolves to sponsor list.\nDrafts targeted outreach."),
        ("Container-\nformat radar",
         "Tracks SCHOTT, Stevanato,\nBD, West, Bonfiglioli\nlaunches. Sizes the\nmethod-dev opportunity."),
        ("Inspection-\nreadiness mirror",
         "Mirror to FDA's own\nagentic inspector.\nPredicts what AI-augmented\ninspectors will find."),
        ("CAR-T cryo\nmethod pipeline",
         "Phase II/III CAR-T radar.\nCryo container choices.\nFailure-rate economics\n(3.87%-25%) drive demand."),
        ("Strategic-\noptionality tracker",
         "CFO-only quarterly view.\nComparable transactions,\nacquirer activity, market\nposition. Board-friendly."),
    ]
    x = 0.75
    col_w = 2.40
    for title, body in uses:
        text(c, x, 4.0, title, size=12, color=NAVY)
        rect(c, x * inch, y_from_top(4.6), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.8, body, size=10, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_not_experimental(c, chart_path):
    page_chrome(c, page_no=12,
                section="The agentic layer is already commercial")
    text(c, 0.75, 1.05,
         "Six top-25 pharmas.  One million cases.  95% accuracy.",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Agentic AI in pharma is no longer experimental.",
         size=20, color=NAVY, line_spacing=1.1)

    c.drawImage(str(chart_path), 0.75 * inch, y_from_top(6.5),
                width=7.6 * inch, height=3.6 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 8.5 * inch, y_from_top(3.0), 0.6 * inch, color=BURGUNDY)
    text(c, 8.5, 3.2,
         "ArisGlobal NavaX: 1M+\ncases. 95% accuracy. 30%\nefficiency. 120% Y/Y\nbookings Q1 2026.",
         size=10.5, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.5, 4.6,
         "Sixth top-25 pharma by\nJun 2025. Top-20 adopted\nSignals + Distribution\nAgents in 6 weeks.",
         size=10.5, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.5, 5.9,
         "FDA itself deployed agentic\nAI Dec 1, 2025. The question\nis no longer if. It is who,\nwhen, and at what tempo.",
         size=10.5, color=CHARCOAL, line_spacing=1.4)


def slide_roi(c, roi_chart, calendar_chart):
    page_chrome(c, page_no=13, section="ROI shape — for the CFO")
    text(c, 0.75, 1.05, "Three revenue paths.  One calendar recovery.",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Conservative pays for itself.\nAggressive changes the category.",
         size=20, color=NAVY, line_spacing=1.1)

    # Two charts side-by-side
    c.drawImage(str(roi_chart), 0.75 * inch, y_from_top(6.0),
                width=6.0 * inch, height=2.85 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')
    c.drawImage(str(calendar_chart), 7.0 * inch, y_from_top(6.0),
                width=5.8 * inch, height=2.85 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 0.75 * inch, y_from_top(6.3), 0.6 * inch, color=BURGUNDY)
    text(c, 0.75, 6.45,
         "Strategic optionality: Nelson Labs (Sotera) publicly named pharmaceutical-capability acquisitions a 2026 priority. The 2018",
         size=9.5, color=CHARCOAL, italic=True, line_spacing=1.4)
    text(c, 0.75, 6.65,
         "Gibraltar Labs (NJ) precedent is on the record. Life-sciences tools subsector: 18-25× EBITDA on recurring revenue.",
         size=9.5, color=CHARCOAL, italic=True, line_spacing=1.4)


def slide_gibraltar(c):
    page_chrome(c, page_no=14, section="Strategic optionality")
    text(c, 0.75, 1.05,
         "Nelson Labs has done this before.  In New Jersey.",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6, "The Gibraltar precedent.",
         size=30, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.6), 11.83 * inch, color=RULE)

    text(c, 0.75, 3.85, "AUGUST 2018",
         size=10, color=BURGUNDY, italic=True)
    text(c, 0.75, 4.25, "The transaction", size=16, color=NAVY)
    text(c, 0.75, 4.85,
         "Sotera Health's Nelson Labs\nacquires Gibraltar Laboratories.\nFairfield, NJ. Family-owned since\n1970. FDA-registered. ISO 17025.\nUSP-compendial microbiology and\nanalytical chemistry. Two tri-state\nfacilities.",
         size=11.5, color=CHARCOAL, line_spacing=1.4)

    text(c, 6.85, 3.85, "MAY 2026",
         size=10, color=BURGUNDY, italic=True)
    text(c, 6.85, 4.25, "The statement", size=16, color=NAVY)
    text(c, 6.85, 4.85,
         "Sotera Health Q1 2026 earnings\ncall: 'Targeted acquisitions or\npartnerships could add testing\ncapability or technology depth.'\nManagement 'exploring strategic\nacquisitions to enhance Nelson\nLabs' pharmaceutical capabilities.'",
         size=11.5, color=CHARCOAL, line_spacing=1.4)

    hairline(c, 0.75 * inch, y_from_top(6.85), 2.5 * inch,
             color=BURGUNDY, weight=1.25)
    text(c, 0.75, 7.0,
         "The structural parallel is uncomfortable to ignore.",
         size=14, color=NAVY, italic=True)


def slide_adjacent(c):
    page_chrome(c, page_no=15, section="Adjacent opportunities")
    text(c, 0.75, 1.05, "Where else this team could play",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Adjacencies, not pivots.\nOptional, not required.",
         size=20, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.85), 11.83 * inch, color=RULE)

    items = [
        ("Combination products",
         "Cross-center (CDER + CDRH) submissions. Few labs ready."),
        ("Device sterilization validation",
         "ISO 11135 / 11137 adjacency. Same sponsor base."),
        ("Cell-therapy distribution",
         "ISTA/ASTM testing for live-cell packaging. In Interphex slate."),
        ("Smart-packaging integrity",
         "Embedded RFID + tamper-evident integrity (TOPPAC infuse class)."),
        ("Compounding pharmacy",
         "USP <797> / <800>. Fragmented buyer base, less competition."),
        ("Veterinary biologics",
         "Same containers. USDA channel. Underpriced relative to human side."),
    ]
    y = 4.05
    for title, body in items:
        text(c, 0.75, y, title, size=12.5, color=NAVY)
        text(c, 4.6, y + 0.05, body, size=11.5, color=CHARCOAL,
             line_spacing=1.4)
        y += 0.52


def slide_risks(c):
    page_chrome(c, page_no=16, section="Risks — named, not glossed")
    text(c, 0.75, 1.05, "What we'd want to disagree about",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6, "Three CFO-relevant risks,\nthree handlings.",
         size=22, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.85), 11.83 * inch, color=RULE)

    risks = [
        ("Capital timing",
         "Lab expansion + RMA + agentic in one fiscal year. Sequence\nagentic behind the lab build; agentic pays back in <12 months."),
        ("Margin dilution from adjacencies",
         "RMA and micro carry lower margin than CCIT. Hold CCIT pricing\nwhile RMA scales; agentic intel keeps the core pipeline full."),
        ("Hallucination in regulated context",
         "Human-in-the-loop on every agent flag touching a filed method.\nAgents are suggestive, not authoritative — by design."),
    ]
    y = 4.05
    for title, body in risks:
        text(c, 0.75, y, title, size=13, color=NAVY)
        text(c, 4.8, y + 0.05, body, size=11, color=CHARCOAL,
             line_spacing=1.4)
        y += 0.95


def slide_engage(c):
    page_chrome(c, page_no=17, section="If you wanted to work with us")
    text(c, 0.75, 1.05, "How TSI-Citadel would engage",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Light footprint. Boxed scope.\nThe CFO holds the gate at day 30 and day 90.",
         size=20, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.95), 11.83 * inch, color=RULE)

    steps = [
        ("DAYS 1 — 30", "Diagnostic",
         "Map data feeds, regulatory\ncycles, inquiry-to-quote flow.\nDeliverables: exposure map,\nBD pipeline seed, regulatory-\nintel trial. No code yet.\nGo / no-go at day 30."),
        ("DAYS 31 — 60", "Two agents live",
         "Regulatory-intel and market-\nradar agents wired to real\nfeeds. Daily output to a\nnamed internal user. First\nmeasurable lift inside\nthe period."),
        ("DAYS 61 — 90", "Decide",
         "Joint readout. Numbers.\nThe three remaining agents\nbuilt only if the first two\nearned them. Fixed pricing\nat outset. CFO holds\nthe gate."),
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

    build = ROOT / "_build"
    c_market    = build / "chart_market.png"
    c_comp      = build / "chart_competitive.png"
    c_roi       = build / "chart_roi.png"
    c_container = build / "chart_container_timeline.png"
    c_agentic   = build / "chart_agentic_adoption.png"
    c_calendar  = build / "chart_calendar_recovery.png"

    c = canvas.Canvas(str(OUT_PDF), pagesize=(PAGE_W, PAGE_H))
    c.setTitle("CS Analytical Opportunity Brief — for Alan Weiss")
    c.setAuthor("Bruce Longley, TSI-Citadel")
    c.setSubject("Market, organic growth, and agentic-AI opportunity")

    slide_cover(c);                       c.showPage()
    slide_thesis(c);                      c.showPage()
    slide_company(c);                     c.showPage()
    slide_market(c, c_market);            c.showPage()
    slide_tailwinds(c);                   c.showPage()
    slide_container_wave(c, c_container); c.showPage()
    slide_regulator_adopted(c);           c.showPage()
    slide_competitive(c, c_comp);         c.showPage()
    slide_in_motion(c);                   c.showPage()
    slide_organic(c);                     c.showPage()
    slide_agentic(c);                     c.showPage()
    slide_not_experimental(c, c_agentic); c.showPage()
    slide_roi(c, c_roi, c_calendar);      c.showPage()
    slide_gibraltar(c);                   c.showPage()
    slide_adjacent(c);                    c.showPage()
    slide_risks(c);                       c.showPage()
    slide_engage(c);                      c.showPage()
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
