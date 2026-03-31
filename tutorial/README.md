# SENTIL tutorial

The long-form tutorial, in LaTeX.

## Build

With a TeX Live installation:

```sh
make
```

That runs `latexmk` and writes `sentil-tutorial.pdf`. Without latexmk, two passes of any modern engine work:

```sh
pdflatex sentil-tutorial.tex && pdflatex sentil-tutorial.tex
```

It also builds with `xelatex` or `lualatex`.

## Layout

`sentil-tutorial.tex` is the master file; it pulls in `preamble.tex` (fonts, palette, code and figure styles) and the chapters under `ch/`.

## Dependencies

Standard TeX Live packages: `newpxtext`/`newpxmath`, `tikz`, `pgfplots`, `listings`, `tcolorbox`, `titlesec`, `adjustbox`, `microtype`, `hyperref`.