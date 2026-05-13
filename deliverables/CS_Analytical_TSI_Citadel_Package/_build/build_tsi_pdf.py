"""Build the CS Analytical × TSI Citadel partnership deck as PDF + PNGs.

Mirrors build_tsi_deck.py one-for-one. Same Caslon serif system. We
reuse the drawing primitives from build_pdf.py.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

THIS = Path(__file__).resolve()
sys.path.insert(0, str(THIS.parent))

from build_pdf import (  # noqa: E402
    BURGUNDY, CHARCOAL, FONTS, GOLD, IVORY, NAVY, PAGE_H, PAGE_W,
    RULE, SLATE,
    fill, hairline, rect, text, y_from_top,
)
from reportlab.lib.units import inch  # noqa: E402
from reportlab.pdfgen import canvas  # noqa: E402

ROOT = THIS.parent.parent
OUT_PDF = ROOT / "06_TSI_Citadel_Variant" / "PDF" / "CS_Analytical_for_TSI_Citadel.pdf"
SLIDES_DIR = ROOT / "06_TSI_Citadel_Variant" / "Slide_Images"


# ---------------------------------------------------------------------------
# Chrome
# ---------------------------------------------------------------------------

def page_chrome(c, *, page_no, total, section):
    rect(c, 0, 0, PAGE_W, PAGE_H, IVORY)
    hairline(c, 0.75 * inch, y_from_top(0.55), 11.83 * inch,
             color=RULE, weight=0.6)
    text(c, 0.75, 0.30, section.upper(), size=8.5, color=SLATE,
         font=FONTS["italic"], italic=True)
    text(c, 8.0, 0.30, "CS ANALYTICAL  ×  TSI CITADEL  ·  PARTNERSHIP BRIEF",
         size=8.5, color=SLATE, italic=True, align="right",
         max_width_in=4.83)
    hairline(c, 0.75 * inch, y_from_top(7.05), 11.83 * inch,
             color=RULE, weight=0.45)
    text(c, 0.75, 7.12, "Confidential  ·  for discussion",
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

    text(c, 1.1, 0.85, "CS ANALYTICAL  ×  TSI CITADEL",
         size=13, color=SLATE, italic=True)
    hairline(c, 1.1 * inch, y_from_top(1.35), 3.5 * inch,
             color=BURGUNDY, weight=1.5)

    text(c, 1.1, 1.85,
         "A partnership brief.\nA regulated first vertical\nfor an agentic platform.",
         size=40, color=NAVY, line_spacing=1.1)

    hairline(c, 1.1 * inch, y_from_top(5.55), 2.2 * inch,
             color=NAVY, weight=1.0)
    text(c, 1.1, 5.75, "Prepared by CS Analytical Laboratory",
         size=18, color=CHARCOAL)
    text(c, 1.1, 6.15, "Brian Mulhall, Chief Executive Officer",
         size=12, color=SLATE, italic=True)
    text(c, 1.1, 6.5, "May 2026", size=10, color=SLATE)


def slide_proposition(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The proposition")
    text(c, 0.75, 1.05, "One sentence.", size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "We'd like to be your\nfirst regulated vertical.",
         size=36, color=NAVY, line_spacing=1.1)

    hairline(c, 0.75 * inch, y_from_top(5.5), 2.5 * inch,
             color=BURGUNDY, weight=1.25)
    text(c, 0.75, 5.7,
         "CS Analytical is small enough to move fast, regulated\nenough to prove the case, and operates inside one of\nthe most data-rich, narrowly-scoped, high-stakes niches\nin pharma. That combination is rare.",
         size=15, color=CHARCOAL, italic=True, line_spacing=1.4)


def slide_company(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The company")
    text(c, 0.75, 1.05, "CS Analytical at a glance",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Forty-eight people, one\nspecialty, thirty years.",
         size=30, color=NAVY, line_spacing=1.08)
    hairline(c, 0.75 * inch, y_from_top(4.15), 2.5 * inch,
             color=BURGUNDY, weight=1.25)

    facts = [
        ("Specialty",
         "Container Closure Integrity Testing.\nUSP <1207>-built. cGMP from the\nfloor up."),
        ("Footprint",
         "48 FTE. PhD-staffed. Founder-led.\nNew Jersey-based, U.S. and\ninternational clients."),
        ("Track record",
         "CEO Brian Mulhall built the first\nFDA-regulated cGMP CCIT lab in\nthe world. Methods filed\nagainst USP <1207>."),
        ("Position",
         "Vendor-neutral on platforms.\nMethod-led, not box-led.\nFounder-accountable on every\nstudy."),
    ]
    x = 0.75
    col_w = 2.95
    for label, body in facts:
        rect(c, x * inch, y_from_top(4.34), (col_w - 0.15) * inch, 2, BURGUNDY)
        text(c, x, 4.45, label, size=14, color=NAVY)
        text(c, x, 4.95, body, size=11.5, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_domain(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The domain")
    text(c, 0.75, 1.05, "Why CCIT is the right first vertical",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Bounded ontology. Structured data.\nReal money attached.",
         size=22, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(3.8), 11.83 * inch, color=RULE)

    facets = [
        ("Bounded ontology",
         "Vials, syringes, autoinjectors,\ncartridges, cryo-bags. A finite\nworld an agent can model."),
        ("Structured public data",
         "FDA filings, ClinicalTrials.gov,\nUSP / EP / PIC/S guidance —\nall machine-readable."),
        ("Regulatory cycle",
         "Clear gating moments: USP\nupdates, 483s, warning letters,\nPDUFA dates."),
        ("Sparse specialist field",
         "Fewer than two dozen credible\nCCIT specialists worldwide.\nSignal-to-noise is unusual."),
    ]
    x = 0.75
    col_w = 2.95
    for title, body in facets:
        text(c, x, 4.05, title, size=15, color=NAVY)
        rect(c, x * inch, y_from_top(4.5), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.7, body, size=11, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_signal_density(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Signal density")
    text(c, 0.75, 1.05, "The five feeds that matter",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Every one of them is public, structured,\nand continuously moving.",
         size=22, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(3.8), 11.83 * inch, color=RULE)

    feeds = [
        ("01", "FDA",
         "Novel approvals. Supplements.\nForm 483 observations. Warning\nletters. Inspection outcomes."),
        ("02", "ClinicalTrials.gov",
         "Sterile-injectable indications.\nPhase II/III readouts. Sponsor\nidentities and timelines."),
        ("03", "USP · EP · JP · PIC/S",
         "Monograph and guidance\nupdates. Resolves directly\nagainst client filed methods."),
        ("04", "Conference rosters",
         "PDA, INTERPHEX, AAPS, BIO.\nWho is presenting tells you\nwho is about to file."),
        ("05", "Container-vendor releases",
         "West, Schott, Stevanato, BD.\nNew formats trigger new\nmethods."),
    ]
    x = 0.75
    col_w = 2.40
    for num, title, body in feeds:
        text(c, x, 4.05, num, size=20, color=BURGUNDY, italic=True)
        text(c, x, 4.55, title, size=13, color=NAVY)
        text(c, x, 5.0, body, size=10.5, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_agents(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Agent specifications")
    text(c, 0.75, 1.05, "Five agents we'd want built",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Concrete specs. Buildable in\nninety days. Measurable from day one.",
         size=20, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(3.8), 11.83 * inch, color=RULE)

    agents = [
        ("Market radar",
         "Ingests five feeds, resolves\nto sponsor entities, emits a\nweekly BD shortlist with\nevidence linked."),
        ("Regulatory intel",
         "Detects USP / EP / PIC/S\nchange. Resolves against\nclient filed methods.\nFlags exposure."),
        ("BD operations",
         "Drafts outbound. Qualifies\nreplies. Schedules.\nPrepares meeting briefs.\nReturns founder calendar."),
        ("Scientific surveillance",
         "Distills CCIT, HVLD,\nhelium-leak, vacuum-decay\nliterature daily. Methods\nupdates surface first."),
        ("Proposal synthesis",
         "Inquiry to quoted proposal\nin hours. Pulls method,\ncapacity, precedent,\nregulatory context."),
    ]
    x = 0.75
    col_w = 2.40
    for title, body in agents:
        text(c, x, 4.05, title, size=13, color=NAVY)
        rect(c, x * inch, y_from_top(4.5), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 4.7, body, size=10.5, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_pilot(c, page, total):
    page_chrome(c, page_no=page, total=total, section="The pilot")
    text(c, 0.75, 1.05, "A 90-day, boxed pilot",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Three workstreams.\nOne joint readout.",
         size=28, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.85), 11.83 * inch, color=RULE)

    streams = [
        ("DAYS 1 — 30", "Stand up the agents",
         "TSI Citadel agents wired to the\nfive feeds. First weekly BD digest\nlive by day 15. First regulatory-\nintel flag delivered by day 20."),
        ("DAYS 31 — 60", "Pilot two growth vectors",
         "Cryo-container methods and\nautoinjector HVLD. Two named\npilot sponsors each. Methods\ndrafted, quotes out, first\nrevenue booked."),
        ("DAYS 61 — 90", "Package and price",
         "Productized CCIT-as-a-service\ntier defined. Two public case\nstudies drafted with client\nco-sign. Joint readout."),
    ]
    x = 0.75
    col_w = 3.95
    for label, title, body in streams:
        text(c, x, 4.05, label, size=10, color=BURGUNDY, italic=True)
        text(c, x, 4.45, title, size=15, color=NAVY)
        rect(c, x * inch, y_from_top(4.95), 0.4 * inch, 2, BURGUNDY)
        text(c, x, 5.15, body, size=11, color=CHARCOAL, line_spacing=1.4)
        x += col_w


def slide_roi(c, page, total):
    page_chrome(c, page_no=page, total=total, section="ROI shape")
    text(c, 0.75, 1.05, "Two readings of the number",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "Conservative pays for itself.\nAggressive changes the category.",
         size=22, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(3.95), 11.83 * inch, color=RULE)

    # Two columns
    text(c, 0.75, 4.15, "CONSERVATIVE", size=10, color=BURGUNDY, italic=True)
    text(c, 0.75, 4.5, "Operational gains", size=16, color=NAVY)
    text(c, 0.75, 5.0,
         "2×  qualified BD opportunities per quarter.\n60%  reduction in inquiry-to-quote cycle.\n30%  founder calendar recovered.\n15-25 pp  win-rate improvement.\n2-3×  revenue uplift over 36 months.",
         size=11, color=CHARCOAL, line_spacing=1.7)

    # vertical divider
    hairline(c, 6.7 * inch, y_from_top(4.15), 0, color=RULE)
    rect(c, 6.7 * inch, y_from_top(6.7), 1, 2.5 * inch, RULE)

    text(c, 6.95, 4.15, "AGGRESSIVE", size=10, color=BURGUNDY, italic=True)
    text(c, 6.95, 4.5, "Category shift", size=16, color=NAVY)
    text(c, 6.95, 5.0,
         "Productized CCIT-as-a-service:\ncontinuous, monitored, regulatory-aware.\nRecurring revenue. Software-augmented\nscience pricing. A new line of business,\nnot a faster version of the old one.",
         size=11, color=CHARCOAL, line_spacing=1.55)


def slide_what_we_bring(c, page, total):
    page_chrome(c, page_no=page, total=total,
                section="What we bring TSI Citadel")
    text(c, 0.75, 1.05,
         "Why this partnership is asymmetric in your favor",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6,
         "A real regulated vertical is\nworth more than a synthetic one.",
         size=22, color=NAVY, line_spacing=1.12)
    hairline(c, 0.75 * inch, y_from_top(3.8), 11.83 * inch, color=RULE)

    items = [
        ("Proof-of-platform",
         "A live, named, FDA-regulated customer running agents against real\nregulatory data — not a sandbox demo."),
        ("Co-marketable case study",
         "Public case studies with named regulatory traction. The kind of\nproof that closes the next ten enterprise conversations for you."),
        ("Scientific feedback loop",
         "PhD-level domain experts pressure-testing agent reasoning against\nthe regulatory reality. The agents get smarter."),
        ("Repeatable template",
         "Specialty laboratories are a fragmented, under-served market.\nCS Analytical is the template; the next ten are the market."),
        ("Stable counterparty",
         "Founder-led, profitable, regulated. Low deployment risk.\nNo platform team has to babysit the integration."),
    ]
    y = 4.0
    for title, body in items:
        text(c, 0.75, y, title, size=13, color=NAVY)
        text(c, 4.4, y + 0.05, body, size=11, color=CHARCOAL,
             line_spacing=1.4)
        y += 0.6


def slide_risks(c, page, total):
    page_chrome(c, page_no=page, total=total, section="Risks and handling")
    text(c, 0.75, 1.05,
         "Things that could go wrong, and what we'd do",
         size=12, color=BURGUNDY, italic=True)
    text(c, 0.75, 1.6, "Named, not glossed.",
         size=28, color=NAVY, line_spacing=1.1)
    hairline(c, 0.75 * inch, y_from_top(3.8), 11.83 * inch, color=RULE)

    risks = [
        ("Platform lock-in",
         "Data feeds are commodity sources. Agent prompts and reasoning\nchains version-controlled and exportable. Switching cost contained."),
        ("Hallucination in regulatory context",
         "Human-in-the-loop on every flag that touches a client filed method.\nRegulatory intel is suggested, not authoritative — by design."),
        ("Pilot cycle-time slippage",
         "Boxed 90-day scope, weekly checkpoints, success criteria defined\nup front. The pilot ends on day 90 with a readout, on time."),
        ("Misaligned incentives",
         "Commercial structure agreed before agents are built: pilot fee,\nrevenue share on productized tier, joint case-study rights."),
    ]
    y = 4.0
    for title, body in risks:
        text(c, 0.75, y, title, size=13, color=NAVY)
        text(c, 4.8, y + 0.05, body, size=11, color=CHARCOAL,
             line_spacing=1.4)
        y += 0.7


def slide_closing(c):
    rect(c, 0, 0, PAGE_W, PAGE_H, NAVY)
    rect(c, 0, 0, 0.08 * inch, PAGE_H, BURGUNDY)

    text(c, 1.1, 1.45, "THE NEXT CONVERSATION",
         size=12, color=GOLD, italic=True)
    hairline(c, 1.1 * inch, y_from_top(2.0), 2.5 * inch,
             color=BURGUNDY, weight=1.5)

    text(c, 1.1, 2.35,
         "A scoping call.\nA look at the agent specs.\nA boxed ninety-day pilot.",
         size=38, color=IVORY, line_spacing=1.1)

    text(c, 1.1, 5.5, "Brian Mulhall  ·  Chief Executive Officer",
         size=16, color=IVORY)
    text(c, 1.1, 5.9, "CS Analytical Laboratory",
         size=12, color=GOLD, italic=True)
    text(c, 1.1, 6.25, "brian.mulhall@csanalytical.com",
         size=12, color=IVORY)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    OUT_PDF.parent.mkdir(parents=True, exist_ok=True)
    SLIDES_DIR.mkdir(parents=True, exist_ok=True)

    c = canvas.Canvas(str(OUT_PDF), pagesize=(PAGE_W, PAGE_H))
    c.setTitle("CS Analytical × TSI Citadel — Partnership Brief")
    c.setAuthor("Brian Mulhall, CEO, CS Analytical Laboratory")
    c.setSubject("Partnership brief")

    total = 10

    slide_cover(c);                    c.showPage()
    slide_proposition(c, 2, total);    c.showPage()
    slide_company(c, 3, total);        c.showPage()
    slide_domain(c, 4, total);         c.showPage()
    slide_signal_density(c, 5, total); c.showPage()
    slide_agents(c, 6, total);         c.showPage()
    slide_pilot(c, 7, total);          c.showPage()
    slide_roi(c, 8, total);            c.showPage()
    slide_what_we_bring(c, 9, total);  c.showPage()
    slide_risks(c, 10, total);         c.showPage()
    slide_closing(c);                  c.showPage()

    c.save()
    print(f"wrote {OUT_PDF}")

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
