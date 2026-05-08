#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage:
  md2book-jp.sh -m <markdown_file> -t <title> -a <author> [-o <output_pdf>]

Options:
  -m  Path to the input Markdown file
  -t  Book title
  -a  Author name
  -o  Output PDF path (default: current working directory / <input>-tate.pdf)
  -h  Show this help
USAGE
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Error: required command not found: $1" >&2
    echo "Install prerequisites first:" >&2
    echo "  brew install --cask mactex" >&2
    echo "  brew install pandoc" >&2
    exit 1
  }
}

check_tex_packages() {
  local missing=0
  local pkg
  for pkg in ltjtbook.cls luatexja.sty luatexja-preset.sty; do
    if ! kpsewhich "$pkg" >/dev/null 2>&1; then
      echo "Error: TeX package not found: $pkg" >&2
      missing=1
    fi
  done
  if [ "$missing" -ne 0 ]; then
    echo "Your TeX environment is incomplete for Japanese vertical PDF generation." >&2
    echo "Install prerequisites first:" >&2
    echo "  brew install --cask mactex" >&2
    echo "  brew install pandoc" >&2
    exit 1
  fi
}

MD_PATH=""
BOOK_TITLE=""
AUTHOR_NAME=""
OUTPUT_PDF=""

while getopts ":m:t:a:o:h" opt; do
  case "$opt" in
    m) MD_PATH="$OPTARG" ;;
    t) BOOK_TITLE="$OPTARG" ;;
    a) AUTHOR_NAME="$OPTARG" ;;
    o) OUTPUT_PDF="$OPTARG" ;;
    h)
      usage
      exit 0
      ;;
    :)
      echo "Error: option -$OPTARG requires an argument." >&2
      usage
      exit 1
      ;;
    \?)
      echo "Error: invalid option -$OPTARG" >&2
      usage
      exit 1
      ;;
  esac
done

if [ -z "$MD_PATH" ] || [ -z "$BOOK_TITLE" ] || [ -z "$AUTHOR_NAME" ]; then
  echo "Error: -m, -t, and -a are all required." >&2
  usage
  exit 1
fi

MD_PATH="${MD_PATH/#\~/$HOME}"
OUTPUT_PDF="${OUTPUT_PDF/#\~/$HOME}"

if [ ! -f "$MD_PATH" ]; then
  echo "Error: Markdown file not found: $MD_PATH" >&2
  exit 1
fi

require_cmd pandoc
require_cmd lualatex
require_cmd kpsewhich
check_tex_packages

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TEMPLATE_PATH="$SCRIPT_DIR/template-tate.tex"

INPUT_BASE="$(basename "$MD_PATH")"
INPUT_STEM="${INPUT_BASE%.*}"

if [ -z "$OUTPUT_PDF" ]; then
  OUTPUT_PDF="$PWD/${INPUT_STEM}-tate.pdf"
fi

mkdir -p "$(dirname "$OUTPUT_PDF")"
OUTPUT_DIR="$(cd "$(dirname "$OUTPUT_PDF")" && pwd)"
OUTPUT_BASE="$(basename "$OUTPUT_PDF")"
OUTPUT_PDF="$OUTPUT_DIR/$OUTPUT_BASE"

cleanup() {
  rm -f "$TEMPLATE_PATH"
}
trap cleanup EXIT

cat > "$TEMPLATE_PATH" <<'EOF2'
\documentclass[a5paper,10pt,openany]{ltjtbook}

\usepackage{luatexja}
\usepackage[haranoaji]{luatexja-preset}
\usepackage{luatexja-otf}
\usepackage{luatexja-ruby}
\usepackage{geometry}
\geometry{
  paperwidth=148mm,
  paperheight=210mm,
  top=18mm,
  bottom=18mm,
  inner=18mm,
  outer=14mm,
  bindingoffset=0mm,
  includefoot
}

$if(title)$
\title{$title$}
$endif$

$if(author)$
\author{$for(author)$$author$$sep$ \and $endfor$}
$endif$

\date{}

\renewcommand{\kanjifamilydefault}{\mcdefault}
\setcounter{secnumdepth}{0}

\makeatletter
\def\ps@plain{%
  \let\@mkboth\@gobbletwo
  \let\@oddhead\@empty
  \let\@evenhead\@empty
  \def\@oddfoot{\hfil\thepage\hfil}%
  \def\@evenfoot{\hfil\thepage\hfil}%
}
\makeatother

\pagestyle{plain}

\begin{document}

$if(title)$
\maketitle
\clearpage
$endif$

$if(toc)$
\tableofcontents
\clearpage
$endif$

$body$

\end{document}
EOF2

if ! pandoc "$MD_PATH" \
  -o "$OUTPUT_PDF" \
  --from gfm+hard_line_breaks \
  --pdf-engine=lualatex \
  --template="$TEMPLATE_PATH" \
  --toc \
  -V title="$BOOK_TITLE" \
  -V author="$AUTHOR_NAME" \
  -M documentclass=ltjtbook \
  -M numbersections=false
then
  echo "Error: PDF conversion failed." >&2
  exit 1
fi

echo "PDF generated: $OUTPUT_PDF"
