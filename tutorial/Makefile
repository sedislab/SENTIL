MAIN = sentil-tutorial

.PHONY: all clean

all: $(MAIN).pdf

$(MAIN).pdf: $(MAIN).tex preamble.tex $(wildcard ch/*.tex)
	latexmk -xelatex -interaction=nonstopmode -halt-on-error $(MAIN).tex

clean:
	latexmk -C $(MAIN).tex
	rm -f $(MAIN).bbl $(MAIN).run.xml