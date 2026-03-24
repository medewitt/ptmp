############################################################
# Project Makefile — generated via template copier
#
# Targets:
#   make pdf       - build the PDF with latexmk
#   make watch     - watch and rebuild on changes (latexmk -pvc)
#   make figures   - run all R/*.R scripts
#   make open      - open the generated PDF
#   make clean     - clean LaTeX aux files in output/
#   make distclean - clean plus remove output/* artifacts
#   make check     - print configuration
############################################################

# --- Configuration (Tera-injected) ---
MAIN       := {{ main_file }}
OUTDIR     := output
FIGDIR     := figures

# LaTeX engine selection: 'pdf', 'xelatex', or 'lualatex'
# (Collected via TUI; defaults to 'pdf')
ENGINE_FLAG := {% if latex_engine == "xelatex" %}-xelatex{% elif latex_engine == "lualatex" %}-lualatex{% else %}-pdf{% endif %}

LATEXMK   := latexmk
RSCRIPTS  := $(wildcard R/*.R)
PDF       := $(OUTDIR)/$(MAIN:.tex=.pdf)

# Detect platform for 'open'
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
    OPEN_CMD := open
else
    OPEN_CMD := xdg-open
endif

.PHONY: all pdf watch figures open clean distclean check ensure_dirs

all: pdf

ensure_dirs:
    @mkdir -p $(OUTDIR) $(FIGDIR)

# Build PDF with latexmk
pdf: ensure_dirs $(PDF)

$(PDF): $(MAIN)
    $(LATEXMK) $(ENGINE_FLAG) -interaction=nonstopmode -halt-on-error -file-line-error -outdir=$(OUTDIR) $(MAIN)

# Continuous preview (rebuild on file changes)
watch: ensure_dirs
    $(LATEXMK) $(ENGINE_FLAG) -pvc -interaction=nonstopmode -file-line-error -outdir=$(OUTDIR) $(MAIN)

# Run all R scripts in R/
figures: ensure_dirs
    @for f in $(RSCRIPTS); do \
        echo "Running $$f"; \
        Rscript "$$f"; \
    done

# Open the built PDF (macOS: open; Linux: xdg-open)
open: pdf
    $(OPEN_CMD) "$(PDF)"

# Clean LaTeX intermediate files from output/
clean:
    $(LATEXMK) -C -outdir=$(OUTDIR)
    @rm -f "$(PDF)"

# Deep clean all artifacts in output/
distclean: clean
    @rm -rf $(OUTDIR)/*

check:
    @echo "Main file:      $(MAIN)"
    @echo "Output dir:     $(OUTDIR)"
    @echo "Figures dir:    $(FIGDIR)"
    @echo "Engine flag:    $(ENGINE_FLAG)"
    @echo "R scripts:      $(RSCRIPTS)"
