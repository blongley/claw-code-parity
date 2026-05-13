"""Charts for the CS Analytical Opportunity Brief.

Numbers are midpoint composites of publicly cited industry ranges
(precedenceresearch.com, marketresearchfuture.com, datahorizzonresearch.com,
roots analysis, BCG/McKinsey/IQVIA agentic-AI commentary). Captured 2026.
"""

from __future__ import annotations

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from pathlib import Path


IVORY = "#FAF7F2"
NAVY = "#0B2545"
BURGUNDY = "#6E1B26"
CHARCOAL = "#1C1C1C"
SLATE = "#555B66"
RULE = "#C9BEA8"


def _style_axes(ax):
    for sp in ("top", "right"):
        ax.spines[sp].set_visible(False)
    for sp in ("left", "bottom"):
        ax.spines[sp].set_color(RULE)
    ax.tick_params(colors=SLATE, labelsize=9)
    ax.grid(axis="y", color="#E6DFCF", linewidth=0.6)
    ax.set_axisbelow(True)


def build_market_chart(out: Path):
    """Two stacked lines: pharma analytical testing ($B) and CCIT ($B).

    Sources (midpoint composites):
      Pharma analytical testing outsourcing — $9.5B (2025), ~9% CAGR.
      CCIT services market — $1.5B (2025), ~9-10% CAGR.
    """
    years = np.array([2024, 2025, 2026, 2027, 2028, 2029, 2030])
    pat = np.array([8.7, 9.5, 10.4, 11.3, 12.3, 13.4, 14.6])    # $B
    ccit = np.array([1.37, 1.50, 1.64, 1.80, 1.97, 2.16, 2.36])  # $B

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
        fontsize=12, color=NAVY, loc="left", pad=14,
        family="serif",
    )

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


def build_competitive_map(out: Path):
    """CCIT competitive map: depth on x, modernity / agentic-readiness on y.

    Players included reflect publicly active CCIT-capable analytical labs
    as of 2026. CS Analytical highlighted in burgundy.
    """
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
        fontsize=12, color=NAVY, loc="left", pad=12, family="serif",
    )

    plt.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


def build_roi_chart(out: Path):
    """Bar chart of revenue uplift over a 36-month horizon — three scenarios."""
    scenarios = ["Status quo", "Organic plan only", "Organic + agentic"]
    revenue = [1.00, 1.55, 2.40]  # multiples of current run-rate at month 36

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
        fontsize=12, color=NAVY, loc="left", pad=12, family="serif",
    )
    for label in ax.get_yticklabels():
        label.set_fontsize(10)
        label.set_color(CHARCOAL)
        label.set_family("serif")

    plt.tight_layout()
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=200, facecolor=fig.get_facecolor())
    plt.close(fig)


if __name__ == "__main__":
    here = Path(__file__).resolve().parent
    build_market_chart(here / "chart_market.png")
    build_competitive_map(here / "chart_competitive.png")
    build_roi_chart(here / "chart_roi.png")
    print("wrote charts")
