index.html: README.md gh-alerts.html
	pandoc README.md \
	  --from gfm+alerts --to html5 \
	  --include-in-header=gh-alerts.html \
	  --mathjax \
	  -V title="Prela" \
	  -s -o index.html
