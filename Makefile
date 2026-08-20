all: index.html tutorial/index.html

index.html: README.md gh-alerts.html
	pandoc README.md \
	  --from gfm+alerts --to html5 \
	  --include-in-header=gh-alerts.html \
	  --metadata pagetitle="Prela" \
	  --mathjax \
	  -s -o index.html

tutorial/index.html: tutorial/index.md
	pandoc tutorial/index.md \
	  --from gfm+alerts --to html5 \
	  --metadata pagetitle="Prela Tutorial" \
	  -s -o tutorial/index.html
