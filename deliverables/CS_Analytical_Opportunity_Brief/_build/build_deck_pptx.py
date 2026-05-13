"""Build the CS Analytical opportunity-brief deck as an editable PowerPoint.

Author voice: Bruce Longley · TSI-Citadel.
Audience: Alan Weiss, CFO, CS Analytical Laboratory.
Subject: CS Analytical's market position, organic growth paths, and the
specific places where TSI-Citadel's agentic platform would add measurable
P&L lift.

Brian Mulhall's name does not appear anywhere — he is the CEO of the
subject company; this is a third-party analyst document.
"""

from __future__ import annotations

from pathlib import Path

from pptx import Presentation
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.util import Emu, Inches, Pt

ROOT = Path(__file__).resolve().parent.parent
OUT_PPTX = ROOT / "01_Deck_PowerPoint" / "CS_Analytical_Opportunity_Brief.pptx"

FONT_HEAD = "Adobe Caslon Pro"
FONT_BODY = "Adobe Caslon Pro"

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
    add_rect(slide, 0, 0, SLIDE_W, SLIDE_H, IVORY)
    add_hairline(slide, Inches(0.75), Inches(0.55), Inches(11.83),
                 color=RULE, weight=0.75)
    add_text(slide, Inches(0.75), Inches(0.28), Inches(8), Inches(0.3),
             section.upper(), size=10, color=SLATE, italic=True, font=FONT_BODY)
    add_text(slide, Inches(8.0), Inches(0.28), Inches(4.83), Inches(0.3),
             "TSI-CITADEL  ·  OPPORTUNITY BRIEF",
             size=10, color=SLATE, italic=True, align=PP_ALIGN.RIGHT, font=FONT_BODY)
    add_hairline(slide, Inches(0.75), Inches(7.05), Inches(11.83),
                 color=RULE, weight=0.5)
    add_text(slide, Inches(0.75), Inches(7.12), Inches(8), Inches(0.3),
             "Prepared for Alan Weiss  ·  CFO, CS Analytical Laboratory  ·  Confidential",
             size=9, color=SLATE, italic=True, font=FONT_BODY)
    add_text(slide, Inches(8.0), Inches(7.12), Inches(4.83), Inches(0.3),
             f"{page_no:02d} / {total:02d}", size=9, color=SLATE,
             align=PP_ALIGN.RIGHT, font=FONT_BODY)


# ---------------------------------------------------------------------------
# Slides
# ---------------------------------------------------------------------------

def slide_cover(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    add_rect(s, 0, 0, SLIDE_W, SLIDE_H, IVORY)
    add_rect(s, 0, 0, Inches(0.6), SLIDE_H, NAVY)
    add_rect(s, Inches(0.6), 0, Inches(0.08), SLIDE_H, BURGUNDY)

    add_text(s, Inches(1.1), Inches(0.85), Inches(11), Inches(0.4),
             "CS ANALYTICAL LABORATORY", size=14, color=SLATE,
             italic=True, font=FONT_BODY)
    add_hairline(s, Inches(1.1), Inches(1.35), Inches(3.5),
                 color=BURGUNDY, weight=1.5)

    add_text(s, Inches(1.1), Inches(1.7), Inches(11), Inches(2.8),
             "An opportunity brief.\nMarket, growth paths,\nand the agentic-AI question.",
             size=50, color=NAVY, font=FONT_HEAD, line_spacing=1.05)

    add_hairline(s, Inches(1.1), Inches(5.55), Inches(2.2),
                 color=NAVY, weight=1.0)
    add_text(s, Inches(1.1), Inches(5.7), Inches(11), Inches(0.4),
             "Prepared by  Bruce Longley  ·  TSI-Citadel",
             size=18, color=CHARCOAL, font=FONT_HEAD)
    add_text(s, Inches(1.1), Inches(6.1), Inches(11), Inches(0.4),
             "For  Alan Weiss  ·  Chief Financial Officer  ·  CS Analytical Laboratory",
             size=13, color=SLATE, italic=True, font=FONT_BODY)
    add_text(s, Inches(1.1), Inches(6.45), Inches(11), Inches(0.4),
             "May 2026  ·  bruce@tsicitadel.ai",
             size=11, color=SLATE, font=FONT_BODY)


def slide_why(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Why this brief")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Why we're sending this", size=14, color=BURGUNDY, italic=True,
             font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(3.0),
             "CS Analytical has been on our\nwatchlist for two years.\nThe last six months made the\nwatchlist a working file.",
             size=44, color=NAVY, font=FONT_HEAD, line_spacing=1.08)

    add_hairline(s, Inches(0.75), Inches(5.85), Inches(2.5),
                 color=BURGUNDY, weight=1.25)
    add_text(s, Inches(0.75), Inches(6.05), Inches(11.83), Inches(1.0),
             "The Clifton expansion, RM Analytical, USP <382>, and the GLP-1\nwave all landed in the same quarter. That is the signal.",
             size=16, color=CHARCOAL, italic=True, font=FONT_HEAD,
             line_spacing=1.35)


def slide_company(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The company")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "CS Analytical at a glance — outside view",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "The only cGMP-built,\nFDA-regulated lab in the world\ndedicated to container testing.",
             size=30, color=NAVY, font=FONT_HEAD, line_spacing=1.08)

    add_hairline(s, Inches(0.75), Inches(4.55), Inches(2.5),
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
    x = Inches(0.75)
    col_w = Inches(2.95)
    for title, body in pillars:
        add_rect(s, x, Inches(4.8), col_w - Inches(0.15), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.95), col_w - Inches(0.15), Inches(0.5),
                 title, size=16, color=NAVY, font=FONT_HEAD)
        add_text(s, x, Inches(5.5), col_w - Inches(0.15), Inches(1.6),
                 body, size=11.5, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


def slide_market(prs, page, total, chart_path: Path):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The market")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "What sits in front of you",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "A $9.5 B host market —\nand a $1.5 B niche moving with it.",
             size=28, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    s.shapes.add_picture(str(chart_path), Inches(0.75), Inches(3.4),
                         width=Inches(7.6))

    add_hairline(s, Inches(8.7), Inches(3.6), Inches(0.6), color=BURGUNDY)
    add_text(s, Inches(8.7), Inches(3.75), Inches(4.0), Inches(3.0),
             "Pharma analytical testing\noutsourcing:  $9.5 B (2025),\n~9% CAGR through 2030.\n\nCCIT services carve-out:\n$1.5 B (2025), ~9-10% CAGR.\n$2.4 B by 2030.\n\nNo single CCIT-pure provider\nholds more than mid-single\ndigit share. The room is open.",
             size=12, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)

    add_text(s, Inches(0.75), Inches(6.75), Inches(11.83), Inches(0.3),
             "Sources (composite midpoints, 2026): Precedence Research, Market Research Future, Data Horizzon, Roots Analysis.",
             size=8, color=SLATE, italic=True, font=FONT_BODY)


def slide_tailwinds(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Tailwinds")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Four forces, one quarter",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Each of these alone would justify\na faster plan. All four landed at once.",
             size=24, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.85), Inches(11.83), color=RULE)
    forces = [
        ("USP <382>",
         "Effective Dec 1, 2025.\nSystem-level functional\nsuitability testing of\nelastomeric components.\nResponsibility shifts to\nthe drug manufacturer."),
        ("Annex 1 convergence",
         "EU GMP Annex 1 now\nmandates validated\ndeterministic CCIT.\nVisual inspection alone\nis no longer acceptable.\nGlobal alignment is here."),
        ("GLP-1 / PFS wave",
         "Prefilled syringe market\n$9.7 B (2025) → $18.1 B\n(2031). GLP-1 autoinjectors\nat 15.6% CAGR. Shift to\nCOP barrels. Volumes\npunish defect rates."),
        ("Biologics share",
         "Biologics now ~40% of\nFDA novel approvals.\nEvery one ships in a\nsterile primary container.\nGlass-delamination recalls\nare in the news again."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.95)
    for title, body in forces:
        add_text(s, x, Inches(4.05), col_w - Inches(0.15), Inches(0.5),
                 title, size=16, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.55), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.7), col_w - Inches(0.15), Inches(2.4),
                 body, size=11, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


def slide_competitive(prs, page, total, chart_path: Path):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The competitive field")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Who else is in the room",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Generalists are broad. Specialists\nare few. The corner is open.",
             size=26, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    s.shapes.add_picture(str(chart_path), Inches(0.75), Inches(3.0),
                         width=Inches(7.8))

    add_hairline(s, Inches(8.8), Inches(3.2), Inches(0.6), color=BURGUNDY)
    add_text(s, Inches(8.8), Inches(3.35), Inches(4.0), Inches(3.6),
             "Eurofins, SGS, Charles River,\nWuXi, Pace, Intertek, Element\n— broad, deep pockets, shallow\non CCIT specifically.\n\nNelson Labs, Boston Analytical,\nWest Pharma Labs — credible\nspecialists, slower to modernize.\n\nThe deep-and-modern corner is\nstructurally empty. CS Analytical\nis the credible occupant.",
             size=11.5, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)


def slide_in_motion(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Already in motion")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "What CS Analytical is already doing right",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "This is not a turnaround brief.\nIt is an acceleration brief.",
             size=26, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.85), Inches(11.83), color=RULE)
    moves = [
        ("RM Analytical launch",
         "Feb 2026.  Raw material and\nexcipient testing under a sister\nbrand. Genuine adjacent revenue."),
        ("Clifton expansion",
         "May 2026.  Doubling the lab\nfootprint. Capacity is being built\nahead of demand, not behind it."),
        ("Micro & gas testing",
         "USP <87> biological reactivity,\nUSP <788> particle size, USP/EP\ngas. Cross-sell into existing book."),
        ("USP <382> readiness",
         "Already running elastomer\nfunctionality. The Dec 2025 wave\nwill find capacity here, not\nelsewhere."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.95)
    for title, body in moves:
        add_text(s, x, Inches(4.05), col_w - Inches(0.15), Inches(0.5),
                 title, size=15, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.55), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.7), col_w - Inches(0.15), Inches(2.4),
                 body, size=11, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


def slide_organic_growth(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Organic growth — no AI required")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Five paths that don't require an AI partner",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Each is buildable on the\nexisting team and footprint.",
             size=26, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
    vectors = [
        ("01", "USP <382> wave capture",
         "Be the named lab for the post-Dec 2025 elastomer functional-suitability rush."),
        ("02", "GLP-1 / PFS HVLD productization",
         "Productized high-voltage leak detection for COP-barrel autoinjectors. Repeatable, high-margin."),
        ("03", "Annex 1 European mandate",
         "EU GMP Annex 1 deterministic CCIT requirement opens European sponsors. Reciprocal-recognition path."),
        ("04", "Cell, gene & cryo container methods",
         "New container formats with no incumbent method. Highest margin per study in the market."),
        ("05", "Regulatory consulting carve-out",
         "Productize the advisory work the team gives away today. Higher margin than the bench."),
    ]
    y = Inches(3.95)
    for num, title, body in vectors:
        add_text(s, Inches(0.75), y, Inches(0.9), Inches(0.5),
                 num, size=24, color=BURGUNDY, italic=True, font=FONT_HEAD)
        add_text(s, Inches(1.7), y + Emu(40000), Inches(5.0), Inches(0.5),
                 title, size=15, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(6.8), y + Emu(60000), Inches(6.0), Inches(0.5),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD, italic=True,
                 line_spacing=1.35)
        y += Inches(0.55)


def slide_agentic(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Agentic AI — TSI-Citadel angle")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Where TSI-Citadel multiplies what's here",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "The healthcare agentic-AI category\nis tracking 9× growth, 2024 → 2030.",
             size=22, color=NAVY, font=FONT_HEAD, line_spacing=1.1)
    add_text(s, Inches(0.75), Inches(3.0), Inches(11.83), Inches(0.5),
             "These are the five we'd build first.",
             size=14, color=SLATE, italic=True, font=FONT_BODY)

    add_hairline(s, Inches(0.75), Inches(3.6), Inches(11.83), color=RULE)
    uses = [
        ("Regulatory intel",
         "Continuous monitoring of USP,\nEP, JP, PIC/S, FDA 483s, warning\nletters. Resolves changes against\nclient filed methods. The single\nhighest-value agent post-<382>."),
        ("Market radar",
         "FDA filings, ClinicalTrials.gov,\nconference rosters, container-\nvendor releases. Daily ingest,\nweekly BD shortlist with\nevidence linked."),
        ("BD operations",
         "Outbound drafting, qualification,\nfollow-up, meeting prep, post-call\nsummary. Returns founder and\nscientist time."),
        ("Scientific surveillance",
         "Daily distillation of CCIT, HVLD,\nleak-detection, vendor white-\npaper literature. Methods surface\nbefore the client asks."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.95)
    for title, body in uses:
        add_text(s, x, Inches(3.85), col_w - Inches(0.15), Inches(0.5),
                 title, size=15, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.35), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.5), col_w - Inches(0.15), Inches(2.5),
                 body, size=11, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.4)
        x += col_w


def slide_roi(prs, page, total, chart_path: Path):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="ROI shape — for the CFO")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Three readings of the same 36 months",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.0),
             "Status quo. Organic. Organic + agentic.",
             size=24, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    s.shapes.add_picture(str(chart_path), Inches(0.75), Inches(3.0),
                         width=Inches(7.6))

    add_hairline(s, Inches(8.6), Inches(3.0), Inches(0.6), color=BURGUNDY)
    add_text(s, Inches(8.6), Inches(3.15), Inches(4.2), Inches(0.5),
             "PAYBACK ON THE AGENTIC LAYER",
             size=11, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(8.6), Inches(3.55), Inches(4.2), Inches(3.0),
             "Conservative case:  9-12 months.\nCarried by founder-calendar\nrecovery and quote-cycle reduction\nalone.\n\nAggressive case:  4-6 months.\nDriven by net-new BD opportunities\nthe radar surfaces and the\nproposal agent closes.\n\nCapital required: low six figures.\nNot a transformation. A bolt-on.",
             size=11, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.4)


def slide_adjacent(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Adjacent opportunities")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Where else this team could play",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Adjacencies, not pivots.\nOptional, not required.",
             size=26, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
    items = [
        ("Combination products",
         "Cross-center (CDER + CDRH) submissions. Few incumbent labs are\nready. Margin profile is favorable."),
        ("Device sterilization validation",
         "ISO 11135 / 11137 adjacency to existing container work. Natural cross-\nsell into the same sponsor base."),
        ("Compounding pharmacy testing",
         "USP <797>, <800>. Different regulatory floor; same testing physics.\nFragmented buyer base, less competition."),
        ("Veterinary biologics",
         "Same containers, smaller competitive set, USDA channel.\nUnderpriced relative to human-side work."),
        ("Industry training & certification",
         "The team's reputation is a productizable asset. Course revenue\nis high margin and is itself a BD funnel."),
    ]
    y = Inches(3.9)
    for title, body in items:
        add_text(s, Inches(0.75), y, Inches(3.5), Inches(0.5),
                 title, size=14, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(4.6), y, Inches(8.2), Inches(0.6),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.35)
        y += Inches(0.55)


def slide_risks(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Risks — named, not glossed")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "What we'd want to disagree about",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Three CFO-relevant risks,\nthree handlings.",
             size=28, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
    risks = [
        ("Capital timing",
         "Lab expansion + RMA build + agentic spend in one fiscal year.\nHandling: sequence the agentic spend behind the lab build;\nagentic stack pays for itself in <12 months."),
        ("Margin dilution from adjacencies",
         "Raw-material and micro work carries lower gross margin than\nCCIT. Handling: hold the CCIT premium pricing while RMA scales;\nuse agentic intel to keep CCIT pipeline full."),
        ("Agentic-AI hallucination in regulated context",
         "An agent that flags a false USP change can damage a sponsor\nrelationship. Handling: human-in-the-loop on every flag that\ntouches a client filed method. Suggested, not authoritative."),
    ]
    y = Inches(3.9)
    for title, body in risks:
        add_text(s, Inches(0.75), y, Inches(3.7), Inches(0.5),
                 title, size=14, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(4.8), y, Inches(8.0), Inches(1.0),
                 body, size=11.5, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        y += Inches(1.0)


def slide_working_together(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="If you wanted to work with us")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "How TSI-Citadel would engage",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Light footprint. Boxed scope.\nMeasurable from day one.",
             size=26, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.85), Inches(11.83), color=RULE)
    steps = [
        ("DAYS 1 — 30",
         "Diagnostic",
         "Sit with the BD, regulatory,\nand operations leads. Map\nthe data feeds, the cycles,\nand the moments. No code\nyet. A written read-out and\na go / no-go at day 30."),
        ("DAYS 31 — 60",
         "Two agents live",
         "Regulatory-intel and market-\nradar agents, wired to real\nfeeds. Daily output to a\nnamed internal user. First\nmeasurable lift inside the\nperiod."),
        ("DAYS 61 — 90",
         "Decide",
         "Joint readout. Numbers. The\nthree remaining agents are\nbuilt only if the first two have\nearned them. The CFO\nholds the gate, by design."),
    ]
    x = Inches(0.75)
    col_w = Inches(3.95)
    for label, title, body in steps:
        add_text(s, x, Inches(4.05), col_w - Inches(0.15), Inches(0.35),
                 label, size=11, color=BURGUNDY, italic=True, font=FONT_BODY)
        add_text(s, x, Inches(4.45), col_w - Inches(0.15), Inches(0.6),
                 title, size=17, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(5.1), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(5.25), col_w - Inches(0.15), Inches(2.0),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


def slide_closing(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    add_rect(s, 0, 0, SLIDE_W, SLIDE_H, NAVY)
    add_rect(s, 0, 0, Inches(0.08), SLIDE_H, BURGUNDY)

    add_text(s, Inches(1.1), Inches(1.4), Inches(11), Inches(0.5),
             "THE NEXT CONVERSATION", size=14, color=GOLD, italic=True,
             font=FONT_BODY)
    add_hairline(s, Inches(1.1), Inches(1.9), Inches(2.5),
                 color=BURGUNDY, weight=1.5)

    add_text(s, Inches(1.1), Inches(2.25), Inches(11), Inches(2.5),
             "Read the long form\non a plane.\nThen tell us where\nwe're wrong.",
             size=44, color=IVORY, font=FONT_HEAD, line_spacing=1.08)

    add_text(s, Inches(1.1), Inches(5.55), Inches(11), Inches(0.4),
             "Bruce Longley", size=20, color=IVORY, font=FONT_HEAD)
    add_text(s, Inches(1.1), Inches(5.95), Inches(11), Inches(0.4),
             "TSI-Citadel", size=13, color=GOLD, italic=True, font=FONT_BODY)
    add_text(s, Inches(1.1), Inches(6.3), Inches(11), Inches(0.4),
             "bruce@tsicitadel.ai", size=13, color=IVORY, font=FONT_BODY)


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
    roi_chart = build_dir / "chart_roi.png"

    total = 13

    slide_cover(prs)
    slide_why(prs, 2, total)
    slide_company(prs, 3, total)
    slide_market(prs, 4, total, market_chart)
    slide_tailwinds(prs, 5, total)
    slide_competitive(prs, 6, total, comp_chart)
    slide_in_motion(prs, 7, total)
    slide_organic_growth(prs, 8, total)
    slide_agentic(prs, 9, total)
    slide_roi(prs, 10, total, roi_chart)
    slide_adjacent(prs, 11, total)
    slide_risks(prs, 12, total)
    slide_working_together(prs, 13, total)
    slide_closing(prs)

    OUT_PPTX.parent.mkdir(parents=True, exist_ok=True)
    prs.save(OUT_PPTX)
    print(f"wrote {OUT_PPTX}")


if __name__ == "__main__":
    main()
