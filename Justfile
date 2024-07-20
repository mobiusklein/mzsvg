
all: png pdf

png:
    resvg --background white image.svg image.png

pdf:
    svg2pdf image.svg image.pdf

changelog tag:
    git cliff -t {{tag}} -o CHANGELOG.md

release tag: (changelog tag)
    git add CHANGELOG.md
    git commit -m "chore: update changelog"
    git tag {{tag}}
    cargo publish
