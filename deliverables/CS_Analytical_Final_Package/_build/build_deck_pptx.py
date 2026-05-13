"""Final-package deck — editable PowerPoint.

Voice: Bruce Longley · TSI-Citadel.
Reader: Alan Weiss, CFO, CS Analytical Laboratory.

Slide order (17 numbered + closing):
   1. Cover
   2. The thesis — three shifts compounded
   3. CS Analytical at a glance (with leadership)
   4. The market (chart)
   5. Tailwinds — five forces
   6. The container-format wave (chart) — NEW
   7. The regulator has already adopted — NEW
   8. Competitive map (chart)
   9. Already in motion
  10. Organic growth — five paths
  11. Agentic AI — TSI-Citadel angle
  12. This is not experimental (chart) — NEW
  13. ROI shape for the CFO (chart) + Strategic optionality
  14. The Gibraltar precedent — NEW
  15. Adjacent opportunities
  16. Risks
  17. If you wanted to work with us
  Closing
"""

from __future__ import annotations

from pathlib import Path

from pptx import Presentation
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE
from pptx.enum.text import PP_ALIGN, MSO_ANCHOR
from pptx.util import Emu, Inches, Pt

ROOT = Path(__file__).resolve().parent.parent
OUT_PPTX = ROOT / "01_Deck" / "CS_Analytical_Opportunity_Brief.pptx"

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
TOTAL = 17


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


def page_chrome(slide, *, page_no, section):
    add_rect(slide, 0, 0, SLIDE_W, SLIDE_H, IVORY)
    add_hairline(slide, Inches(0.75), Inches(0.55), Inches(11.83),
                 color=RULE, weight=0.75)
    add_text(slide, Inches(0.75), Inches(0.28), Inches(8), Inches(0.3),
             section.upper(), size=10, color=SLATE, italic=True, font=FONT_BODY)
    add_text(slide, Inches(8.0), Inches(0.28), Inches(4.83), Inches(0.3),
             "TSI-CITADEL  ·  OPPORTUNITY BRIEF",
             size=10, color=SLATE, italic=True, align=PP_ALIGN.RIGHT,
             font=FONT_BODY)
    add_hairline(slide, Inches(0.75), Inches(7.05), Inches(11.83),
                 color=RULE, weight=0.5)
    add_text(slide, Inches(0.75), Inches(7.12), Inches(8), Inches(0.3),
             "Prepared for Alan Weiss  ·  CFO, CS Analytical Laboratory  ·  Confidential",
             size=9, color=SLATE, italic=True, font=FONT_BODY)
    add_text(slide, Inches(8.0), Inches(7.12), Inches(4.83), Inches(0.3),
             f"{page_no:02d} / {TOTAL:02d}", size=9, color=SLATE,
             align=PP_ALIGN.RIGHT, font=FONT_BODY)


# ---------------------------------------------------------------------------
# Slide 1 — Cover
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


# ---------------------------------------------------------------------------
# Slide 2 — Thesis: three shifts compounded (NEW — combined R2-22/23 framing)
# ---------------------------------------------------------------------------

def slide_thesis(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=2, section="The thesis")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Why we're sending this", size=14, color=BURGUNDY,
             italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.6),
             "Three industry shifts compounded\nin one six-month window.",
             size=38, color=NAVY, font=FONT_HEAD, line_spacing=1.08)

    add_hairline(s, Inches(0.75), Inches(4.05), Inches(11.83), color=RULE)

    shifts = [
        ("USP <382>  ·  Dec 1 2025",
         "Elastomer functional suitability\nshifts to the drug manufacturer.\nUSP: 'early adoption is encouraged.'"),
        ("FDA agentic AI  ·  Dec 1 2025",
         "All FDA employees — including\ninspectors — receive secure\nagentic AI. Regulator is no longer\nagentically naive."),
        ("Nelson Labs publicly acquisitive  ·  May 2026",
         "Sotera Health Q1 2026: 'exploring\nstrategic acquisitions to enhance\nNelson Labs' pharmaceutical\ncapabilities.' NJ precedent exists."),
    ]
    x = Inches(0.75)
    col_w = Inches(3.95)
    for label, body in shifts:
        add_text(s, x, Inches(4.25), col_w - Inches(0.15), Inches(0.5),
                 label, size=14, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.78), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.95), col_w - Inches(0.15), Inches(2.0),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


# ---------------------------------------------------------------------------
# Slide 3 — Company at a glance + leadership (integrated R1-1, R1-3)
# ---------------------------------------------------------------------------

def slide_company(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=3, section="The company")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "CS Analytical at a glance — outside view",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.5),
             "The only cGMP-built,\nFDA-regulated lab in the world\ndedicated to container testing.",
             size=28, color=NAVY, font=FONT_HEAD, line_spacing=1.08)

    add_hairline(s, Inches(0.75), Inches(4.4), Inches(2.5),
                 color=BURGUNDY, weight=1.25)

    pillars = [
        ("Where & what",
         "Clifton, NJ. Doubling lab\nspace, May 2026. USP <1207>\ndeterministic portfolio plus\nUSP <382>, <87>, <788>,\ngas, micro."),
        ("Who — leadership",
         "Brian Mulhall, CEO. Alan Weiss,\nCFO, leading finance, BD, HR.\nBrandon Zurawlow, CSO —\nPDA TR-86 contributor.\nFounder-led, PhD-staffed."),
        ("Recent moves",
         "RM Analytical launch (Feb 2026).\nClifton doubling (May 2026).\nInterphex 2026 showcase: IV bag\nCCIT, <382>, cell-therapy\ndistribution testing."),
        ("Position",
         "Vendor-neutral. Method-led.\nFounder-accountable on every\nstudy. Inspected by design,\nnot retrofit."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.95)
    for title, body in pillars:
        add_rect(s, x, Inches(4.65), col_w - Inches(0.15), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.8), col_w - Inches(0.15), Inches(0.5),
                 title, size=14, color=NAVY, font=FONT_HEAD)
        add_text(s, x, Inches(5.35), col_w - Inches(0.15), Inches(1.7),
                 body, size=11, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


# ---------------------------------------------------------------------------
# Slide 4 — Market (carried)
# ---------------------------------------------------------------------------

def slide_market(prs, chart_path):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=4, section="The market")

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
             "Pharma analytical testing\noutsourcing:  $9.5 B (2025),\n~9% CAGR through 2030.\n\nCCIT services carve-out:\n$1.5 B (2025), ~9-10% CAGR.\n$2.4 B by 2030.\n\nBioPlan 2025: outsourcing\nbudgets surged 11%, U.S.\ncaptured 75% of intentions.\nTop-5 CDMOs hold only 15%.",
             size=11.5, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)

    add_text(s, Inches(0.75), Inches(6.75), Inches(11.83), Inches(0.3),
             "Sources: Precedence Research, Grand View, MarketsAndMarkets, BioPlan Associates 22nd Annual Report (Jul 2025). Composite midpoints.",
             size=8, color=SLATE, italic=True, font=FONT_BODY)


# ---------------------------------------------------------------------------
# Slide 5 — Tailwinds (UPDATED: 5 tiles with R1-5 GLP-1 + R2-5 USP <382>)
# ---------------------------------------------------------------------------

def slide_tailwinds(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=5, section="Tailwinds")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Five forces, one quarter",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Each alone would justify a faster plan.\nAll five landed in the same window.",
             size=22, color=NAVY, font=FONT_HEAD, line_spacing=1.12)

    add_hairline(s, Inches(0.75), Inches(3.85), Inches(11.83), color=RULE)

    forces = [
        ("USP <382>",
         "Dec 1, 2025. System-level\nelastomer testing. Burden\nshifts to drug manufacturer.\nUSP: 'early adoption\nencouraged.'"),
        ("Annex 1 convergence",
         "EU GMP Annex 1 requires\nvalidated deterministic CCIT.\nGrade A bioContamination\nlimit = zero. Lyo pre-batch\nsterilization enforced."),
        ("GLP-1 / PFS wave",
         "Prefilled syringes\n$9.7B → $18.1B by 2031.\nGLP-1 autoinjectors at\n15.6% CAGR. COP barrels\nadopted; methods reset."),
        ("503B sterility crisis",
         "GenoGenix Jul 2025 recall.\nProRx Oct 2025: 36,000+\nsemaglutide vials. 1,150\nadverse events filed YTD.\n503B is now CCIT-buying."),
        ("Biologics share",
         "~40% of FDA novel\napprovals. Every one in a\nsterile primary container.\nGlass-delamination recalls\nback in 2026 headlines."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.40)
    for title, body in forces:
        add_text(s, x, Inches(4.05), col_w - Inches(0.1), Inches(0.5),
                 title, size=13, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.55), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.7), col_w - Inches(0.1), Inches(2.4),
                 body, size=10.5, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


# ---------------------------------------------------------------------------
# Slide 6 — Container-format wave (NEW — Slide C)
# ---------------------------------------------------------------------------

def slide_container_wave(prs, chart_path):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=6, section="The container-format wave")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "New containers. New methods. No incumbent.",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.0),
             "Each new format below is a discrete\nCCIT method-development engagement.",
             size=22, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    # Place the timeline chart prominently
    s.shapes.add_picture(str(chart_path), Inches(0.75), Inches(3.15),
                         width=Inches(8.2))

    add_hairline(s, Inches(9.2), Inches(3.3), Inches(0.6), color=BURGUNDY)
    add_text(s, Inches(9.2), Inches(3.5), Inches(3.6), Inches(3.5),
             "None of the generalists\nhas these formats validated.\n\nCS Analytical's CSO has\npublicly presented on\ncryogenic CCIT — the only\nspecialty lab with the\ncredential to claim that\nground.\n\nEvery launch above is a\nbillable engagement waiting\nto be sized.",
             size=11, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.4)


# ---------------------------------------------------------------------------
# Slide 7 — The regulator has already adopted (NEW — Slide B)
# ---------------------------------------------------------------------------

def slide_regulator_adopted(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=7, section="Why now")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "December 1, 2025.  Two things became official.",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.0),
             "The regulator is no longer agentically naive.",
             size=30, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    # Two-panel layout
    # Top panel — USP <382> (navy)
    add_rect(s, Inches(0.75), Inches(3.2), Inches(11.83), Inches(1.85), NAVY)
    add_text(s, Inches(1.0), Inches(3.35), Inches(11.4), Inches(0.4),
             "USP <382>", size=14, color=GOLD, italic=True, font=FONT_BODY)
    add_text(s, Inches(1.0), Inches(3.75), Inches(11.4), Inches(1.2),
             "Elastomeric Component Functional Suitability — official.\nSystem-level testing. Responsibility shifts from elastomer\nsupplier to drug manufacturer. USP: 'early adoption is encouraged.'",
             size=14, color=IVORY, font=FONT_HEAD, line_spacing=1.35)

    # Bottom panel — FDA (ivory with burgundy left rule)
    add_rect(s, Inches(0.75), Inches(5.15), Inches(11.83), Inches(1.7), IVORY)
    add_rect(s, Inches(0.75), Inches(5.15), Inches(0.04), Inches(1.7), BURGUNDY)
    add_text(s, Inches(1.0), Inches(5.3), Inches(11.4), Inches(0.4),
             "The FDA — same day",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(1.0), Inches(5.7), Inches(11.4), Inches(1.0),
             "Agentic AI deployed to all FDA employees. Secure GovCloud, no\ntraining on industry submissions. Supports pre-market review,\npost-market surveillance, INSPECTIONS, and compliance.",
             size=13, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.4)


# ---------------------------------------------------------------------------
# Slide 8 — Competitive map (UPDATED with R1-15/R1-14/R2-13)
# ---------------------------------------------------------------------------

def slide_competitive(prs, chart_path):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=8, section="The competitive field")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Who else is in the room",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Generalists are broad.\nSpecialists are scaling. The corner is open.",
             size=24, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    s.shapes.add_picture(str(chart_path), Inches(0.75), Inches(3.0),
                         width=Inches(7.8))

    add_hairline(s, Inches(8.8), Inches(3.2), Inches(0.6), color=BURGUNDY)
    add_text(s, Inches(8.8), Inches(3.4), Inches(4.0), Inches(3.7),
             "Three competitive moves in\nthe last six months:\n\n  Nelson Labs doubling\n  cleanrooms and publicly\n  acquisitive (Sotera Q1).\n\n  West Pharma 165k sq ft\n  Dublin (Mar 2026) for\n  GLP-1 vertical integration.\n\n  Eurofins adding 157k m²\n  of lab space in 2025-2026.\n\nThe deep-and-modern\ncorner remains empty.",
             size=10.5, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)


# ---------------------------------------------------------------------------
# Slide 9 — Already in motion (UPDATED with R1-16)
# ---------------------------------------------------------------------------

def slide_in_motion(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=9, section="Already in motion")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "What CS Analytical is already doing right",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "This is not a turnaround brief.\nIt is an acceleration brief.",
             size=24, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.85), Inches(11.83), color=RULE)
    moves = [
        ("RM Analytical launch  ·  Feb 2026",
         "Raw material and excipient\ntesting under a sister brand.\nAdjacent revenue. Cross-sell\ninto existing base."),
        ("Clifton expansion  ·  May 2026",
         "Doubling the lab footprint.\nCapacity built ahead of\ndemand, not behind it."),
        ("Micro, gas, IV bag",
         "USP <87>, <788>, USP/EP gas,\nIV bag CCIT. Each is\ncross-sell into existing\nsponsor relationships."),
        ("Interphex 2026 showcase",
         "Public commitment to IV bag,\nUSP <382>, and cell-therapy\nISTA/ASTM distribution\ntesting. Ahead of competitors."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.95)
    for title, body in moves:
        add_text(s, x, Inches(4.05), col_w - Inches(0.15), Inches(0.6),
                 title, size=13, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.65), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.8), col_w - Inches(0.15), Inches(2.2),
                 body, size=11, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


# ---------------------------------------------------------------------------
# Slide 10 — Organic growth (5 paths)
# ---------------------------------------------------------------------------

def slide_organic(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=10, section="Organic growth — no AI required")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Five paths that don't require an AI partner",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Each is buildable on the\nexisting team and footprint.",
             size=22, color=NAVY, font=FONT_HEAD, line_spacing=1.1)
    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)

    vectors = [
        ("01", "USP <382> wave capture",
         "Named lab for the post-Dec 2025 elastomer rush."),
        ("02", "GLP-1 / PFS HVLD productization",
         "HVLD for COP-barrel autoinjectors. Repeatable, high-margin."),
        ("03", "Annex 1 European mandate",
         "EU GMP Annex 1 opens European sponsors. Reciprocal-recognition path."),
        ("04", "Cell, gene & cryo container methods",
         "No incumbent method. Highest margin per study. CSO credential."),
        ("05", "Regulatory consulting carve-out",
         "Productize the advisory work the team gives away today."),
    ]
    y = Inches(3.95)
    for num, title, body in vectors:
        add_text(s, Inches(0.75), y, Inches(0.9), Inches(0.5),
                 num, size=24, color=BURGUNDY, italic=True, font=FONT_HEAD)
        add_text(s, Inches(1.7), y + Emu(40000), Inches(5.0), Inches(0.5),
                 title, size=15, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(6.8), y + Emu(60000), Inches(6.0), Inches(0.5),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD,
                 italic=True, line_spacing=1.35)
        y += Inches(0.55)


# ---------------------------------------------------------------------------
# Slide 11 — Agentic AI angle (UPDATED with R2-20 McKinsey)
# ---------------------------------------------------------------------------

def slide_agentic(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=11, section="Agentic AI — TSI-Citadel angle")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Where TSI-Citadel multiplies what's already here",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "McKinsey: 35-45% productivity gain\nacross 270 pharma workflows.",
             size=22, color=NAVY, font=FONT_HEAD, line_spacing=1.1)
    add_text(s, Inches(0.75), Inches(3.0), Inches(11.83), Inches(0.5),
             "Five agents we would build first.",
             size=13, color=SLATE, italic=True, font=FONT_BODY)

    add_hairline(s, Inches(0.75), Inches(3.6), Inches(11.83), color=RULE)

    uses = [
        ("USP <382>\nwave-capture",
         "Monitors every 483 / WL\nfor elastomer findings.\nResolves to sponsor list.\nDrafts targeted outreach."),
        ("Container-\nformat radar",
         "Tracks SCHOTT, Stevanato,\nBD, West, Bonfiglioli\nlaunches. Sizes the\nmethod-dev opportunity."),
        ("Inspection-\nreadiness mirror",
         "Mirror to FDA's own\nagentic inspector. Predicts\nwhat AI-augmented\ninspectors will find."),
        ("CAR-T cryo\nmethod pipeline",
         "Phase II/III CAR-T radar.\nCryo container choices.\nFailure-rate economics\n(3.87% — 25%) drive demand."),
        ("Strategic-\noptionality tracker",
         "CFO-only quarterly view.\nComparable transactions,\nacquirer activity, market\nposition. Board-friendly."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.40)
    for title, body in uses:
        add_text(s, x, Inches(3.85), col_w - Inches(0.1), Inches(0.7),
                 title, size=13, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.6), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.75), col_w - Inches(0.1), Inches(2.3),
                 body, size=10.5, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


# ---------------------------------------------------------------------------
# Slide 12 — This is not experimental (NEW — Slide D + chart)
# ---------------------------------------------------------------------------

def slide_not_experimental(prs, chart_path):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=12, section="The agentic layer is already commercial")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Six top-25 pharmas.  One million cases.  95% accuracy.",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.0),
             "Agentic AI in pharma is no longer experimental.",
             size=24, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    s.shapes.add_picture(str(chart_path), Inches(0.75), Inches(2.9),
                         width=Inches(7.6))

    add_hairline(s, Inches(8.5), Inches(3.0), Inches(0.6), color=BURGUNDY)
    add_text(s, Inches(8.5), Inches(3.2), Inches(4.3), Inches(3.7),
             "ArisGlobal NavaX: 1M+ safety\ncases. 95%+ accuracy. 30%+\nefficiency gain. 120% Y/Y\nbookings Q1 2026.\n\nSixth top-25 pharma adopted\nby Jun 2025. Top-20 adopted\nSignals + Distribution Agents\nin 6 weeks from launch.\n\nFDA itself deployed agentic\nAI Dec 1, 2025. The question\nis no longer if. It is who,\nwhen, and at what tempo.",
             size=11, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.35)


# ---------------------------------------------------------------------------
# Slide 13 — ROI shape + calendar recovery + strategic optionality
# ---------------------------------------------------------------------------

def slide_roi(prs, roi_chart, calendar_chart):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=13, section="ROI shape — for the CFO")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Three revenue paths.  One calendar recovery.",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.0),
             "Conservative pays for itself.\nAggressive changes the category.",
             size=22, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    # ROI bar chart on the left
    s.shapes.add_picture(str(roi_chart), Inches(0.75), Inches(3.0),
                         width=Inches(6.0))
    # Calendar recovery on the right
    s.shapes.add_picture(str(calendar_chart), Inches(7.0), Inches(3.0),
                         width=Inches(5.8))

    add_hairline(s, Inches(0.75), Inches(6.3), Inches(0.6), color=BURGUNDY)
    add_text(s, Inches(0.75), Inches(6.45), Inches(11.83), Inches(0.5),
             "Strategic optionality: Nelson Labs (Sotera) publicly named pharmaceutical-capability acquisitions as a 2026 priority. The 2018 Gibraltar Labs (NJ) precedent is on the record. Life-sciences tools subsector trades at 18-25× EBITDA on recurring-revenue characteristics.",
             size=10, color=CHARCOAL, font=FONT_HEAD, italic=True,
             line_spacing=1.4)


# ---------------------------------------------------------------------------
# Slide 14 — The Gibraltar Precedent (NEW — Slide A)
# ---------------------------------------------------------------------------

def slide_gibraltar(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=14, section="Strategic optionality")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Nelson Labs has done this before.  In New Jersey.",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "The Gibraltar precedent.",
             size=34, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.5), Inches(11.83), color=RULE)

    # Two columns
    add_text(s, Inches(0.75), Inches(3.7), Inches(5.5), Inches(0.4),
             "AUGUST 2018", size=11, color=BURGUNDY, italic=True,
             font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(4.1), Inches(5.5), Inches(0.5),
             "The transaction", size=18, color=NAVY, font=FONT_HEAD)
    add_text(s, Inches(0.75), Inches(4.7), Inches(5.5), Inches(2.0),
             "Sotera Health's Nelson Labs\nacquires Gibraltar Laboratories.\nFairfield, NJ. Family-owned since\n1970. FDA-registered. ISO 17025\naccredited. USP-compendial\nmicrobiology and analytical\nchemistry. Two tri-state-area\nfacilities.",
             size=12, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.4)

    add_text(s, Inches(6.85), Inches(3.7), Inches(5.5), Inches(0.4),
             "MAY 2026", size=11, color=BURGUNDY, italic=True,
             font=FONT_BODY)
    add_text(s, Inches(6.85), Inches(4.1), Inches(5.5), Inches(0.5),
             "The statement", size=18, color=NAVY, font=FONT_HEAD)
    add_text(s, Inches(6.85), Inches(4.7), Inches(5.5), Inches(2.0),
             "Sotera Health Q1 2026 earnings\ncall: 'Targeted acquisitions or\npartnerships could add testing\ncapability, regional sterilization\naccess, or technology depth.'\nManagement 'exploring strategic\nacquisitions to enhance Nelson\nLabs' pharmaceutical capabilities.'",
             size=12, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.4)

    add_hairline(s, Inches(0.75), Inches(6.7), Inches(2.5),
                 color=BURGUNDY, weight=1.25)
    add_text(s, Inches(0.75), Inches(6.85), Inches(11.83), Inches(0.4),
             "The structural parallel is uncomfortable to ignore.",
             size=15, color=NAVY, font=FONT_HEAD, italic=True)


# ---------------------------------------------------------------------------
# Slide 15 — Adjacent opportunities
# ---------------------------------------------------------------------------

def slide_adjacent(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=15, section="Adjacent opportunities")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Where else this team could play",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Adjacencies, not pivots.\nOptional, not required.",
             size=22, color=NAVY, font=FONT_HEAD, line_spacing=1.1)
    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)

    items = [
        ("Combination products",
         "Cross-center (CDER + CDRH) submissions. Few labs ready."),
        ("Device sterilization validation",
         "ISO 11135 / 11137 adjacency. Same sponsor base."),
        ("Cell-therapy distribution",
         "ISTA/ASTM testing for live-cell packaging. Already in Interphex slate."),
        ("Smart-packaging integrity",
         "Embedded RFID + tamper-evident integrity (TOPPAC infuse class)."),
        ("Compounding pharmacy",
         "USP <797> / <800>. Fragmented buyer base, less competition."),
        ("Veterinary biologics",
         "Same containers. USDA channel. Underpriced."),
    ]
    y = Inches(3.9)
    for title, body in items:
        add_text(s, Inches(0.75), y, Inches(3.5), Inches(0.5),
                 title, size=13, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(4.6), y, Inches(8.2), Inches(0.5),
                 body, size=11.5, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.35)
        y += Inches(0.5)


# ---------------------------------------------------------------------------
# Slide 16 — Risks
# ---------------------------------------------------------------------------

def slide_risks(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=16, section="Risks — named, not glossed")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "What we'd want to disagree about",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Three CFO-relevant risks,\nthree handlings.",
             size=24, color=NAVY, font=FONT_HEAD, line_spacing=1.1)
    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)

    risks = [
        ("Capital timing",
         "Lab expansion + RMA + agentic in one fiscal year. Sequence\nagentic behind the lab build; agentic pays back in <12 months."),
        ("Margin dilution from adjacencies",
         "RMA and micro carry lower margin than CCIT. Hold CCIT pricing\nwhile RMA scales; use agentic intel to keep core pipeline full."),
        ("Hallucination in regulated context",
         "Human-in-the-loop on every agent flag that touches a filed\nmethod. Agents are suggestive, not authoritative — by design."),
    ]
    y = Inches(3.9)
    for title, body in risks:
        add_text(s, Inches(0.75), y, Inches(3.7), Inches(0.5),
                 title, size=14, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(4.8), y, Inches(8.0), Inches(1.0),
                 body, size=11.5, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        y += Inches(1.0)


# ---------------------------------------------------------------------------
# Slide 17 — If you wanted to work with us
# ---------------------------------------------------------------------------

def slide_engage(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=17, section="If you wanted to work with us")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "How TSI-Citadel would engage",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Light footprint. Boxed scope.\nThe CFO holds the gate at day 30 and day 90.",
             size=22, color=NAVY, font=FONT_HEAD, line_spacing=1.1)
    add_hairline(s, Inches(0.75), Inches(3.85), Inches(11.83), color=RULE)

    steps = [
        ("DAYS 1 — 30",
         "Diagnostic",
         "Map data feeds, regulatory\ncycles, inquiry-to-quote workflow.\nDeliver: exposure map, BD pipeline\nseed, regulatory-intel trial.\nNo code yet. Go / no-go at day 30."),
        ("DAYS 31 — 60",
         "Two agents live",
         "Regulatory-intel and market-radar\nagents wired to real feeds. Daily\noutput to a named internal user.\nFirst measurable lift inside\nthe period."),
        ("DAYS 61 — 90",
         "Decide",
         "Joint readout. Numbers. The three\nremaining agents are built only if\nthe first two have earned them.\nFixed pricing at outset.\nThe CFO holds the gate."),
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
                 body, size=11.5, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


# ---------------------------------------------------------------------------
# Closing
# ---------------------------------------------------------------------------

def slide_closing(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    add_rect(s, 0, 0, SLIDE_W, SLIDE_H, NAVY)
    add_rect(s, 0, 0, Inches(0.08), SLIDE_H, BURGUNDY)

    add_text(s, Inches(1.1), Inches(1.4), Inches(11), Inches(0.5),
             "THE NEXT CONVERSATION", size=14, color=GOLD,
             italic=True, font=FONT_BODY)
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

    build = ROOT / "_build"
    c_market    = build / "chart_market.png"
    c_comp      = build / "chart_competitive.png"
    c_roi       = build / "chart_roi.png"
    c_container = build / "chart_container_timeline.png"
    c_agentic   = build / "chart_agentic_adoption.png"
    c_calendar  = build / "chart_calendar_recovery.png"

    slide_cover(prs)
    slide_thesis(prs)
    slide_company(prs)
    slide_market(prs, c_market)
    slide_tailwinds(prs)
    slide_container_wave(prs, c_container)
    slide_regulator_adopted(prs)
    slide_competitive(prs, c_comp)
    slide_in_motion(prs)
    slide_organic(prs)
    slide_agentic(prs)
    slide_not_experimental(prs, c_agentic)
    slide_roi(prs, c_roi, c_calendar)
    slide_gibraltar(prs)
    slide_adjacent(prs)
    slide_risks(prs)
    slide_engage(prs)
    slide_closing(prs)

    OUT_PPTX.parent.mkdir(parents=True, exist_ok=True)
    prs.save(OUT_PPTX)
    print(f"wrote {OUT_PPTX}")


if __name__ == "__main__":
    main()
