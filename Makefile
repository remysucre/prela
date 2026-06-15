index.html: README.md gh-alerts.html
	pandoc README.md \
	  --from gfm+alerts --to html5 \
	  --include-in-header=gh-alerts.html \
	  --metadata pagetitle="Prela" \
	  --mathjax \
	  -s -o index.html
