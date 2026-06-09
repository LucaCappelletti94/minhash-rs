# /// script
# requires-python = ">=3.9"
# dependencies = ["pandas", "matplotlib"]
# ///
"""Regenerate the benchmark figures from the committed CSV results.

This replaces the plotting cells of the old Jupyter notebook. It reads the
gzipped result tables in ``tests/`` and writes the JPG figures referenced by
``BENCHMARKS.md`` into ``figures/``.

Run it (no manual environment setup needed) with:

    uv run benchmarks/plot.py

or, with a classic toolchain:

    pip install pandas matplotlib && python benchmarks/plot.py
"""

import math
import os

import matplotlib.pyplot as plt
import pandas as pd

FIGURES_DIR = "figures"


def load_results():
    """Load the MinHash and HyperLogLog result tables, aggregated over runs."""
    minhash = pd.read_csv("tests/test_minhash_jaccard_large.csv.gz")
    minhash["mse"] = (minhash["approximation"] - minhash["ground_truth"]) ** 2
    minhash_across_all = (
        minhash.groupby(["permutations", "word", "memory"])
        .agg(["mean", "std", "count"])
        .reset_index()
    )
    minhash = (
        minhash.groupby(["elements", "permutations", "word", "memory"])
        .agg(["mean", "std", "count"])
        .reset_index()
    )

    hll = pd.read_csv("tests/test_hll_jaccard_large.csv.gz")
    hll["mse"] = (hll["approximation"] - hll["ground_truth"]) ** 2
    hll_across_all = (
        hll.groupby(["bits", "memory"]).agg(["mean", "std", "count"]).reset_index()
    )
    hll = (
        hll.groupby(["elements", "bits", "memory"])
        .agg(["mean", "std", "count"])
        .reset_index()
    )
    return minhash, minhash_across_all, hll, hll_across_all


def _grid(n):
    plots_per_row = 4
    rows = math.ceil(n / plots_per_row)
    fig, axes = plt.subplots(
        figsize=(plots_per_row * 7, 5 * rows),
        nrows=rows,
        ncols=plots_per_row,
        dpi=200,
    )
    return fig, axes


def _save(name):
    os.makedirs(FIGURES_DIR, exist_ok=True)
    plt.tight_layout()
    plt.savefig(os.path.join(FIGURES_DIR, name), bbox_inches="tight")
    plt.close()


def plot_mse_per_memory(minhash, hll):
    fig, axes = _grid(len(hll.elements.unique()))
    for elements, ax in zip(hll.elements.unique(), axes.flatten()):
        for word in minhash.word.unique():
            f = minhash[(minhash.word == word) & (minhash.elements == elements)].sort_values("memory")
            ax.plot(f.memory, f["mse"]["mean"], ls="--", label=f"MinHash {word}")
        for bits in hll.bits.unique():
            f = hll[(hll.bits == bits) & (hll.elements == elements)].sort_values("memory")
            ax.plot(f.memory, f["mse"]["mean"], label=f"HLL {bits}")
        ax.set_title(f"Universe of {elements} elements")
        ax.set_xlabel("Memory (bits, log scale)")
        ax.set_ylabel("MSE (log scale)")
        ax.legend(ncol=3)
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.grid()
    _save("minhash_hll_jaccard_MSE_memory.jpg")


def plot_mse_per_time(minhash, hll):
    fig, axes = _grid(len(hll.elements.unique()))
    for elements, ax in zip(hll.elements.unique(), axes.flatten()):
        for word in minhash.word.unique():
            # Drop time == 0 rows: they are below the measurement resolution, not truly zero.
            f = minhash[
                (minhash.word == word)
                & (minhash.elements == elements)
                & (minhash[("time", "mean")] > 0)
            ].sort_values(("time", "mean"))
            ax.plot(f.time["mean"], f["mse"]["mean"], ls="--", label=f"MinHash {word}")
        for bits in hll.bits.unique():
            f = hll[
                (hll.bits == bits)
                & (hll.elements == elements)
                & (hll[("time", "mean")] > 0)
            ].sort_values(("time", "mean"))
            ax.plot(f.time["mean"], f["mse"]["mean"], label=f"HLL {bits}")
        ax.set_title(f"Universe of {elements} elements")
        ax.set_xlabel("Time (microseconds, log scale)")
        ax.set_ylabel("MSE (log scale)")
        ax.legend(ncol=3)
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.grid()
    _save("minhash_hll_jaccard_MSE_time.jpg")


def plot_mse_times_time_per_memory(minhash, hll):
    fig, axes = _grid(len(hll.elements.unique()))
    flat_axes = iter(axes.flatten())
    for elements, ax in zip(hll.elements.unique(), flat_axes):
        for word in minhash.word.unique():
            f = minhash[
                (minhash.word == word)
                & (minhash.elements == elements)
                & (minhash[("time", "mean")] > 0)
            ].sort_values("memory")
            ax.plot(
                f.memory,
                f["mse"]["mean"] * f.time["mean"],
                ls="--",
                label=f"MinHash {word}",
            )
        for bits in hll.bits.unique():
            f = hll[
                (hll.bits == bits)
                & (hll.elements == elements)
                & (hll[("time", "mean")] > 0)
            ].sort_values("memory")
            ax.plot(f.memory, f["mse"]["mean"] * f.time["mean"], label=f"HLL {bits}")
        ax.set_title(f"Universe of {elements} elements")
        ax.set_xlabel("Memory (bits, log scale)")
        ax.set_ylabel("MSE x time (log scale)")
        ax.legend(ncol=3)
        ax.set_xscale("log")
        ax.set_yscale("log")
        ax.grid()
    for ax in flat_axes:
        ax.axis("off")
    _save("minhash_hll_jaccard_MSE_x_time_and_memory.jpg")


def plot_average_mse_times_time_per_memory(minhash_across_all, hll_across_all):
    fig, axes = plt.subplots(figsize=(7, 5), dpi=200)
    for word in minhash_across_all.word.unique():
        f = minhash_across_all[
            (minhash_across_all.word == word)
            & (minhash_across_all[("time", "mean")] > 0)
        ].sort_values("memory")
        axes.plot(
            f.memory,
            f["mse"]["mean"] * f["time"]["mean"],
            ls="--",
            label=f"MinHash {word}",
        )
    for bits in hll_across_all.bits.unique():
        f = hll_across_all[
            (hll_across_all.bits == bits) & (hll_across_all[("time", "mean")] > 0)
        ].sort_values("memory")
        axes.plot(f.memory, f["mse"]["mean"] * f["time"]["mean"], label=f"HLL {bits}")
    axes.set_xlabel("Memory (bits, log scale)")
    axes.set_ylabel("MSE x time (log scale)")
    axes.legend(ncol=3)
    axes.set_xscale("log")
    axes.set_yscale("log")
    axes.grid(which="both")
    axes.set_title("MSE x time (microseconds) per memory (bits)")
    _save("minhash_hll_jaccard_average_MSE_x_time_per_memory.jpg")


def main():
    minhash, minhash_across_all, hll, hll_across_all = load_results()
    plot_mse_per_memory(minhash, hll)
    plot_mse_per_time(minhash, hll)
    plot_mse_times_time_per_memory(minhash, hll)
    plot_average_mse_times_time_per_memory(minhash_across_all, hll_across_all)
    print(f"Wrote 4 figures to {FIGURES_DIR}/")


if __name__ == "__main__":
    main()
