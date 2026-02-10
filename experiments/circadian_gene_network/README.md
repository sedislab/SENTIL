# Case study: a circadian gene-regulatory network

A cell's circadian clock is a small gene network whose activator and repressor proteins rise and fall on a roughly 24-hour cycle, and the property that matters is not the level at any instant but whether the oscillation keeps going. That is a temporal statement, so a threshold alarm cannot express it while an STL formula can. This case study runs SENTIL over a reference ensemble of the Barkai-Leibler activator-repressor oscillator and checks that the network sustains its rhythm.

## The data

`circadian_traces.csv` holds the reference ensemble: 100 stochastic realizations of the activator protein population, sampled once per hour for 270 hours, plus their mean. The model is the two-gene activator-repressor oscillator of Barkai and Leibler; the protein count swings between about 400 at the troughs and about 6200 at the peaks with a period near 24 hours. This script reads the traces and checks them; it does not resimulate the network.

## The specifications

| Name | Formula | Meaning |
| --- | --- | --- |
| oscillation_peaks | `always[0,240] (eventually[0,24] (activator > 3000))` | a peak above 3000 recurs within every 24-hour window |
| oscillation_troughs | `always[0,240] (eventually[0,24] (activator < 2000))` | a trough below 2000 recurs within every window |
| within_capacity | `always (activator < 7000)` | the population stays within its ceiling |

The peaks and troughs together are what "keeps oscillating" means: the protein has to come back up in every window and back down in every window, so a signal that saturated high or collapsed to zero would fail one of them even though a single threshold check would pass.

## Result

SENTIL confirms the rhythm. On the ensemble mean the peaks recur with robustness about +2530 and the troughs with about +1040, both comfortably positive, and the measured period is 23.8 hours. Across the 100 realizations all 100 satisfy the joint oscillation property, so the empirical probability that the network oscillates is 1.0. A probabilistic check that lifts each reading by a single-cell measurement error and asks `P>=0.9(...)` that the peaks persist holds at probability 1.0, since the peak margin is far larger than the sensor noise. The plot shows thirty realizations, their mean, and the two thresholds.

## Run it

```
python experiments/circadian_gene_network/circadian_gene_network.py
```

## Reference

Barkai, N. and Leibler, S. Circadian clocks limited by noise. Nature 403, 267-268 (2000).