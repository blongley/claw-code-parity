"""Build the CS Analytical × TSI Citadel partnership deck (PowerPoint).

Posture: a B2B partnership pitch, not a personal favor. The protagonist
of this deck is the partnership, not Brian Mulhall. Same Caslon serif
system as the Al Weiss deck so the brand stays consistent.
"""

from __future__ import annotations

import sys
from pathlib import Path

THIS = Path(__file__).resolve()
sys.path.insert(0, str(THIS.parent))

# Reuse the visual primitives and palette from the Al Weiss deck.
from build_deck import (  # noqa: E402
    add_hairline, add_rect, add_text,
    BURGUNDY, CHARCOAL, FONT_BODY, FONT_HEAD, GOLD, IVORY, NAVY, RULE, SLATE,
    SLIDE_H, SLIDE_W,
)
from pptx import Presentation  # noqa: E402
from pptx.enum.text import PP_ALIGN  # noqa: E402
from pptx.util import Emu, Inches  # noqa: E402

ROOT = THIS.parent.parent
OUT_PPTX = ROOT / "06_TSI_Citadel_Variant" / "PowerPoint" / "CS_Analytical_for_TSI_Citadel.pptx"


# ---------------------------------------------------------------------------
# Chrome
# ---------------------------------------------------------------------------

def page_chrome(slide, *, page_no, total, section):
    add_rect(slide, 0, 0, SLIDE_W, SLIDE_H, IVORY)
    add_hairline(slide, Inches(0.75), Inches(0.55), Inches(11.83),
                 color=RULE, weight=0.75)
    add_text(slide, Inches(0.75), Inches(0.28), Inches(8), Inches(0.3),
             section.upper(), size=10, color=SLATE, italic=True,
             font=FONT_BODY)
    add_text(slide, Inches(8.0), Inches(0.28), Inches(4.83), Inches(0.3),
             "CS ANALYTICAL  ×  TSI CITADEL  ·  PARTNERSHIP BRIEF",
             size=10, color=SLATE, italic=True, align=PP_ALIGN.RIGHT,
             font=FONT_BODY)
    add_hairline(slide, Inches(0.75), Inches(7.05), Inches(11.83),
                 color=RULE, weight=0.5)
    add_text(slide, Inches(0.75), Inches(7.12), Inches(8), Inches(0.3),
             "Confidential  ·  for discussion", size=9, color=SLATE,
             italic=True, font=FONT_BODY)
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
             "CS ANALYTICAL  ×  TSI CITADEL", size=14, color=SLATE,
             italic=True, font=FONT_BODY)
    add_hairline(s, Inches(1.1), Inches(1.35), Inches(3.5),
                 color=BURGUNDY, weight=1.5)

    add_text(s, Inches(1.1), Inches(1.7), Inches(11), Inches(2.5),
             "A partnership brief.\nA regulated first vertical\nfor an agentic platform.",
             size=50, color=NAVY, font=FONT_HEAD, line_spacing=1.05)

    add_hairline(s, Inches(1.1), Inches(5.55), Inches(2.2),
                 color=NAVY, weight=1.0)
    add_text(s, Inches(1.1), Inches(5.7), Inches(11), Inches(0.4),
             "Prepared by CS Analytical Laboratory",
             size=18, color=CHARCOAL, font=FONT_HEAD)
    add_text(s, Inches(1.1), Inches(6.1), Inches(11), Inches(0.4),
             "Brian Mulhall, Chief Executive Officer",
             size=13, color=SLATE, italic=True, font=FONT_BODY)
    add_text(s, Inches(1.1), Inches(6.45), Inches(11), Inches(0.4),
             "May 2026", size=11, color=SLATE, font=FONT_BODY)


def slide_proposition(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The proposition")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "One sentence.", size=14, color=BURGUNDY, italic=True,
             font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(3.0),
             "We'd like to be your\nfirst regulated vertical.",
             size=48, color=NAVY, font=FONT_HEAD, line_spacing=1.05)

    add_hairline(s, Inches(0.75), Inches(5.3), Inches(2.5),
                 color=BURGUNDY, weight=1.25)
    add_text(s, Inches(0.75), Inches(5.5), Inches(11.83), Inches(2.0),
             "CS Analytical is small enough to move fast, regulated\nenough to prove the case, and operates inside one of\nthe most data-rich, narrowly-scoped, high-stakes niches\nin pharma. That combination is rare.",
             size=18, color=CHARCOAL, italic=True, font=FONT_HEAD,
             line_spacing=1.35)


def slide_company(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The company")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "CS Analytical at a glance",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Forty-eight people, one\nspecialty, thirty years.",
             size=36, color=NAVY, font=FONT_HEAD, line_spacing=1.08)

    add_hairline(s, Inches(0.75), Inches(4.05), Inches(2.5),
                 color=BURGUNDY, weight=1.25)

    facts = [
        ("Specialty", "Container Closure Integrity Testing.\nUSP <1207>-built. cGMP from the floor up."),
        ("Footprint", "48 FTE.  PhD-staffed.  Founder-led.\nNew Jersey-based, U.S. and international clients."),
        ("Track record", "CEO Brian Mulhall built the first\nFDA-regulated cGMP CCIT lab in the world.\nMethods filed against USP <1207>."),
        ("Position", "Vendor-neutral on platforms.\nMethod-led, not box-led.\nFounder-accountable on every study."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.95)
    for label, body in facts:
        add_rect(s, x, Inches(4.3), col_w - Inches(0.15), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.45), col_w - Inches(0.15), Inches(0.5),
                 label, size=16, color=NAVY, font=FONT_HEAD)
        add_text(s, x, Inches(5.0), col_w - Inches(0.15), Inches(2.0),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


def slide_domain(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The domain")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Why CCIT is the right first vertical",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Bounded ontology. Structured data.\nReal money attached.",
             size=28, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
    facets = [
        ("Bounded ontology",
         "Vials, syringes, autoinjectors,\ncartridges, cryo-bags. A finite\nworld an agent can model in full."),
        ("Structured public data",
         "FDA filings, ClinicalTrials.gov,\nUSP / EP / PIC/S guidance —\nall machine-readable, all moving."),
        ("Regulatory cycle",
         "Clear gating moments: USP\nupdates, 483s, warning letters,\nPDUFA dates. The clock is loud."),
        ("Sparse specialist field",
         "Fewer than two dozen credible\nCCIT specialists worldwide. The\nsignal-to-noise ratio is unusual."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.95)
    for title, body in facets:
        add_text(s, x, Inches(3.9), col_w - Inches(0.15), Inches(0.5),
                 title, size=17, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.4), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.55), col_w - Inches(0.15), Inches(2.5),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


def slide_signal_density(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Signal density")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "The five feeds that matter",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Every one of them is public,\nstructured, and continuously moving.",
             size=26, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)

    feeds = [
        ("01", "FDA",
         "Novel approvals. Supplements.\nForm 483 observations. Warning\nletters. Inspection outcomes."),
        ("02", "ClinicalTrials.gov",
         "Sterile-injectable indications.\nPhase II/III readouts. Sponsor\nidentities and timelines."),
        ("03", "USP · EP · JP · PIC/S",
         "Monograph and guidance updates.\nResolves directly against client\nfiled methods."),
        ("04", "Conference rosters",
         "PDA, INTERPHEX, AAPS, BIO.\nWho is presenting what tells\nyou who is about to file."),
        ("05", "Container-vendor releases",
         "West, Schott, Stevanato, BD.\nNew formats trigger new\nmethods. The cycle is loud."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.40)
    y_label = Inches(3.95)
    y_title = Inches(4.45)
    y_body = Inches(5.0)
    for num, title, body in feeds:
        add_text(s, x, y_label, col_w - Inches(0.1), Inches(0.4),
                 num, size=22, color=BURGUNDY, italic=True, font=FONT_HEAD)
        add_text(s, x, y_title, col_w - Inches(0.1), Inches(0.5),
                 title, size=15, color=NAVY, font=FONT_HEAD)
        add_text(s, x, y_body, col_w - Inches(0.1), Inches(2.2),
                 body, size=11, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


def slide_agents(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Agent specifications")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Five agents we'd want built",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Concrete specs. Buildable in\nninety days. Measurable from day one.",
             size=24, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
    agents = [
        ("Market radar",
         "Ingests five feeds, resolves to\nsponsor entities, emits a weekly\nBD shortlist with evidence linked."),
        ("Regulatory intel",
         "Detects USP / EP / PIC/S change.\nResolves against client filed\nmethods. Flags exposure."),
        ("BD operations",
         "Drafts outbound. Qualifies replies.\nSchedules. Prepares meeting briefs.\nReturns founder calendar."),
        ("Scientific surveillance",
         "Distills CCIT, HVLD, helium-leak,\nvacuum-decay literature daily.\nMethods updates surface first."),
        ("Proposal synthesis",
         "Inquiry to quoted proposal in\nhours. Pulls method, capacity,\nprecedent, regulatory context."),
    ]
    x = Inches(0.75)
    col_w = Inches(2.40)
    for title, body in agents:
        add_text(s, x, Inches(3.95), col_w - Inches(0.1), Inches(0.5),
                 title, size=14, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.45), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(4.6), col_w - Inches(0.1), Inches(2.5),
                 body, size=11, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


def slide_pilot(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="The pilot")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "A 90-day, boxed pilot",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Three workstreams.\nOne joint readout.",
             size=32, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
    streams = [
        ("DAYS 1 — 30",
         "Stand up the agents",
         "TSI Citadel agents wired to the\nfive feeds. First weekly BD digest\nlive by day 15. First regulatory-\nintel flag delivered by day 20.\nInternal users trained."),
        ("DAYS 31 — 60",
         "Pilot two growth vectors",
         "Cryo-container methods and\nautoinjector HVLD. Two named\npilot sponsors each. Methods\ndrafted, quotes out, first revenue\nbooked."),
        ("DAYS 61 — 90",
         "Package and price",
         "Productized CCIT-as-a-service\ntier defined. Two public case\nstudies drafted with client\nco-sign. Joint readout to CS\nAnalytical leadership and the\nTSI Citadel partnership team."),
    ]
    x = Inches(0.75)
    col_w = Inches(3.95)
    for label, title, body in streams:
        add_text(s, x, Inches(3.9), col_w - Inches(0.15), Inches(0.35),
                 label, size=11, color=BURGUNDY, italic=True, font=FONT_BODY)
        add_text(s, x, Inches(4.3), col_w - Inches(0.15), Inches(0.6),
                 title, size=17, color=NAVY, font=FONT_HEAD)
        add_rect(s, x, Inches(4.95), Inches(0.4), Inches(0.04), BURGUNDY)
        add_text(s, x, Inches(5.1), col_w - Inches(0.15), Inches(2.0),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.4)
        x += col_w


def slide_roi(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="ROI shape")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Two readings of the number",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Conservative pays for itself.\nAggressive changes the category.",
             size=26, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.85), Inches(11.83), color=RULE)

    # Two-column comparison
    add_text(s, Inches(0.75), Inches(4.05), Inches(5.5), Inches(0.35),
             "CONSERVATIVE", size=11, color=BURGUNDY, italic=True,
             font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(4.45), Inches(5.5), Inches(0.5),
             "Operational gains", size=18, color=NAVY, font=FONT_HEAD)
    add_text(s, Inches(0.75), Inches(5.0), Inches(5.5), Inches(2.0),
             "2×  qualified BD opportunities per quarter.\n60%  reduction in inquiry-to-quote cycle.\n30%  founder calendar recovered.\n15-25 pp  win-rate improvement.\n2-3×  revenue uplift over 36 months.",
             size=12, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.6)

    add_text(s, Inches(6.85), Inches(4.05), Inches(5.5), Inches(0.35),
             "AGGRESSIVE", size=11, color=BURGUNDY, italic=True,
             font=FONT_BODY)
    add_text(s, Inches(6.85), Inches(4.45), Inches(5.5), Inches(0.5),
             "Category shift", size=18, color=NAVY, font=FONT_HEAD)
    add_text(s, Inches(6.85), Inches(5.0), Inches(5.5), Inches(2.0),
             "Productized CCIT-as-a-service:\ncontinuous, monitored, regulatory-aware.\nRecurring revenue. Software-augmented\nscience pricing. A new line of business\nrather than a faster version of the old one.",
             size=12, color=CHARCOAL, font=FONT_HEAD, line_spacing=1.5)

    # Center divider
    add_hairline(s, Inches(6.6), Inches(4.1), Inches(0), color=RULE)
    add_rect(s, Inches(6.6), Inches(4.1), Emu(8000), Inches(2.8), RULE)


def slide_what_we_bring(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="What we bring TSI Citadel")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Why this partnership is asymmetric in your favor",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "A real regulated vertical is\nworth more than a synthetic one.",
             size=26, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
    items = [
        ("Proof-of-platform",
         "A live, named, FDA-regulated customer running agents against real\nregulatory data — not a sandbox demo."),
        ("Co-marketable case study",
         "Public case studies with named regulatory traction. The kind of\nproof that closes the next ten enterprise conversations for you."),
        ("Scientific feedback loop",
         "PhD-level domain experts pressure-testing agent reasoning against\nthe regulatory reality. The agents get smarter; the platform improves."),
        ("Repeatable template",
         "Specialty laboratories are a fragmented, under-served market.\nCS Analytical is the template; the next ten are the market."),
        ("Stable counterparty",
         "Founder-led, profitable, regulated. Low deployment risk. No\nplatform team has to babysit the integration."),
    ]
    y = Inches(3.9)
    for title, body in items:
        add_text(s, Inches(0.75), y, Inches(3.5), Inches(0.5),
                 title, size=14, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(4.6), y, Inches(8.2), Inches(0.6),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.35)
        y += Inches(0.6)


def slide_risks(prs, page, total):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    page_chrome(s, page_no=page, total=total, section="Risks and handling")

    add_text(s, Inches(0.75), Inches(1.0), Inches(11.83), Inches(0.5),
             "Things that could go wrong, and what we'd do",
             size=14, color=BURGUNDY, italic=True, font=FONT_BODY)
    add_text(s, Inches(0.75), Inches(1.5), Inches(11.83), Inches(1.2),
             "Named, not glossed.",
             size=32, color=NAVY, font=FONT_HEAD, line_spacing=1.1)

    add_hairline(s, Inches(0.75), Inches(3.7), Inches(11.83), color=RULE)
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
    y = Inches(3.9)
    for title, body in risks:
        add_text(s, Inches(0.75), y, Inches(3.7), Inches(0.5),
                 title, size=14, color=NAVY, font=FONT_HEAD)
        add_text(s, Inches(4.8), y, Inches(8.0), Inches(0.7),
                 body, size=12, color=CHARCOAL, font=FONT_HEAD,
                 line_spacing=1.35)
        y += Inches(0.7)


def slide_closing(prs):
    s = prs.slides.add_slide(prs.slide_layouts[6])
    add_rect(s, 0, 0, SLIDE_W, SLIDE_H, NAVY)
    add_rect(s, 0, 0, Inches(0.08), SLIDE_H, BURGUNDY)

    add_text(s, Inches(1.1), Inches(1.4), Inches(11), Inches(0.5),
             "THE NEXT CONVERSATION", size=14, color=GOLD, italic=True,
             font=FONT_BODY)
    add_hairline(s, Inches(1.1), Inches(1.9), Inches(2.5),
                 color=BURGUNDY, weight=1.5)

    add_text(s, Inches(1.1), Inches(2.2), Inches(11), Inches(2.5),
             "A scoping call.\nA look at the agent specs.\nA boxed ninety-day pilot.",
             size=46, color=IVORY, font=FONT_HEAD, line_spacing=1.08)

    add_text(s, Inches(1.1), Inches(5.45), Inches(11), Inches(0.4),
             "Brian Mulhall  ·  Chief Executive Officer",
             size=15, color=IVORY, font=FONT_HEAD)
    add_text(s, Inches(1.1), Inches(5.85), Inches(11), Inches(0.4),
             "CS Analytical Laboratory",
             size=13, color=GOLD, italic=True, font=FONT_BODY)
    add_text(s, Inches(1.1), Inches(6.25), Inches(11), Inches(0.4),
             "brian.mulhall@csanalytical.com",
             size=13, color=IVORY, font=FONT_BODY)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    prs = Presentation()
    prs.slide_width = SLIDE_W
    prs.slide_height = SLIDE_H

    total = 10

    slide_cover(prs)
    slide_proposition(prs, 2, total)
    slide_company(prs, 3, total)
    slide_domain(prs, 4, total)
    slide_signal_density(prs, 5, total)
    slide_agents(prs, 6, total)
    slide_pilot(prs, 7, total)
    slide_roi(prs, 8, total)
    slide_what_we_bring(prs, 9, total)
    slide_risks(prs, 10, total)
    slide_closing(prs)

    OUT_PPTX.parent.mkdir(parents=True, exist_ok=True)
    prs.save(OUT_PPTX)
    print(f"wrote {OUT_PPTX}")


if __name__ == "__main__":
    main()
