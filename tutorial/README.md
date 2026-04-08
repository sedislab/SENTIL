# SENTIL tutorial

The long-form tutorial, in LaTeX.

## Build

With a TeX Live installation:

```sh
make
```

That runs `latexmk -xelatex` and writes `sentil-tutorial.pdf`. Without latexmk, two passes work:

```sh
xelatex sentil-tutorial.tex && xelatex sentil-tutorial.tex
```

The preamble loads `fontspec` for the Inter and JetBrains Mono faces under `fonts/`, which needs XeTeX or LuaTeX; `lualatex` works in place of `xelatex`, and pdfTeX does not.

## Layout

`sentil-tutorial.tex` is the master file; it pulls in `preamble.tex` (fonts, palette, code and figure styles) and the chapters under `ch/`.

## Dependencies

Standard TeX Live packages: `newpxtext`/`newpxmath`, `tikz`, `pgfplots`, `listings`, `tcolorbox`, `titlesec`, `adjustbox`, `microtype`, `hyperref`.