"""Charts for the CS Analytical Final Package.

Six charts in total. Three were carried from the prior package, three
are new for this round:

  chart_market.png           — pharma analytical testing + CCIT carve-out
  chart_competitive.png      — depth × modernity competitive map
  chart_roi.png              — three revenue paths over 36 months
  chart_container_timeline.png  (NEW) — container-format launches vs. regs
  chart_agentic_adoption.png (NEW) — agentic-AI adoption curve at scale
  chart_calendar_recovery.png(NEW) — founder/scientist hours before/after

All composite midpoints of publicly cited industry sources, May 2026.
"""

from __future__ import annotations

from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.patches import FancyBboxPatch
import numpy as np


IVORY = "#FAF7F2"
NAVY = "#0B2545"
BURGUNDY = "#6E1B26"
CHARCOAL = "#1C1C1C"
SLATE = "#555B66"
RULE = "#C9BEA8"
GOLD = "#B58B3A"
TEAL = "#2F5D62"


def _style_axes(ax):
    for sp in ("top", "right"):
        ax.spines[sp].set_visible(False)
    for sp in ("left", "bottom"):
        ax.spines[sp].set_color(RULE)
    ax.tick_params(colors=SLATE, labelsize=9)
    ax.grid(axis="y", color="#E6DFCF", linewidth=0.6)
    ax.set_axisbelow(True)


# ---------------------------------------------------------------------------
# Chart 1 — Market (carried)
# ---------------------------------------------------------------------------

def build_market_chart(out: Path):
    years = np.array([2024, 2025, 2026, 2027, 2028, 2029, 2030])
    pat = np.array([8.7, 9.5, 10.4, 11.3, 12.3, 13.4, 14.6])
    ccit = np.array([1.37, 1.50, 1.64, 1.80, 1.97, 2.16, 2.36])

    fig, ax = plt.subplots(figsize=(8.4, 4.6), dpi=200)
    fig.patch.set_facecolor(IVORY)
    ax.set_facecolor(IVORY)

    ax.plot(years, pat, color=NAVY, linewidth=2.4, marker="o",
            markersize=5, label="Pharma analytical testing outsourcing  ($B)")
    ax.plot(years, ccit, color=BURGUNDY, linewidth=2.4, marker="o",
            markersize=5, label="CCIT services carve-out  ($B)")

    _style_axes(ax)
    ax.set_xticks(years)
    ax.legend(loc="upper left", frameon=False, fontsize=9,
              labelcolor=CHARCOAL)
    ax.set_title(
        "Pharma analytical testing  vs.  CCIT carve-out, 2024 — 2030",
        fontsize=12, color=NAVY, loc="left", pad=14, family="serif")

    for x, y in zip(years, pat):
        ax.annotate(f"${y:.1f}B", (x, y), textcoords="offset points",
                    xytext=(0, 9), ha="center", fontsize=8, color=NAVY)
    for x, y in zip(years, ccit):
        ax.annotate(f"${y:.2f}B", (x, y), textcoords="offset points",
                    xytext=(0, -15), ha="center", fontsize=8, color=BURGUNDY)

    plt.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


# ---------------------------------------------------------------------------
# Chart 2 — Competitive map (carried)
# ---------------------------------------------------------------------------

def build_competitive_map(out: Path):
    players = [
        ("Eurofins",          2.3, 4.8, 1900),
        ("SGS",               2.6, 4.5, 1500),
        ("Charles River",     3.1, 5.0, 1300),
        ("WuXi AppTec",       2.8, 5.2, 1200),
        ("Pace Analytical",   2.0, 4.0, 700),
        ("Intertek",          2.4, 4.4, 750),
        ("Element",           1.9, 4.2, 800),
        ("Nelson Labs",       6.5, 5.4, 480),
        ("Boston Analytical", 5.9, 4.8, 200),
        ("West Pharma Labs",  6.8, 5.0, 350),
        ("CS Analytical",     8.6, 7.6, 240),
    ]

    fig, ax = plt.subplots(figsize=(8.6, 5.6), dpi=200)
    fig.patch.set_facecolor(IVORY)
    ax.set_facecolor(IVORY)

    for name, x, y, r in players:
        is_us = name == "CS Analytical"
        color = BURGUNDY if is_us else NAVY
        alpha = 1.0 if is_us else 0.45
        ax.scatter([x], [y], s=r * 0.55, color=color, alpha=alpha,
                   edgecolor=IVORY, linewidth=1.2)
        dx, dy = 0.15, 0.18
        if is_us:
            dx, dy = 0.2, 0.28
        ax.annotate(name, (x, y), xytext=(x + dx, y + dy),
                    fontsize=9 if not is_us else 11,
                    color=CHARCOAL if not is_us else BURGUNDY,
                    family="serif",
                    weight="regular" if not is_us else "bold")

    ax.set_xlim(0, 10)
    ax.set_ylim(2, 10)
    ax.set_xlabel("Depth in CCIT  →", fontsize=10, color=SLATE,
                  family="serif")
    ax.set_ylabel("Modernity, agentic readiness  →",
                  fontsize=10, color=SLATE, family="serif")

    ax.axhline(6.5, color=RULE, linewidth=0.7, linestyle="--")
    ax.axvline(5.5, color=RULE, linewidth=0.7, linestyle="--")

    _style_axes(ax)

    ax.text(7.8, 9.6, "Deep  ·  Modern", color=BURGUNDY,
            fontsize=10, family="serif", style="italic")
    ax.text(0.3, 9.6, "Broad  ·  Modern", color=SLATE,
            fontsize=9, family="serif", style="italic")
    ax.text(0.3, 2.4, "Broad  ·  Legacy", color=SLATE,
            fontsize=9, family="serif", style="italic")
    ax.text(7.8, 2.4, "Deep  ·  Legacy", color=SLATE,
            fontsize=9, family="serif", style="italic")

    ax.set_title(
        "CCIT competitive map — depth × modernity",
        fontsize=12, color=NAVY, loc="left", pad=12, family="serif")

    plt.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


# ---------------------------------------------------------------------------
# Chart 3 — ROI shape (carried)
# ---------------------------------------------------------------------------

def build_roi_chart(out: Path):
    scenarios = ["Status quo", "Organic plan only", "Organic + agentic"]
    revenue = [1.00, 1.55, 2.40]

    fig, ax = plt.subplots(figsize=(8.4, 4.0), dpi=200)
    fig.patch.set_facecolor(IVORY)
    ax.set_facecolor(IVORY)

    colors = [SLATE, NAVY, BURGUNDY]
    bars = ax.barh(scenarios, revenue, color=colors, height=0.55)
    for bar, val in zip(bars, revenue):
        ax.text(val + 0.04, bar.get_y() + bar.get_height() / 2,
                f"{val:.2f}×", va="center", fontsize=11,
                color=CHARCOAL, family="serif")

    _style_axes(ax)
    ax.set_xlim(0, 2.9)
    ax.set_xlabel("Revenue at month 36, as multiple of today",
                  fontsize=10, color=SLATE, family="serif")
    ax.set_title(
        "Three revenue paths over a 36-month horizon",
        fontsize=12, color=NAVY, loc="left", pad=12, family="serif")
    for label in ax.get_yticklabels():
        label.set_fontsize(10)
        label.set_color(CHARCOAL)
        label.set_family("serif")

    plt.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


# ---------------------------------------------------------------------------
# Chart 4 (NEW) — Container Technology Timeline
# ---------------------------------------------------------------------------

def build_container_timeline(out: Path):
    """Vertical timeline. Time flows top-to-bottom. Date column on the
    left, color-coded marker, event title and detail on the right.
    Cleaner than a horizontal layout when there are many events."""

    # Events in chronological order. (date, lane, title, detail)
    events = [
        ("Jan 2025",  "industry",  "SCHOTT TOPPAC infuse",
         "First polymer prefilled syringe with embedded RFID + tamper-evident cap."),
        ("Jan 2025",  "industry",  "Stevanato Nexa Flex",
         "COP/COC prefilled-syringe platform validated; biologic-friendly."),
        ("May 2025",  "industry",  "SCHOTT TOPPAC freeze",
         "Polymer syringe validated to maintain CCI at −180°C for cell/gene."),
        ("May 2025",  "industry",  "SCHOTT deep-cold syringes",
         "Prefilled syringes engineered for −100°C drug storage."),
        ("Aug 2025",  "regulatory","Annex 1 lyo pre-batch sterilization",
         "Manual-loaded lyophilizers require sterilization before every batch."),
        ("Dec 1 2025","regulatory","USP <382> official",
         "Elastomer functional suitability shifts to the drug manufacturer."),
        ("Dec 1 2025","regulatory","FDA deploys agentic AI",
         "All FDA employees — including inspectors — receive secure agentic AI."),
        ("Feb 2026",  "csa",       "CS Analytical: RM Analytical launch",
         "Sister brand for raw-material and excipient analytical testing."),
        ("Apr 2026",  "industry",  "Bonfiglioli IVB Flex",
         "First-of-its-kind vacuum-decay leak detection for IV-bag systems."),
        ("Apr 2026",  "csa",       "CS Analytical: Interphex 2026 showcase",
         "IV-bag CCIT, USP <382>, cell-therapy distribution testing."),
        ("May 2026",  "csa",       "CS Analytical: Clifton expansion",
         "Doubles laboratory footprint to support expanded service lines."),
    ]

    n = len(events)
    fig_h = 0.6 + 0.55 * n
    fig, ax = plt.subplots(figsize=(10.8, fig_h), dpi=200)
    fig.patch.set_facecolor(IVORY)
    ax.set_facecolor(IVORY)

    # Vertical spine (timeline)
    spine_x = 2.4
    ax.plot([spine_x, spine_x], [-0.5, n - 0.5], color=RULE,
            linewidth=1.0, zorder=1)

    color_map = {"industry": BURGUNDY, "regulatory": NAVY, "csa": GOLD}
    lane_label_map = {"industry": "INDUSTRY", "regulatory": "REGULATORY",
                      "csa": "CS ANALYTICAL"}

    for i, (date, lane, title, detail) in enumerate(events):
        y = n - 1 - i  # top to bottom
        color = color_map[lane]

        # Date on the left
        ax.text(spine_x - 0.35, y, date, fontsize=9.5, color=SLATE,
                family="serif", ha="right", va="center", style="italic")

        # Lane chip
        ax.text(spine_x - 1.95, y, lane_label_map[lane], fontsize=7.5,
                color=color, family="serif", ha="left", va="center",
                weight="bold", style="italic")

        # Marker
        ax.scatter([spine_x], [y], s=180, color=color, zorder=4,
                   edgecolor=IVORY, linewidth=2.0)

        # Title + detail on the right
        ax.text(spine_x + 0.25, y + 0.15, title, fontsize=10.5,
                color=NAVY, family="serif", va="bottom",
                weight="bold")
        ax.text(spine_x + 0.25, y - 0.15, detail, fontsize=9,
                color=CHARCOAL, family="serif", va="top")

    ax.set_xlim(0, 11)
    ax.set_ylim(-1, n + 0.5)
    ax.set_xticks([])
    ax.set_yticks([])
    for sp in ("top", "right", "left", "bottom"):
        ax.spines[sp].set_visible(False)

    ax.set_title(
        "The 2024 — 2026 wave:  container formats, regulation, and CS Analytical's moves",
        fontsize=12.5, color=NAVY, loc="left", pad=10, family="serif", x=0)

    plt.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


# ---------------------------------------------------------------------------
# Chart 5 (NEW) — Agentic-AI adoption curve
# ---------------------------------------------------------------------------

def build_agentic_adoption(out: Path):
    """Customer count over time + step events. ArisGlobal NavaX as the
    reference platform comparable for what TSI-Citadel proposes."""

    fig, ax = plt.subplots(figsize=(8.4, 4.6), dpi=200)
    fig.patch.set_facecolor(IVORY)
    ax.set_facecolor(IVORY)

    # x = months from Jan 2024
    points = [
        ("2024-Q2", 8,  1, "1st top-25 pharma\non NavaX"),
        ("2024-Q4", 14, 3, "3rd top-25\npharma adopted"),
        ("2025-Q2", 17, 6, "6th top-25 pharma\nadopted (Jun 2025)"),
        ("2025-Q4", 23, 9, "1M+ cases\nin production"),
        ("2026-Q1", 26, 13, "120% Y/Y\nbookings growth\nQ1 2026"),
        ("2026-Q2", 28, 17, "Top-20 pharma\nadopts Signals+\nDistribution Agents\nin 6 weeks"),
    ]

    xs = [p[1] for p in points]
    ys = [p[2] for p in points]
    ax.plot(xs, ys, color=BURGUNDY, linewidth=2.5)
    ax.scatter(xs, ys, color=BURGUNDY, s=70, zorder=4, edgecolor=IVORY,
               linewidth=1.5)

    for _, x, y, label in points:
        ax.annotate(label, (x, y), xytext=(8, 8),
                    textcoords="offset points", fontsize=8,
                    color=CHARCOAL, family="serif")

    # FDA agentic deployment marker
    ax.axvline(23, color=NAVY, linewidth=1.5, linestyle="--", alpha=0.7)
    ax.annotate("FDA deploys agentic AI\nto all employees\n(Dec 1, 2025)",
                (23, 0.5), xytext=(8, 0), textcoords="offset points",
                fontsize=8.5, color=NAVY, family="serif", style="italic")

    _style_axes(ax)
    ax.set_xlim(4, 32)
    ax.set_ylim(0, 20)
    ax.set_xticks([6, 12, 18, 24, 30])
    ax.set_xticklabels(["Q1 '24", "Q3 '24", "Q1 '25", "Q3 '25", "Q1 '26"])
    ax.set_ylabel("Cumulative enterprise adoption  (illustrative scale)",
                  fontsize=10, color=SLATE, family="serif")
    ax.set_title(
        "Agentic AI in pharma is no longer experimental",
        fontsize=12, color=NAVY, loc="left", pad=12, family="serif")

    plt.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


# ---------------------------------------------------------------------------
# Chart 6 (NEW) — Calendar recovery (before/after)
# ---------------------------------------------------------------------------

def build_calendar_recovery(out: Path):
    """Twin donut charts: founder + scientist hours by category, before
    and after the agentic layer."""

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(9.2, 5.2), dpi=200)
    fig.patch.set_facecolor(IVORY)
    for ax in (ax1, ax2):
        ax.set_facecolor(IVORY)

    labels = ["Science /\nmethods", "BD ops /\noutreach", "Regulatory\nmonitoring",
              "Proposal /\nquote drafting", "Admin /\ninternal"]
    before = [35, 22, 12, 16, 15]
    after  = [62, 10, 6, 8, 14]
    colors = [BURGUNDY, NAVY, GOLD, TEAL, SLATE]

    def _draw(ax, sizes, title):
        wedges, _ = ax.pie(sizes, colors=colors, startangle=90,
                           wedgeprops=dict(width=0.35, edgecolor=IVORY,
                                           linewidth=2.2))
        ax.set_title(title, fontsize=11, color=NAVY, family="serif",
                     pad=18)
        for w, label, val in zip(wedges, labels, sizes):
            ang = (w.theta1 + w.theta2) / 2
            x = 0.83 * np.cos(np.deg2rad(ang))
            y = 0.83 * np.sin(np.deg2rad(ang))
            ax.annotate(f"{label}\n{val}%", (x, y), ha="center",
                        va="center", fontsize=8, color=CHARCOAL,
                        family="serif")

    _draw(ax1, before, "Today — founder & scientist hours")
    _draw(ax2, after,  "With the agentic layer")

    fig.suptitle(
        "Calendar recovery — where the agentic layer returns time",
        fontsize=13, color=NAVY, x=0.06, ha="left", family="serif", y=0.985)

    plt.subplots_adjust(top=0.85, bottom=0.05, left=0.04, right=0.96, wspace=0.1)
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    here = Path(__file__).resolve().parent
    out_charts = here.parent / "05_Charts"
    build_dir = here

    # The deck builders look up charts from the _build dir; we also
    # publish copies into 05_Charts/ as the user-facing deliverable.
    pairs = [
        ("chart_market.png",            build_market_chart),
        ("chart_competitive.png",       build_competitive_map),
        ("chart_roi.png",               build_roi_chart),
        ("chart_container_timeline.png", build_container_timeline),
        ("chart_agentic_adoption.png",  build_agentic_adoption),
        ("chart_calendar_recovery.png", build_calendar_recovery),
    ]
    for name, fn in pairs:
        fn(build_dir / name)
        fn(out_charts / name)
    print(f"wrote {len(pairs)} charts to {build_dir} and {out_charts}")
