"""Render the long-form market brief as DOCX and PDF.

Both renderers consume the same `BLOCKS` from report_content.py so the
two files stay in lock-step. Caslon is the named font in DOCX (Word will
substitute if not installed); the PDF uses a Caslon font when available
on the host system, otherwise a registered serif fallback so the layout
stays book-grade.
"""

from __future__ import annotations

import sys
from pathlib import Path

THIS = Path(__file__).resolve()
sys.path.insert(0, str(THIS.parent))
from report_content import BLOCKS  # noqa: E402

ROOT = THIS.parent.parent
OUT_DOCX = ROOT / "05_Market_Analysis_Report" / "CS_Analytical_Market_Brief.docx"
OUT_PDF = ROOT / "05_Market_Analysis_Report" / "CS_Analytical_Market_Brief.pdf"


# ===========================================================================
# DOCX renderer
# ===========================================================================

def render_docx(blocks, out: Path):
    from docx import Document
    from docx.enum.text import WD_ALIGN_PARAGRAPH, WD_LINE_SPACING
    from docx.oxml.ns import qn
    from docx.oxml import OxmlElement
    from docx.shared import Inches, Pt, RGBColor

    doc = Document()

    # Set default font on Normal style to Caslon
    style = doc.styles["Normal"]
    style.font.name = "Adobe Caslon Pro"
    style.font.size = Pt(11.5)
    rpr = style.element.get_or_add_rPr()
    rfonts = rpr.find(qn("w:rFonts"))
    if rfonts is None:
        rfonts = OxmlElement("w:rFonts")
        rpr.append(rfonts)
    for attr in ("w:ascii", "w:hAnsi", "w:cs", "w:eastAsia"):
        rfonts.set(qn(attr), "Adobe Caslon Pro")

    # Page margins
    for section in doc.sections:
        section.top_margin = Inches(0.9)
        section.bottom_margin = Inches(0.9)
        section.left_margin = Inches(1.1)
        section.right_margin = Inches(1.1)

    NAVY = RGBColor(0x0B, 0x25, 0x45)
    BURGUNDY = RGBColor(0x6E, 0x1B, 0x26)
    CHARCOAL = RGBColor(0x1C, 0x1C, 0x1C)
    SLATE = RGBColor(0x55, 0x5B, 0x66)

    def set_run_font(run, *, size, color=CHARCOAL, bold=False, italic=False,
                     font="Adobe Caslon Pro"):
        run.font.name = font
        run.font.size = Pt(size)
        run.font.bold = bold
        run.font.italic = italic
        run.font.color.rgb = color
        rpr = run._element.get_or_add_rPr()
        rfonts = rpr.find(qn("w:rFonts"))
        if rfonts is None:
            rfonts = OxmlElement("w:rFonts")
            rpr.append(rfonts)
        for attr in ("w:ascii", "w:hAnsi", "w:cs", "w:eastAsia"):
            rfonts.set(qn(attr), font)

    def add_para(text, *, size=11.5, color=CHARCOAL, bold=False, italic=False,
                 align=None, space_before=0, space_after=6, line_spacing=1.35,
                 left_indent=None):
        p = doc.add_paragraph()
        pf = p.paragraph_format
        pf.space_before = Pt(space_before)
        pf.space_after = Pt(space_after)
        pf.line_spacing = line_spacing
        if align is not None:
            p.alignment = align
        if left_indent is not None:
            pf.left_indent = Inches(left_indent)
        run = p.add_run(text)
        set_run_font(run, size=size, color=color, bold=bold, italic=italic)
        return p

    def add_h1(text):
        add_para(text, size=24, color=NAVY, bold=False,
                 space_before=24, space_after=10, line_spacing=1.1)
        # underline rule
        rule = doc.add_paragraph()
        rule.paragraph_format.space_before = Pt(0)
        rule.paragraph_format.space_after = Pt(8)
        rule_run = rule.add_run("———————")
        set_run_font(rule_run, size=11, color=BURGUNDY)

    def add_h2(text):
        add_para(text, size=15.5, color=NAVY,
                 space_before=14, space_after=4, line_spacing=1.15)

    def add_h3(text):
        add_para(text, size=12.5, color=BURGUNDY, italic=True,
                 space_before=10, space_after=2, line_spacing=1.2)

    def add_image(path, caption):
        p = doc.add_paragraph()
        p.alignment = WD_ALIGN_PARAGRAPH.CENTER
        r = p.add_run()
        r.add_picture(str(path), width=Inches(6.0))
        cap = doc.add_paragraph()
        cap.alignment = WD_ALIGN_PARAGRAPH.CENTER
        cap.paragraph_format.space_after = Pt(12)
        cr = cap.add_run(caption)
        set_run_font(cr, size=9.5, color=SLATE, italic=True)

    def add_bul(items):
        for item in items:
            p = doc.add_paragraph(style="List Bullet")
            p.paragraph_format.space_after = Pt(4)
            p.paragraph_format.line_spacing = 1.35
            # ensure the bullet text run uses our font
            for r in p.runs:
                set_run_font(r, size=11.5)
            r = p.add_run(item)
            set_run_font(r, size=11.5)

    def add_num(items):
        for item in items:
            p = doc.add_paragraph(style="List Number")
            p.paragraph_format.space_after = Pt(4)
            p.paragraph_format.line_spacing = 1.35
            for r in p.runs:
                set_run_font(r, size=11.5)
            r = p.add_run(item)
            set_run_font(r, size=11.5)

    def add_callout(title, body):
        # Title
        add_para(title, size=12, color=BURGUNDY, italic=True, bold=False,
                 space_before=10, space_after=2, left_indent=0.3)
        # Body
        add_para(body, size=11, color=CHARCOAL, italic=False,
                 space_after=12, left_indent=0.3, line_spacing=1.4)

    def add_rule():
        p = doc.add_paragraph()
        p.alignment = WD_ALIGN_PARAGRAPH.CENTER
        p.paragraph_format.space_before = Pt(6)
        p.paragraph_format.space_after = Pt(6)
        r = p.add_run("— — —")
        set_run_font(r, size=11, color=BURGUNDY)

    def add_quote(text):
        add_para(text, size=12.5, color=NAVY, italic=True,
                 space_before=8, space_after=10, line_spacing=1.45,
                 left_indent=0.4)

    def add_lead(text):
        add_para(text, size=13, color=NAVY, italic=True,
                 space_before=4, space_after=12, line_spacing=1.4)

    def add_cover(payload):
        # Eyebrow
        add_para(payload["eyebrow"], size=11, color=SLATE, italic=True,
                 space_before=120, space_after=8)
        # Title
        add_para(payload["title"], size=32, color=NAVY,
                 space_after=14, line_spacing=1.1)
        # Subtitle
        add_para(payload["subtitle"], size=15, color=CHARCOAL, italic=True,
                 space_after=80, line_spacing=1.3)
        # Author + date
        add_para(payload["author"], size=12, color=CHARCOAL, space_after=2)
        add_para(payload["date"], size=11, color=SLATE, italic=True,
                 space_after=10)

    def add_pagebreak():
        p = doc.add_paragraph()
        r = p.add_run()
        br = OxmlElement("w:br")
        br.set(qn("w:type"), "page")
        r._r.append(br)

    # ----- dispatch -----
    for kind, payload in blocks:
        if kind == "cover":
            add_cover(payload)
        elif kind == "h1":
            add_h1(payload)
        elif kind == "h2":
            add_h2(payload)
        elif kind == "h3":
            add_h3(payload)
        elif kind == "p":
            add_para(payload)
        elif kind == "lead":
            add_lead(payload)
        elif kind == "quote":
            add_quote(payload)
        elif kind == "bul":
            add_bul(payload)
        elif kind == "num":
            add_num(payload)
        elif kind == "img":
            path, caption = payload
            add_image(path, caption)
        elif kind == "rule":
            add_rule()
        elif kind == "callout":
            title, body = payload
            add_callout(title, body)
        elif kind == "pagebreak":
            add_pagebreak()

    out.parent.mkdir(parents=True, exist_ok=True)
    doc.save(out)
    print(f"wrote {out}")


# ===========================================================================
# PDF renderer (reportlab Platypus)
# ===========================================================================

def render_pdf(blocks, out: Path):
    from reportlab.lib.colors import HexColor
    from reportlab.lib.enums import TA_LEFT, TA_CENTER
    from reportlab.lib.pagesizes import LETTER
    from reportlab.lib.styles import ParagraphStyle
    from reportlab.lib.units import inch
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    from reportlab.platypus import (
        BaseDocTemplate, Frame, PageTemplate, Paragraph, Spacer, Image,
        PageBreak, KeepTogether, ListFlowable, ListItem, HRFlowable, Table,
        TableStyle,
    )

    NAVY_HEX = "#0B2545"
    BURGUNDY_HEX = "#6E1B26"
    CHARCOAL_HEX = "#1C1C1C"
    SLATE_HEX = "#555B66"
    RULE_HEX = "#C9BEA8"
    IVORY_HEX = "#FAF7F2"
    NAVY = HexColor(NAVY_HEX)
    BURGUNDY = HexColor(BURGUNDY_HEX)
    CHARCOAL = HexColor(CHARCOAL_HEX)
    SLATE = HexColor(SLATE_HEX)
    IVORY_BG = HexColor("#F4EFE3")

    # ---- font setup ----
    def try_register(name, path):
        if Path(path).exists():
            try:
                pdfmetrics.registerFont(TTFont(name, path))
                return True
            except Exception:
                return False
        return False

    body_font = "Times-Roman"
    bold_font = "Times-Bold"
    italic_font = "Times-Italic"
    bold_italic_font = "Times-BoldItalic"

    serif_candidates = [
        ("CaslonLike", "/usr/share/fonts/truetype/caslon/AdobeCaslonPro-Regular.ttf"),
        ("CaslonLike", "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf"),
    ]
    for name, path in serif_candidates:
        if try_register(name, path):
            body_font = name
            break
    if try_register("CaslonLike-Bold", "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Bold.ttf"):
        bold_font = "CaslonLike-Bold"
    if try_register("CaslonLike-Italic", "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf"):
        italic_font = "CaslonLike-Italic"
    if try_register("CaslonLike-BoldItalic", "/usr/share/fonts/truetype/dejavu/DejaVuSerif-BoldItalic.ttf"):
        bold_italic_font = "CaslonLike-BoldItalic"

    # Register a font family so <b> / <i> tags resolve correctly
    from reportlab.pdfbase.pdfmetrics import registerFontFamily
    registerFontFamily(
        body_font, normal=body_font, bold=bold_font,
        italic=italic_font, boldItalic=bold_italic_font,
    )

    # ---- page templates ----
    PAGE_W, PAGE_H = LETTER
    MARGIN = 0.85 * inch
    FRAME_X = MARGIN
    FRAME_Y = MARGIN + 0.4 * inch
    FRAME_W = PAGE_W - 2 * MARGIN
    FRAME_H = PAGE_H - 2 * MARGIN - 0.4 * inch

    cover_frame = Frame(MARGIN, MARGIN, PAGE_W - 2 * MARGIN,
                        PAGE_H - 2 * MARGIN, showBoundary=0,
                        leftPadding=0, rightPadding=0,
                        topPadding=0, bottomPadding=0)
    body_frame = Frame(FRAME_X, FRAME_Y, FRAME_W, FRAME_H, showBoundary=0,
                       leftPadding=0, rightPadding=0,
                       topPadding=0, bottomPadding=0)

    def draw_cover_chrome(c, doc):
        c.saveState()
        c.setFillColor(HexColor(IVORY_HEX))
        c.rect(0, 0, PAGE_W, PAGE_H, fill=1, stroke=0)
        c.setFillColor(NAVY)
        c.rect(0, 0, 0.5 * inch, PAGE_H, fill=1, stroke=0)
        c.setFillColor(BURGUNDY)
        c.rect(0.5 * inch, 0, 0.07 * inch, PAGE_H, fill=1, stroke=0)
        c.restoreState()

    def draw_body_chrome(c, doc):
        c.saveState()
        # subtle ivory ground
        c.setFillColor(HexColor(IVORY_HEX))
        c.rect(0, 0, PAGE_W, PAGE_H, fill=1, stroke=0)
        # top rule
        c.setStrokeColor(HexColor(RULE_HEX))
        c.setLineWidth(0.5)
        c.line(MARGIN, PAGE_H - MARGIN + 0.25 * inch,
               PAGE_W - MARGIN, PAGE_H - MARGIN + 0.25 * inch)
        # running header
        c.setFont(italic_font, 8)
        c.setFillColor(SLATE)
        c.drawString(MARGIN, PAGE_H - MARGIN + 0.35 * inch,
                     "CS ANALYTICAL  ·  MARKET BRIEF")
        c.drawRightString(PAGE_W - MARGIN, PAGE_H - MARGIN + 0.35 * inch,
                          "Prepared for Al Weiss  ·  Confidential")
        # bottom rule
        c.line(MARGIN, MARGIN + 0.15 * inch,
               PAGE_W - MARGIN, MARGIN + 0.15 * inch)
        c.setFont(body_font, 9)
        c.drawString(MARGIN, MARGIN - 0.05 * inch, "Brian Mulhall  ·  CS Analytical Laboratory")
        c.drawRightString(PAGE_W - MARGIN, MARGIN - 0.05 * inch, f"{doc.page}")
        c.restoreState()

    doc = BaseDocTemplate(
        str(out), pagesize=LETTER,
        leftMargin=MARGIN, rightMargin=MARGIN,
        topMargin=MARGIN, bottomMargin=MARGIN,
        title="CS Analytical — Market Brief for Al Weiss",
        author="Brian Mulhall, CEO, CS Analytical Laboratory",
        subject="Market opportunity & agentic AI thesis",
    )
    doc.addPageTemplates([
        PageTemplate(id="cover", frames=[cover_frame], onPage=draw_cover_chrome),
        PageTemplate(id="body", frames=[body_frame], onPage=draw_body_chrome),
    ])

    # ---- styles ----
    base = ParagraphStyle(
        "base", fontName=body_font, fontSize=11, leading=15.5,
        textColor=CHARCOAL, alignment=TA_LEFT, spaceAfter=8,
    )
    h1 = ParagraphStyle(
        "h1", parent=base, fontName=body_font, fontSize=20,
        leading=24, textColor=NAVY, spaceBefore=22, spaceAfter=8,
    )
    h2 = ParagraphStyle(
        "h2", parent=base, fontName=body_font, fontSize=14,
        leading=18, textColor=NAVY, spaceBefore=14, spaceAfter=4,
    )
    h3 = ParagraphStyle(
        "h3", parent=base, fontName=italic_font, fontSize=11.5,
        leading=15, textColor=BURGUNDY, spaceBefore=10, spaceAfter=3,
    )
    lead = ParagraphStyle(
        "lead", parent=base, fontName=italic_font, fontSize=12.5,
        leading=18, textColor=NAVY, spaceAfter=12,
    )
    quote = ParagraphStyle(
        "quote", parent=base, fontName=italic_font, fontSize=12,
        leading=17, textColor=NAVY, leftIndent=24, rightIndent=12,
        spaceBefore=6, spaceAfter=10,
    )
    body = ParagraphStyle(
        "body", parent=base, fontName=body_font, fontSize=11,
        leading=15.5, textColor=CHARCOAL, spaceAfter=9,
    )
    caption = ParagraphStyle(
        "caption", parent=base, fontName=italic_font, fontSize=9,
        leading=12, textColor=SLATE, alignment=TA_CENTER,
        spaceBefore=4, spaceAfter=14,
    )
    bullet = ParagraphStyle(
        "bullet", parent=body, leftIndent=18, bulletIndent=4,
        spaceAfter=5,
    )
    cover_eyebrow = ParagraphStyle(
        "cover_eyebrow", parent=base, fontName=italic_font, fontSize=11,
        leading=14, textColor=SLATE, alignment=TA_LEFT,
        spaceBefore=200, spaceAfter=12,
    )
    cover_title = ParagraphStyle(
        "cover_title", parent=base, fontName=body_font, fontSize=30,
        leading=36, textColor=NAVY, spaceAfter=12,
    )
    cover_subtitle = ParagraphStyle(
        "cover_subtitle", parent=base, fontName=italic_font, fontSize=14,
        leading=20, textColor=CHARCOAL, spaceAfter=120,
    )
    cover_author = ParagraphStyle(
        "cover_author", parent=base, fontName=body_font, fontSize=12,
        leading=16, textColor=CHARCOAL, spaceAfter=2,
    )
    cover_date = ParagraphStyle(
        "cover_date", parent=base, fontName=italic_font, fontSize=10,
        leading=13, textColor=SLATE,
    )
    callout_title = ParagraphStyle(
        "callout_title", parent=base, fontName=italic_font, fontSize=11.5,
        leading=15, textColor=BURGUNDY, spaceAfter=4,
    )
    callout_body = ParagraphStyle(
        "callout_body", parent=base, fontName=body_font, fontSize=10.5,
        leading=15, textColor=CHARCOAL, spaceAfter=4,
    )

    story = []

    # cover at left margin 0.7" from the navy spine
    def emit_cover(payload):
        story.append(Paragraph(payload["eyebrow"], cover_eyebrow))
        story.append(Paragraph(payload["title"], cover_title))
        story.append(Paragraph(payload["subtitle"], cover_subtitle))
        story.append(Paragraph(payload["author"], cover_author))
        story.append(Paragraph(payload["date"], cover_date))

    from reportlab.platypus.doctemplate import NextPageTemplate

    for kind, payload in blocks:
        if kind == "cover":
            story.append(NextPageTemplate("cover"))
            emit_cover(payload)
            continue
        if kind == "pagebreak":
            story.append(NextPageTemplate("body"))
            story.append(PageBreak())
            continue
        if kind == "h1":
            story.append(Paragraph(payload, h1))
            story.append(HRFlowable(width="20%", thickness=1.2,
                                    color=BURGUNDY, spaceBefore=0,
                                    spaceAfter=8))
        elif kind == "h2":
            story.append(Paragraph(payload, h2))
        elif kind == "h3":
            story.append(Paragraph(payload, h3))
        elif kind == "p":
            story.append(Paragraph(payload, body))
        elif kind == "lead":
            story.append(Paragraph(payload, lead))
        elif kind == "quote":
            story.append(Paragraph(payload, quote))
        elif kind == "bul":
            items = [ListItem(Paragraph(t, bullet), bulletColor=BURGUNDY)
                     for t in payload]
            story.append(ListFlowable(items, bulletType="bullet",
                                      bulletFontSize=8,
                                      leftIndent=18, bulletColor=BURGUNDY))
            story.append(Spacer(1, 6))
        elif kind == "num":
            items = [ListItem(Paragraph(t, bullet)) for t in payload]
            story.append(ListFlowable(items, bulletType="1", leftIndent=22,
                                      bulletFormat="%s."))
            story.append(Spacer(1, 6))
        elif kind == "img":
            path, cap = payload
            from PIL import Image as PILImage
            img_w_in = 6.0
            with PILImage.open(path) as im:
                aspect = im.height / im.width
            img_h_in = img_w_in * aspect
            img = Image(str(path), width=img_w_in * inch, height=img_h_in * inch)
            img.hAlign = "CENTER"
            story.append(KeepTogether([img, Paragraph(cap, caption)]))
        elif kind == "rule":
            story.append(Spacer(1, 8))
            story.append(HRFlowable(width="100%", thickness=0.5,
                                    color=HexColor(RULE_HEX),
                                    spaceBefore=4, spaceAfter=10))
        elif kind == "callout":
            title, content = payload
            inner = [Paragraph(title, callout_title),
                     Paragraph(content, callout_body)]
            table = Table([[inner]], colWidths=[FRAME_W - 0.5 * inch])
            table.setStyle(TableStyle([
                ("BACKGROUND", (0, 0), (-1, -1), IVORY_BG),
                ("LEFTPADDING", (0, 0), (-1, -1), 14),
                ("RIGHTPADDING", (0, 0), (-1, -1), 14),
                ("TOPPADDING", (0, 0), (-1, -1), 10),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 10),
                ("LINEBEFORE", (0, 0), (0, -1), 2, BURGUNDY),
            ]))
            story.append(Spacer(1, 4))
            story.append(table)
            story.append(Spacer(1, 8))

    out.parent.mkdir(parents=True, exist_ok=True)
    doc.build(story)
    print(f"wrote {out}")


# ===========================================================================
# Main
# ===========================================================================

def main():
    render_docx(BLOCKS, OUT_DOCX)
    render_pdf(BLOCKS, OUT_PDF)


if __name__ == "__main__":
    main()
