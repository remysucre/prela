index.html: README.md gh-alerts.html
	pandoc README.md \
	  --from gfm+alerts --to html5 \
	  --include-in-header=gh-alerts.html \
	  --metadata pagetitle="Prela" \
	  --mathjax \
	  -s -o index.html

guide/guide.html: guide/guide.md gh-alerts.html guide/codapi-head.html guide/codapi-body.html
	pandoc guide/guide.md \
	  --from gfm+alerts --to html5 \
	  --include-in-header=gh-alerts.html \
	  --include-in-header=guide/codapi-head.html \
	  --include-after-body=guide/codapi-body.html \
	  --metadata pagetitle="Prela User Guide" \
	  --mathjax \
	  -s -o guide/guide.html
