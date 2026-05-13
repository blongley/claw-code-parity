"""Build the CS Analytical executive deck as a vector PDF.

Mirrors build_deck.py (PowerPoint) one-for-one — same slide order, same
copy, same Caslon serif system. We use a Caslon-family font when one is
installed; otherwise we fall back to a registered serif (Charter / Times)
so the layout stays book-grade.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

from reportlab.lib.pagesizes import landscape
from reportlab.lib.units import inch
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas

ROOT = Path(__file__).resolve().parent.parent
OUT_PDF = ROOT / "02_PDF_Deck" / "CS_Analytical_Executive_Brief.pdf"
SLIDES_DIR = ROOT / "03_Slide_Images"

# 16:9 page, sized in inches converted to points
PAGE_W = 13.333 * inch
PAGE_H = 7.5 * inch

# Palette
IVORY = (0xFA / 255, 0xF7 / 255, 0xF2 / 255)
NAVY = (0x0B / 255, 0x25 / 255, 0x45 / 255)
BURGUNDY = (0x6E / 255, 0x1B / 255, 0x26 / 255)
CHARCOAL = (0x1C / 255, 0x1C / 255, 0x1C / 255)
SLATE = (0x55 / 255, 0x5B / 255, 0x66 / 255)
GOLD = (0xB5 / 255, 0x8B / 255, 0x3A / 255)
RULE = (0xC9 / 255, 0xBE / 255, 0xA8 / 255)


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


def setup_fonts():
    candidates_regular = [
        "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    ]
    candidates_bold = [
        "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf",
    ]
    candidates_italic = [
        "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Italic.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf",
    ]
    head = body = "Times-Roman"
    head_bold = body_bold = "Times-Bold"
    head_italic = body_italic = "Times-Italic"
    for c in candidates_regular:
        if _try_register("CaslonLike", c):
            head = body = "CaslonLike"
            break
    for c in candidates_bold:
        if _try_register("CaslonLike-Bold", c):
            head_bold = body_bold = "CaslonLike-Bold"
            break
    for c in candidates_italic:
        if _try_register("CaslonLike-Italic", c):
            head_italic = body_italic = "CaslonLike-Italic"
            break
    return {
        "regular": head,
        "bold": head_bold,
        "italic": head_italic,
    }


FONTS = setup_fonts()


# ---------------------------------------------------------------------------
# Drawing primitives
# ---------------------------------------------------------------------------

def fill(c, rgb):
    c.setFillColorRGB(*rgb)


def stroke(c, rgb):
    c.setStrokeColorRGB(*rgb)


def rect(c, x, y, w, h, color):
    fill(c, color)
    c.rect(x, y, w, h, stroke=0, fill=1)


def hairline(c, x, y, w, color=RULE, weight=0.6):
    stroke(c, color)
    c.setLineWidth(weight)
    c.line(x, y, x + w, y)


def y_from_top(top_inches):
    """Convert a top-anchored inches coordinate to reportlab bottom-anchored points."""
    return PAGE_H - top_inches * inch


def text(c, x_in, top_in, body, *, size, color=CHARCOAL, font=None,
         italic=False, bold=False, align="left", line_spacing=1.15,
         max_width_in=None):
    """Draw paragraphs with manual line breaks. Anchors at top-in inches from top."""
    if font is None:
        font = FONTS["italic"] if italic else (FONTS["bold"] if bold else FONTS["regular"])
    fill(c, color)
    c.setFont(font, size)

    line_height = size * line_spacing
    x = x_in * inch
    y = y_from_top(top_in) - size  # baseline of the first line

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
    hairline(c, 0.75 * inch, y_from_top(0.55), 11.83 * inch, color=RULE, weight=0.6)
    text(c, 0.75, 0.30, section.upper(), size=8.5, color=SLATE,
         font=FONTS["italic"], italic=True)
    text(c, 8.0, 0.30, "CS ANALYTICAL  ·  EXECUTIVE BRIEF",
         size=8.5, color=SLATE, italic=True, align="right", max_width_in=4.83)
    hairline(c, 0.75 * inch, y_from_top(7.05), 11.83 * inch, color=RULE, weight=0.45)
    text(c, 0.75, 7.12, "Prepared for Al Weiss  ·  Confidential",
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
         "An executive brief\non market, opportunity,\nand the case for agentic AI.",
         size=44, color=NAVY, line_spacing=1.1)

    hairline(c, 1.1 * inch, y_from_top(5.55), 2.2 * inch, color=NAVY, weight=1.0)
    text(c, 1.1, 5.75, "Prepared for Al Weiss", size=18, color=CHARCOAL)
    text(c, 1.1, 6.15,
         "By Brian Mulhall  ·  Chief Executive Officer  ·  CS Analytical",
         size=12, color=SLATE, italic=True)
    text(c, 1.1, 6.5, "May 2026", size=10, color=SLATE)


def slide_thesis(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The thesis")
    text(c, 0.75, 1.05, "One sentence.", size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Container Closure Integrity is the\nsingle most regulated, most ignored,\nand most under-served line item\non a sterile-drug spec sheet.",
         size=36, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(5.85), 2.5 * inch,
             color=BURGUNDY, weight=1.25)
    text(c, 0.75, 6.05,
         "CS Analytical exists to fix that — and to scale\nthe fix with agentic AI partners.",
         size=16, color=CHARCOAL, italic=True, line_spacing=1.3)


def slide_operator(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The operator")
    text(c, 0.75, 1.05, "Brian Mulhall  —  Chief Executive Officer",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Thirty years building the labs the\nindustry actually trusts.",
         size=30, color=NAVY, line_spacing=1.1)

    hairline(c, 0.75 * inch, y_from_top(3.7), 11.83 * inch, color=RULE)
    cols = [
        ("1990s",
         "Sales, marketing,\nand national management\nat Organon (Schering Plough)\nand Ferring."),
        ("Late 1990s",
         "VP, Pharmaceutical Services,\nSGS US Testing — chemistry,\nmicrobiology, and toxicology\nacross the U.S. and Canada."),
        ("2002 — 2015",
         "Founded Whitehouse\nAnalytical Laboratories.\nBuilt to 48 FTE, 25,000 sq ft.\nSold in 2015."),
        ("2015 — today",
         "CEO, Leak Detection Associates.\nAdvisor and BD lead, Visikol.\nNow CEO, CS Analytical."),
    ]
    x = 0.75
    col_w = 2.95
    for label, body in cols:
        text(c, x, 3.9, label, size=10, color=BURGUNDY, italic=True)
        text(c, x, 4.3, body, size=12, color=CHARCOAL, line_spacing=1.4)
        x += col_w

    text(c, 0.75, 6.6,
         "Architect of the first FDA-regulated, cGMP Container Closure Integrity Testing laboratory in the world —",
         size=10, color=SLATE, italic=True)
    text(c, 0.75, 6.82, "the work that informed USP <1207>.",
         size=10, color=SLATE, italic=True)


def slide_company(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The company")
    text(c, 0.75, 1.05, "CS Analytical Laboratory",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "A center of excellence,\nnot a generalist lab.",
         size=36, color=NAVY, line_spacing=1.08)
    hairline(c, 0.75 * inch, y_from_top(4.15), 2.5 * inch,
             color=BURGUNDY, weight=1.25)

    pillars = [
        ("Specialty depth",
         "CCIT done at a level the\nbig labs can't staff for —\nhelium leak, HVLD, vacuum\ndecay, dye ingress, methods\ndevelopment, validation."),
        ("Regulatory posture",
         "Built to USP <1207>.\ncGMP from the floor up.\nMethods filed, not promised.\nThe inspectors already\nknow the address."),
        ("Operator's company",
         "Founder-led, scientist-staffed,\nclient-facing PhDs. No\nhand-offs. The person\nrunning your study is the\nperson who designed it."),
    ]
    x = 0.75
    col_w = 3.95
    for title, body in pillars:
        rect(c, x * inch, y_from_top(4.34), (col_w - 0.15) * inch, 2, BURGUNDY)
        text(c, x, 4.45, title, size=16, color=NAVY)
        text(c, x, 5.0, body, size=12, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_market_size(c, page, total, chart_path):
    page_chrome(c, page_no=page, total=total, section="The market")
    text(c, 0.75, 1.05, "Where the money sits",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "A $6–7 B contract analytical market —\nand a $300 M niche growing twice as fast.",
         size=22, color=NAVY, line_spacing=1.15)

    c.drawImage(str(chart_path), 0.75 * inch, y_from_top(6.8),
                width=7.6 * inch, height=3.4 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 8.7 * inch, y_from_top(3.6), 0.6 * inch, color=BURGUNDY)
    text(c, 8.7, 3.85,
         "Pharma analytical testing\noverall:  $6.4 B,  ~9% CAGR.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.7, 4.65,
         "CCIT carve-out:  $280 M\ntoday  →  $475 M by 2030.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.7, 5.45,
         "That is the bet:\nthe carve-out grows faster\nthan the host market — and\nCS Analytical is built for the\ncarve-out, not the host.",
         size=11, color=CHARCOAL, line_spacing=1.4)

    text(c, 0.75, 6.85,
         "Figures are illustrative composites of publicly cited industry ranges. Intended for strategic discussion.",
         size=7.5, color=SLATE, italic=True)


def slide_tailwinds(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Tailwinds")
    text(c, 0.75, 1.05, "Why now", size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Four forces are pushing CCIT\nfrom afterthought to gating step.",
         size=26, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.8), 11.83 * inch, color=RULE)

    forces = [
        ("Biologics",
         "Roughly 40% of FDA novel\napprovals are biologics. Every\none of them ships in a sterile\nprimary container."),
        ("GLP-1s",
         "Multi-pen, pre-filled,\nhigh-volume launches.\nIntegrity defects scale\nwith volume."),
        ("Cell, gene, mRNA",
         "Cryo storage. Closed systems.\nNo room for leak. Methods\ndon't yet exist for half of\nthese formats."),
        ("USP <1207>",
         "Deterministic methods are\nthe expectation. Probabilistic\nmethods are increasingly\ndefensive only."),
    ]
    x = 0.75
    col_w = 2.95
    for title, body in forces:
        text(c, x, 4.05, title, size=16, color=NAVY)
        rect(c, x * inch, y_from_top(4.5), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.7, body, size=11.5, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_competitive(c, page, total, chart_path):
    page_chrome(c, page_no=page, total=total, section="The field")
    text(c, 0.75, 1.05, "Who is in the room",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Big labs are broad. CCIT\nspecialists are few.",
         size=28, color=NAVY, line_spacing=1.1)

    c.drawImage(str(chart_path), 0.75 * inch, y_from_top(6.8),
                width=7.8 * inch, height=3.8 * inch,
                preserveAspectRatio=True, anchor="nw", mask='auto')

    hairline(c, 8.8 * inch, y_from_top(3.2), 0.6 * inch, color=BURGUNDY)
    text(c, 8.8, 3.4,
         "The big-five generalists win\non footprint and pricing power.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.8, 4.2,
         "The specialists — Nelson Labs,\nWhitehouse, Boston Analytical,\nWest — win on depth in one\nor two modalities.",
         size=11, color=CHARCOAL, line_spacing=1.4)
    text(c, 8.8, 5.55,
         "The upper-right quadrant —\ndeep AND modern AND fast —\nis essentially empty. That is\nthe CS Analytical position.",
         size=11, color=CHARCOAL, line_spacing=1.4)


def slide_edge(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The edge")
    text(c, 0.75, 1.05, "Why clients pick us",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Vendor-neutral, method-led,\nfounder-accountable.",
         size=28, color=NAVY, line_spacing=1.1)

    items = [
        ("Vendor-neutral",
         "We own no leak-detection platform — so we recommend the right one.\nPTI, LDA, Bonfiglioli, ATEQ — we run them all and tell you which fits\nyour container."),
        ("Method-led, not box-led",
         "We start with the molecule, the container, and the failure mode.\nThe instrument is the last decision, not the first."),
        ("Founder-accountable",
         "Brian signs the study report. There is no account manager between\nthe client and the scientist."),
        ("Built to be inspected",
         "Every CS Analytical method is filed with FDA-readiness as the design\ngoal — not a retrofit."),
    ]
    hairline(c, 0.75 * inch, y_from_top(3.65), 11.83 * inch, color=RULE)
    y = 3.85
    for title, body in items:
        text(c, 0.75, y, title, size=14, color=NAVY)
        text(c, 4.4, y, body, size=12, color=CHARCOAL, line_spacing=1.4)
        y += 0.9


def slide_growth_vectors(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Growth vectors")
    text(c, 0.75, 1.05, "Where we want to go",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Five vectors. Sequenced, not\nstacked.",
         size=28, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.7), 11.83 * inch, color=RULE)

    vectors = [
        ("01", "Sterile-injectable CCIT",
         "The core. Defend, deepen, raise price."),
        ("02", "Cell & gene cryo containers",
         "New methods, new formats. First mover."),
        ("03", "Pre-filled syringe / autoinjector",
         "GLP-1 wave demands repeatable HVLD."),
        ("04", "Combination products & device-led",
         "FDA cross-center — few labs are ready."),
        ("05", "Regulatory consulting carve-out",
         "Sell the brain, not just the bench."),
    ]
    y = 3.95
    for num, title, body in vectors:
        text(c, 0.75, y, num, size=22, color=BURGUNDY, italic=True)
        text(c, 1.7, y + 0.08, title, size=15, color=NAVY)
        text(c, 6.3, y + 0.1, body, size=12, color=CHARCOAL, italic=True)
        y += 0.6


def slide_tsi_citadel(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The agentic layer")
    text(c, 0.75, 1.05, "Where TSI Citadel multiplies us",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "We are a 48-person lab. Agentic AI\ngives us a 480-person reach.",
         size=24, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.85), 11.83 * inch, color=RULE)

    uses = [
        ("Market radar",
         "Continuous scan of FDA filings,\nclinical trials, conference\nrosters, container-vendor\ncatalogs — turned into a\nweekly BD shortlist."),
        ("Regulatory intelligence",
         "Track USP, EP, JP, PIC/S\nchanges in real time. Flag\nthe ones that break a\nclient's filed method."),
        ("BD operations",
         "Outbound, qualification,\nfollow-up, meeting prep.\nThe work that adds no\nscience but consumes\nfounder hours."),
        ("Scientific literature",
         "Daily distillation of CCIT,\nHVLD, helium-leak, and\ncontainer-vendor literature.\nMethods updates surfaced\nbefore the customer asks."),
    ]
    x = 0.75
    col_w = 2.95
    for title, body in uses:
        text(c, x, 4.1, title, size=14, color=NAVY)
        rect(c, x * inch, y_from_top(4.55), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.75, body, size=10.5, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_90_day(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Ninety days")
    text(c, 0.75, 1.05, "What we'd do first",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Three workstreams, ninety days,\none readout.",
         size=26, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.85), 11.83 * inch, color=RULE)

    streams = [
        ("DAYS 1—30", "Stand up the agentic stack",
         "TSI Citadel agents wired to FDA,\nClinicalTrials.gov, USP, EP, PIC/S,\nconference rosters, container-vendor\nrelease feeds. Weekly BD digest live."),
        ("DAYS 31—60", "Pilot two growth vectors",
         "Cryo-container method development\nand autoinjector HVLD. Two named\npilot clients each. Methods drafted,\nquotes out, first revenue booked."),
        ("DAYS 61—90", "Package and price",
         "Productized CCIT-as-a-service tier.\nRegulatory-consulting carve-out\npriced. Public case studies (two)\nwith client co-sign."),
    ]
    x = 0.75
    col_w = 3.95
    for label, title, body in streams:
        text(c, x, 4.05, label, size=10, color=BURGUNDY, italic=True)
        text(c, x, 4.45, title, size=16, color=NAVY)
        rect(c, x * inch, y_from_top(4.95), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 5.15, body, size=11.5, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_ask(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The ask")
    text(c, 0.75, 1.05, "What we'd love your help on",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "An introduction. A read. A reaction.",
         size=28, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.65), 11.83 * inch, color=RULE)

    asks = [
        ("Introduction",
         "A warm hand-off to the TSI Citadel team — at the\nlevel where the agentic-platform partnership\nconversation can actually begin."),
        ("A read",
         "Twenty minutes of your eyes on the long-form\nmarket brief. Tell us where the thesis is thin and\nwhere it's right."),
        ("A reaction",
         "If you see a client, a partner, or a hire we should\nbe in front of — say the name. We'll do the rest."),
    ]
    y = 3.95
    for title, body in asks:
        text(c, 0.75, y, title, size=18, color=NAVY)
        text(c, 4.4, y + 0.05, body, size=13, color=CHARCOAL, line_spacing=1.4)
        y += 1.05


def slide_closing(c):
    rect(c, 0, 0, PAGE_W, PAGE_H, NAVY)
    rect(c, 0, 0, 0.08 * inch, PAGE_H, BURGUNDY)

    text(c, 1.1, 1.45, "THANK YOU", size=12, color=GOLD, italic=True)
    hairline(c, 1.1 * inch, y_from_top(2.0), 2.5 * inch,
             color=BURGUNDY, weight=1.5)

    text(c, 1.1, 2.35,
         "The right work,\nin the right container,\nat the right cadence.",
         size=44, color=IVORY, line_spacing=1.08)

    text(c, 1.1, 5.55, "Brian Mulhall", size=18, color=IVORY)
    text(c, 1.1, 5.95,
         "Chief Executive Officer  ·  CS Analytical Laboratory",
         size=12, color=GOLD, italic=True)
    text(c, 1.1, 6.3, "brian.mulhall@csanalytical.com",
         size=12, color=IVORY)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    OUT_PDF.parent.mkdir(parents=True, exist_ok=True)
    SLIDES_DIR.mkdir(parents=True, exist_ok=True)

    build_dir = ROOT / "_build"
    market_chart = build_dir / "chart_market.png"
    comp_chart = build_dir / "chart_competitive.png"

    c = canvas.Canvas(str(OUT_PDF), pagesize=(PAGE_W, PAGE_H))
    c.setTitle("CS Analytical — Executive Brief for Al Weiss")
    c.setAuthor("Brian Mulhall, CEO, CS Analytical Laboratory")
    c.setSubject("Market opportunity & agentic AI thesis")

    total = 12

    slide_cover(c);                c.showPage()
    slide_thesis(c, 2, total);     c.showPage()
    slide_operator(c, 3, total);   c.showPage()
    slide_company(c, 4, total);    c.showPage()
    slide_market_size(c, 5, total, market_chart); c.showPage()
    slide_tailwinds(c, 6, total);  c.showPage()
    slide_competitive(c, 7, total, comp_chart);   c.showPage()
    slide_edge(c, 8, total);       c.showPage()
    slide_growth_vectors(c, 9, total); c.showPage()
    slide_tsi_citadel(c, 10, total); c.showPage()
    slide_90_day(c, 11, total);    c.showPage()
    slide_ask(c, 12, total);       c.showPage()
    slide_closing(c);              c.showPage()

    c.save()
    print(f"wrote {OUT_PDF}")

    # Export each page as PNG via pdftoppm
    out_prefix = SLIDES_DIR / "slide"
    subprocess.run(
        [
            "pdftoppm", "-png", "-r", "180",
            str(OUT_PDF), str(out_prefix),
        ],
        check=True,
    )
    print(f"wrote PNGs to {SLIDES_DIR}")


if __name__ == "__main__":
    main()
