
all: png pdf

png:
    resvg --background white image.svg image.png

pdf:
    svg2pdf image.svg image.pdf