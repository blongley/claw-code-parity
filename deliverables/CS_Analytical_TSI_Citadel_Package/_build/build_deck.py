"""Build the CS Analytical executive deck (PowerPoint).

Produces a 16:9 deck styled in a Caslon serif system. Slides follow a
"show, don't tell" rhythm: one headline, one supporting element.
"""

from __future__ import annotations

from pathlib import Path

from pptx import Presentation
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.util import Emu, Inches, Pt

ROOT = Path(__file__).resolve().parent.parent
OUT_PPTX = ROOT / "01_PowerPoint" / "CS_Analytical_Executive_Brief.pptx"

# Caslon font stack — PowerPoint will use whichever Caslon variant is
# installed on the user's machine. "Adobe Caslon Pro" is the most common.
FONT_HEAD = "Adobe Caslon Pro"
FONT_BODY = "Adobe Caslon Pro"

# Palette — warm ivory ground, deep navy authority, burgundy accent.
IVORY = RGBColor(0xFA, 0xF7, 0xF2)
NAVY = RGBColor(0x0B, 0x25, 0x45)
BURGUNDY = RGBColor(0x6E, 0x1B, 0x26)
CHARCOAL = RGBColor(0x1C, 0x1C, 0x1C)
SLATE = RGBColor(0x55, 0x5B, 0x66)
GOLD = RGBColor(0xB5, 0x8B, 0x3A)
RULE = RGBColor(0xC9, 0xBE, 0xA8)

SLIDE_W = Inches(13.333)
SLIDE_H = Inches(7.5)


# ---------------------------------------------------------------------------
# Primitives
# ---------------------------------------------------------------------------

def set_run(run, *, size, color=CHARCOAL, bold=False, italic=False, font=FONT_BODY):
    run.font.name = font
    run.font.size = Pt(size)
    run.font.bold = bold
    run.font.italic = italic
    run.font.color.rgb = color


def add_text(slide, left, top, width, height, text, *, size, color=CHARCOAL,
             bold=False, italic=False, align=PP_ALIGN.LEFT, anchor=MSO_ANCHOR.TOP,
             font=FONT_BODY, line_spacing=1.15):
    box = slide.shapes.add_textbox(left, top, width, height)
    tf = box.text_frame
    tf.word_wrap = True
    tf.margin_left = tf.margin_right = Emu(0)
    tf.margin_top = tf.margin_bottom = Emu(0)
    tf.vertical_anchor = anchor

    lines = text.split("\n") if isinstance(text, str) else text
    for i, line in enumerate(lines):
        para = tf.paragraphs[0] if i == 0 else tf.add_paragraph()
        para.alignment = align
        para.line_spacing = line_spacing
        run = para.add_run()
        run.text = line
        set_run(run, size=size, color=color, bold=bold, italic=italic, font=font)
    return box


def add_rect(slide, left, top, width, height, fill, line=None):
    shape = slide.shapes.add_shape(MSO_SHAPE.RECTANGLE, left, top, width, height)
    shape.fill.solid()
    shape.fill.fore_color.rgb = fill
    if line is None:
        shape.line.fill.background()
    else:
        shape.line.color.rgb = line
        shape.line.width = Pt(0.75)
    shape.shadow.inherit = False
    return shape


def add_hairline(slide, left, top, width, color=RULE, weight=0.75):
    line = slide.shapes.add_connector(1, left, top, left + width, top)
    line.line.color.rgb = color
    line.line.width = Pt(weight)
    return line


def page_chrome(slide, *, page_no, total, section):
    """Standard slide chrome: ivory ground, top rule, footer."""
    bg = add_rect(slide, 0, 0, SLIDE_W, SLIDE_H, IVORY)
    # Top thin rule
    add_hairline(slide, Inches(0.75), Inches(0.55), Inches(11.83), color=RULE, weight=0.75)
    # Top-left section eyebrow
    add_text(slide, Inches(0.75), Inches(0.28), Inches(8), Inches(0.3),
             section.upper(), size=10, color=SLATE, font=FONT_BODY, italic=True)
    # Top-right brand mark
    add_text(slide, Inches(8.0), Inches(0.28), Inches(4.83), Inches(0.3),
             "CS ANALYTICAL  ·  EXECUTIVE BRIEF",
             size=10, color=SLATE, align=PP_ALIGN.RIGHT, font=FONT_BODY, italic=True)
    # Bottom rule
    add_hairline(slide, Inches(0.75), Inches(7.05), Inches(11.83), color=RULE, weight=0.5)
    # Footer left
    add_text(slide, Inches(0.75), Inches(7.12), Inches(8), Inches(0.3),
             "Prepared for Al Weiss  ·  Confidential", size=9, color=SLATE,
             font=FONT_BODY, italic=True)
    # Footer right
    add_text(slide, Inches(8.0), Inches(7.12), Inches(4.83), Inches(0.3),
             f"{page_no:02d} / {total:02d}", size=9, color=SLATE,
             align=PP_ALIGN.RIGHT, font=FONT_BODY)


# ---------------------------------------------------------------------------
# Slide builders
# ---------------------------------------------------------------------------

def slide_cover(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    add_rect(s, 0, 0, SLIDE_W, SLIDE_H, IVORY)
    # Side block
    add_rect(s, 0, 0, Inches(0.6), SLIDE_H, NAVY)
    add_rect(s, Inches(0.6), 0, Inches(0.08), SLIDE_H, BURGUNDY)

    add_text(s, Inches(1.1), Inches(0.85), Inches(11), Inches(0.4),
             "CS ANALYTICAL LABORATORY", size=14, color=SLATE,
             font=FONT_BODY, italic=True)
    add_hairline(s, Inches(1.1), Inches(1.35), Inches(3.5), color=BURGUNDY, weight=1.5)

    add_text(s, Inches(1.1), Inches(1.7), Inches(11), Inches(2.2),
             "An executive brief\non market, opportunity,\nand the case for agentic AI.",
             size=54, color=NAVY, bold=False, font=FONT_HEAD, line_spacing=1.05)

    add_hairline(s, Inches(1.1), Inches(5.55), Inches(2.2), color=NAVY, weight=1.0)
    add_text(s, Inches(1.1), Inches(5.7), Inches(11), Inches(0.4),
             "Prepared for Al Weiss", size=18, color=CHARCOAL, font=FONT_HEAD)
    add_text(s, Inches(1.1), Inches(6.1), Inches(11), Inches(0.4),
             "By Brian Mulhall  ·  Chief Executive Officer  ·  CS Analytical",
             size=13, color=SLATE, font=FONT_BODY, italic=True)
    add_text(s, Inches(1.1), Inches(6.45), Inches(11), Inches(0.4),
             "May 2026", size=11, color=SLATE, font=FONT_BODY)


def slide_thesis(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The thesis")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "One sentence.", size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(3.5),
             "Container Closure Integrity is the\nsingle most regulated, most ignored,\nand most under-served line item\non a sterile-drug spec sheet.",
             size=46, color=NAVY, font=FONT_HEAD, line_spacing=1.08)

    add_hairline(s, Inches(0.75), Inches(5.65), Inches(2.5), color=BURGUNDY, weight=1.25)
    add_text(s, Inches(0.75), Inches(5.8), Inches(11.83), Inches(1.0),
             "CS Analytical exists to fix that — and to scale the\nfix with agentic AI partners.",
             size=18, color=CHARCOAL, italic=True, font=FONT_HEAD, line_spacing=1.25)


def slide_operator(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The operator")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Brian Mulhall  —  Chief Executive Officer",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Thirty years building the labs the\nindustry actually trusts.",
             size=36, color=NAVY, font=FONT_HEAD, line_spacing=1.08)

    # Four-column track record
    add_hairline(s, Inches(0.75), Inches(3.6), Inches(11.83), color=RULE)
    cols = [
        ("1990s", "Sales, marketing,\nand national management\nat Organon (Schering Plough)\nand Ferring."),
        ("Late 1990s", "VP, Pharmaceutical Services,\nSGS US Testing — chemistry,\nmicrobiology, and toxicology\nacross the U.S. and Canada."),
        ("2002 — 2015", "Founded Whitehouse\nAnalytical Laboratories.\nBuilt to 48 FTE, 25,000 sq ft.\nSold in 2015."),
        ("2015 — today", "CEO, Leak Detection Associates.\nAdvisor and BD lead, Visikol.\nNow CEO, CS Analytical."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.95)
    for label, body in cols:
        add_text(s, x, Inches(3.8), col_w, Inches(0.35),
                 label, size=11, color=BURGUNDY, italic=True, font=FONT_BODY)
        add_text(s, x, Inches(4.2), col_w, Inches(2.4),
                 body, size=14, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.3)
        x += col_w

    add_text(s, Inches(0.75), Inches(6.55), Inches(11.83), Inches(0.4),
             "Architect of the first FDA-regulated, cGMP Container Closure Integrity Testing laboratory in the world — the work that informed USP <1207>.",
             size=12, color=SLATE, italic=True, font=FONT_HEAD)


def slide_company(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The company")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "CS Analytical Laboratory",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "A center of excellence,\nnot a generalist lab.",
             size=44, color=NAVY, font=FONT_HEAD, line_spacing=1.05)

    add_hairline(s, Inches(0.75), Inches(4.05), Inches(2.5), color=BURGUNDY, weight=1.25)

    # Three-pillar grid
    pillars = [
        ("Specialty depth",
         "CCIT done at a level the\nbig labs can't staff for —\nhelium leak, HVLD, vacuum\ndecay, dye ingress, methods\ndevelopment, validation."),
        ("Regulatory posture",
         "Built to USP <1207>.\ncGMP from the floor up.\nMethods filed, not promised.\nThe inspectors already\nknow the address."),
        ("Operator's company",
         "Founder-led, scientist-staffed,\nclient-facing PhDs. No\nhand-offs. The person\nrunning your study is the\nperson who designed it."),
    ]
    x = Inches(0.75)
    col_w = Inches(3.95)
    for title, body in pillars:
        add_rect(s, x, Inches(4.3), col_w - Inches(0.15), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.45), col_w - Inches(0.15), Inches(0.5),
                 title, size=18, color=NAVY, font=FONT_HEAD, bold=False)
        add_text(s, x, Inches(5.0), col_w - Inches(0.15), Inches(2.0),
                 body, size=13, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)
        x += col_w


def slide_market_size(prs, page, total, chart_path: Path):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The market")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Where the money sits",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "A $6–7 B contract analytical market —\nand a $300 M niche growing twice as fast.",
             size=28, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    s.shapes.add_picture(str(chart_path), Inches(0.75), Inches(3.4),
                         width=Inches(7.6))

    add_hairline(s, Inches(8.7), Inches(3.6), Inches(0.6), color=BURGUNDY)
    add_text(s, Inches(8.7), Inches(3.75), Inches(4.0), Inches(3.0),
             "Pharma analytical testing\noverall:  $6.4 B,  ~9% CAGR.\n\nCCIT carve-out:  $280 M\ntoday  →  $475 M by 2030.\n\nThat is the bet:\nthe carve-out grows faster\nthan the host market — and\nCS Analytical is built for the\ncarve-out, not the host.",
             size=13, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)

    add_text(s, Inches(0.75), Inches(6.75), Inches(11.83), Inches(0.3),
             "Figures are illustrative composites of publicly cited industry ranges (Grand View, Markets and Markets, BioPlan). Intended for strategic discussion.",
             size=8, color=SLATE, italic=True, font=FONT_BODY)


def slide_tailwinds(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Tailwinds")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Why now",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Four forces are pushing CCIT\nfrom afterthought to gating step.",
             size=30, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
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
    x = Inches(0.75)
    col_w = Inches(2.95)
    for title, body in forces:
        add_text(s, x, Inches(3.9), col_w - Inches(0.15), Inches(0.5),
                 title, size=18, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.35), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.5), col_w - Inches(0.15), Inches(2.5),
                 body, size=13, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)
        x += col_w


def slide_competitive(prs, page, total, chart_path: Path):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The field")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Who is in the room",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Big labs are broad. CCIT\nspecialists are few.",
             size=32, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    s.shapes.add_picture(str(chart_path), Inches(0.75), Inches(3.0),
                         width=Inches(7.8))

    add_hairline(s, Inches(8.8), Inches(3.2), Inches(0.6), color=BURGUNDY)
    add_text(s, Inches(8.8), Inches(3.35), Inches(4.0), Inches(3.5),
             "The big-five generalists —\nEurofins, SGS, Charles River,\nPace, Element — win on\nfootprint and pricing power.\n\nThe specialists — Nelson Labs,\nWhitehouse, Boston Analytical,\nWest — win on depth in one\nor two modalities.\n\nThe upper-right quadrant —\ndeep AND modern AND fast —\nis essentially empty. That is\nthe CS Analytical position.",
             size=12, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.3)


def slide_edge(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The edge")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Why clients pick us",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Vendor-neutral, method-led,\nfounder-accountable.",
             size=34, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    items = [
        ("Vendor-neutral",
         "We own no leak-detection platform — so we recommend the right one. PTI, LDA, Bonfiglioli, ATEQ — we run them all and tell you which fits your container."),
        ("Method-led, not box-led",
         "We start with the molecule, the container, and the failure mode. The instrument is the last decision, not the first."),
        ("Founder-accountable",
         "Brian signs the study report. There is no account manager between the client and the scientist."),
        ("Built to be inspected",
         "Every CS Analytical method is filed with FDA-readiness as the design goal — not a retrofit."),
    ]
    add_hairline(s, Inches(0.75), Inches(3.55), Inches(11.83), color=RULE)
    y = Inches(3.7)
    for title, body in items:
        add_text(s, Inches(0.75), y, Inches(3.5), Inches(0.5),
                 title, size=16, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(4.4), y - Emu(20000), Inches(8.2), Inches(0.9),
                 body, size=13, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)
        y += Inches(0.82)


def slide_growth_vectors(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Growth vectors")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Where we want to go",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Five vectors. Sequenced, not\nstacked.",
             size=32, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.6), Inches(11.83), color=RULE)
    vectors = [
        ("01", "Sterile-injectable CCIT", "The core. Defend, deepen, raise price."),
        ("02", "Cell & gene cryo containers", "New methods, new formats. First mover."),
        ("03", "Pre-filled syringe / autoinjector", "GLP-1 wave demands repeatable HVLD."),
        ("04", "Combination products & device-led", "FDA cross-center — few labs are ready."),
        ("05", "Regulatory consulting carve-out", "Sell the brain, not just the bench."),
    ]
    y = Inches(3.85)
    for num, title, body in vectors:
        add_text(s, Inches(0.75), y, Inches(0.9), Inches(0.5),
                 num, size=26, color=BURGUNDY, font=FONT_HEAD, italic=True)
        add_text(s, Inches(1.7), y - Emu(50000), Inches(4.5), Inches(0.5),
                 title, size=17, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(6.3), y - Emu(30000), Inches(6.3), Inches(0.5),
                 body, size=13, color=CHARCOAL, font=FONT_HEAD, italic=True,
                 line_spacing=1.35)
        y += Inches(0.55)


def slide_tsi_citadel(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The agentic layer")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Where TSI Citadel multiplies us",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "We are a 48-person lab.\nAgentic AI gives us a 480-person reach.",
             size=28, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
    uses = [
        ("Market radar",
         "Continuous scan of FDA filings,\nclinical trials, conference rosters,\ncontainer-vendor catalogs — turned\ninto a weekly BD shortlist."),
        ("Regulatory intelligence",
         "Track USP, EP, JP, PIC/S changes\nin real time. Flag the ones that\nbreak a client's filed method."),
        ("BD operations",
         "Outbound, qualification, follow-up,\nmeeting prep. The work that\nadds no science but consumes\nfounder hours."),
        ("Scientific literature",
         "Daily distillation of CCIT, HVLD,\nhelium-leak, and container-vendor\nliterature. Methods updates surfaced\nbefore the customer asks."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.95)
    for title, body in uses:
        add_text(s, x, Inches(3.95), col_w - Inches(0.15), Inches(0.5),
                 title, size=16, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.4), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.55), col_w - Inches(0.15), Inches(2.5),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)
        x += col_w


def slide_90_day(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Ninety days")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "What we'd do first",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Three workstreams, ninety days,\none readout.",
             size=32, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
    streams = [
        ("DAYS 1—30",
         "Stand up the agentic stack",
         "TSI Citadel agents wired to FDA,\nClinicalTrials.gov, USP, EP, PIC/S,\nconference rosters, container-vendor\nrelease feeds. Weekly BD digest live."),
        ("DAYS 31—60",
         "Pilot two growth vectors",
         "Cryo-container method development and\nautoinjector HVLD. Two named pilot\nclients each. Methods drafted, quotes\nout, first revenue booked."),
        ("DAYS 61—90",
         "Package and price",
         "Productized CCIT-as-a-service tier.\nRegulatory-consulting carve-out\npriced. Public case studies (two)\nwith client co-sign."),
    ]
    x = Inches(0.75)
    col_w = Inches(3.95)
    for label, title, body in streams:
        add_text(s, x, Inches(3.9), col_w - Inches(0.15), Inches(0.35),
                 label, size=11, color=BURGUNDY, italic=True, font=FONT_BODY)
        add_text(s, x, Inches(4.3), col_w - Inches(0.15), Inches(0.6),
                 title, size=18, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.95), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(5.1), col_w - Inches(0.15), Inches(2.2),
                 body, size=13, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)
        x += col_w


def slide_ask(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The ask")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "What we'd love your help on",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "An introduction. A read. A reaction.",
             size=34, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.55), Inches(11.83), color=RULE)
    asks = [
        ("Introduction",
         "A warm hand-off to the TSI Citadel team — at the\nlevel where the agentic-platform partnership\nconversation can actually begin."),
        ("A read",
         "Twenty minutes of your eyes on the long-form\nmarket brief. Tell us where the thesis is thin and\nwhere it's right."),
        ("A reaction",
         "If you see a client, a partner, or a hire we should\nbe in front of — say the name. We'll do the rest."),
    ]
    y = Inches(3.8)
    for title, body in asks:
        add_text(s, Inches(0.75), y, Inches(3.3), Inches(0.5),
                 title, size=20, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(4.4), y - Emu(20000), Inches(8.2), Inches(1.0),
                 body, size=14, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)
        y += Inches(1.0)


def slide_closing(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    add_rect(s, 0, 0, SLIDE_W, SLIDE_H, NAVY)
    add_rect(s, 0, 0, Inches(0.08), SLIDE_H, BURGUNDY)

    add_text(s, Inches(1.1), Inches(1.4), Inches(11), Inches(0.5),
             "THANK YOU", size=14, color=GOLD, font=FONT_BODY, italic=True)
    add_hairline(s, Inches(1.1), Inches(1.9), Inches(2.5), color=BURGUNDY, weight=1.5)

    add_text(s, Inches(1.1), Inches(2.2), Inches(11), Inches(2.5),
             "The right work,\nin the right container,\nat the right cadence.",
             size=52, color=IVORY, font=FONT_HEAD, line_spacing=1.05)

    add_text(s, Inches(1.1), Inches(5.5), Inches(11), Inches(0.4),
             "Brian Mulhall", size=20, color=IVORY, font=FONT_HEAD)
    add_text(s, Inches(1.1), Inches(5.9), Inches(11), Inches(0.4),
             "Chief Executive Officer  ·  CS Analytical Laboratory",
             size=13, color=GOLD, italic=True, font=FONT_BODY)
    add_text(s, Inches(1.1), Inches(6.3), Inches(11), Inches(0.4),
             "brian.mulhall@csanalytical.com", size=13, color=IVORY, font=FONT_BODY)


# ---------------------------------------------------------------------------
# Charts
# ---------------------------------------------------------------------------

def build_market_chart(out: Path):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    import numpy as np

    years = np.array([2024, 2025, 2026, 2027, 2028, 2029, 2030])
    overall = np.array([6.4, 7.0, 7.6, 8.3, 9.0, 9.8, 10.7])
    ccit = np.array([0.28, 0.31, 0.34, 0.37, 0.41, 0.45, 0.48])

    fig, ax = plt.subplots(figsize=(8.2, 4.4), dpi=200)
    fig.patch.set_facecolor("#FAF7F2")
    ax.set_facecolor("#FAF7F2")

    ax.plot(years, overall, color="#0B2545", linewidth=2.4, marker="o",
            markersize=5, label="Pharma analytical testing  ($B)")
    ax.plot(years, ccit * 10, color="#6E1B26", linewidth=2.4, marker="o",
            markersize=5, label="CCIT carve-out  ($B, ×10 scale)")

    for sp in ("top", "right"):
        ax.spines[sp].set_visible(False)
    for sp in ("left", "bottom"):
        ax.spines[sp].set_color("#C9BEA8")

    ax.set_xticks(years)
    ax.tick_params(colors="#555B66", labelsize=9)
    ax.grid(axis="y", color="#E6DFCF", linewidth=0.6)
    ax.set_axisbelow(True)

    ax.legend(loc="upper left", frameon=False, fontsize=9, labelcolor="#1C1C1C")
    ax.set_title("Pharma analytical testing  vs.  CCIT carve-out, 2024 — 2030",
                 fontsize=12, color="#0B2545", loc="left", pad=14,
                 family="serif", weight="regular")

    for x, y in zip(years, overall):
        ax.annotate(f"${y:.1f}B", (x, y), textcoords="offset points",
                    xytext=(0, 8), ha="center", fontsize=8, color="#0B2545")
    for x, y in zip(years, ccit):
        ax.annotate(f"${y*1000:.0f}M", (x, y * 10), textcoords="offset points",
                    xytext=(0, -14), ha="center", fontsize=8, color="#6E1B26")

    plt.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


def build_competitive_map(out: Path):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    # (x = CCIT depth, y = speed/modernity, size = revenue scale, label)
    players = [
        ("Eurofins",          2.3, 4.8, 1700),
        ("SGS",               2.7, 4.5, 1400),
        ("Charles River",     3.0, 5.0, 1200),
        ("Pace Analytical",   2.0, 4.0, 700),
        ("Element",           1.8, 4.2, 800),
        ("Nelson Labs",       6.5, 5.4, 450),
        ("Whitehouse Labs",   7.5, 5.5, 250),
        ("Boston Analytical", 6.0, 4.8, 180),
        ("West Pharma Labs",  6.8, 5.0, 320),
        ("Almac",             4.5, 5.3, 600),
        ("CS Analytical",     8.6, 8.3, 220),
    ]

    fig, ax = plt.subplots(figsize=(8.4, 5.6), dpi=200)
    fig.patch.set_facecolor("#FAF7F2")
    ax.set_facecolor("#FAF7F2")

    for name, x, y, r in players:
        is_us = name == "CS Analytical"
        color = "#6E1B26" if is_us else "#0B2545"
        alpha = 1.0 if is_us else 0.45
        ax.scatter([x], [y], s=r * 0.55, color=color, alpha=alpha,
                   edgecolor="#FAF7F2", linewidth=1.2)
        dx, dy = 0.15, 0.15
        if name == "CS Analytical":
            dx, dy = 0.2, 0.25
        ax.annotate(name, (x, y), xytext=(x + dx, y + dy),
                    fontsize=9 if not is_us else 11,
                    color="#1C1C1C" if not is_us else "#6E1B26",
                    family="serif",
                    weight="regular" if not is_us else "bold")

    ax.set_xlim(0, 10)
    ax.set_ylim(2, 10)
    ax.set_xlabel("Depth in CCIT  →", fontsize=10, color="#555B66",
                  family="serif")
    ax.set_ylabel("Speed, modernity, agentic readiness  →",
                  fontsize=10, color="#555B66", family="serif")

    # Quadrant rules
    ax.axhline(6.5, color="#C9BEA8", linewidth=0.7, linestyle="--")
    ax.axvline(5.5, color="#C9BEA8", linewidth=0.7, linestyle="--")

    for sp in ("top", "right"):
        ax.spines[sp].set_visible(False)
    for sp in ("left", "bottom"):
        ax.spines[sp].set_color("#C9BEA8")

    ax.tick_params(colors="#555B66", labelsize=8)

    ax.text(7.8, 9.6, "Deep  ·  Modern", color="#6E1B26",
            fontsize=10, family="serif", style="italic")
    ax.text(0.3, 9.6, "Broad  ·  Modern", color="#555B66",
            fontsize=9, family="serif", style="italic")
    ax.text(0.3, 2.4, "Broad  ·  Legacy", color="#555B66",
            fontsize=9, family="serif", style="italic")
    ax.text(7.8, 2.4, "Deep  ·  Legacy", color="#555B66",
            fontsize=9, family="serif", style="italic")

    ax.set_title("CCIT competitive map — depth vs. agentic readiness",
                 fontsize=12, color="#0B2545", loc="left", pad=12,
                 family="serif")

    plt.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    prs = Presentation()
    prs.slide_width = SLIDE_W
    prs.slide_height = SLIDE_H

    build_dir = ROOT / "_build"
    market_chart = build_dir / "chart_market.png"
    comp_chart = build_dir / "chart_competitive.png"
    build_market_chart(market_chart)
    build_competitive_map(comp_chart)

    total = 12

    slide_cover(prs)
    slide_thesis(prs, 2, total)
    slide_operator(prs, 3, total)
    slide_company(prs, 4, total)
    slide_market_size(prs, 5, total, market_chart)
    slide_tailwinds(prs, 6, total)
    slide_competitive(prs, 7, total, comp_chart)
    slide_edge(prs, 8, total)
    slide_growth_vectors(prs, 9, total)
    slide_tsi_citadel(prs, 10, total)
    slide_90_day(prs, 11, total)
    slide_ask(prs, 12, total)
    slide_closing(prs, 12, total)  # closing page

    OUT_PPTX.parent.mkdir(parents=True, exist_ok=True)
    prs.save(OUT_PPTX)
    print(f"wrote {OUT_PPTX}")


if __name__ == "__main__":
    main()
