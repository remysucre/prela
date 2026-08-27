PANDOC_OPTIONS := -V mainfont=palatino,serif \
                   --include-in-header=gh-alerts.html \
                   --include-in-header=header.html \
                   -s

all: index.html tutorial/index.html

index.html: README.md gh-alerts.html header.html
	pandoc README.md \
	  --from gfm+alerts --to html5 \
	  $(PANDOC_OPTIONS) \
	  --metadata pagetitle="Prela" \
	  --mathjax \
	  -o index.html

tutorial/index.html: tutorial/index.md gh-alerts.html header.html
	pandoc tutorial/index.md \
	  --from gfm+alerts+fenced_divs --to html5 \
	  $(PANDOC_OPTIONS) \
	  --metadata pagetitle="Prela Tutorial" \
	  -o tutorial/index.html
