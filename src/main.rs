use std::io;
use std::path::PathBuf;

use clap::Parser;

use mzdata;
use mzdata::io::MZFileReader;
use mzdata::prelude::*;
#[allow(unused)]
use mzdata::spectrum::{SignalContinuity, SpectrumLike};
use mzsvg::SpectrumSVG;

use mzsvg::util::{MZRange, Dimensions};


#[derive(Parser, Default, Debug)]
struct App {
    #[arg(help = "Path to MS data file to draw")]
    path: PathBuf,

    #[arg(short = 's', long = "scan-number")]
    scan_number: usize,

    #[arg(short='m', long="mz-range", value_name="BEGIN-END", default_value_t=MZRange::default())]
    mz_range: MZRange,

    #[arg(short='d', long="dimensions", default_value_t=Dimensions(600, 200))]
    dimensions: Dimensions,

    #[arg(short='o', long="output-path", default_value="image.svg", help="Where to save the image to.")]
    output_path: String,

    #[arg(short = 'r', long = "reprofile", default_value_t = false)]
    reprofile: bool,

    #[arg(long = "pdf", default_value_t = false)]
    pdf: bool,

    #[arg(long = "png", default_value_t = false)]
    png: bool,
}

fn main() -> io::Result<()> {
    let args = App::parse();

    let path = args.path;
    let scan_index = args.scan_number;

    let mut document = SpectrumSVG::with_size(args.dimensions.0, args.dimensions.1);

    let mut reader = mzdata::MZReader::open_path(path)?;
    if let Some(mut spectrum) = reader.get_spectrum_by_index(scan_index) {
        let _has_deconv = spectrum.try_build_deconvoluted_centroids().is_ok();
        let has_centroid = spectrum.try_build_centroids().is_ok();
        document.axes_from(&spectrum).xlim(args.mz_range);
        document.draw_spectrum(&spectrum);

        if has_centroid
            && spectrum.signal_continuity() == SignalContinuity::Centroid
            && args.reprofile
        {
            if let Ok(()) = spectrum.reprofile_with_shape(0.0025, 0.025) {
                document.draw_profile(spectrum.arrays.as_ref().unwrap());
            }
        }
        document.finish();

        let output_path = PathBuf::from(args.output_path);
        document.save(&output_path.with_extension("svg"))?;

        #[cfg(feature = "pdf")]
        if args.pdf {
            document.save_pdf(output_path.with_extension("pdf"))?;
        }
        #[cfg(not(feature = "pdf"))]
        if args.pdf {
            eprintln!("Cannot generate PDF file from SVG. Enable the `pdf` feature.")
        }

        #[cfg(feature = "png")]
        if args.png {
            document.save_png(output_path.with_extension("png"))?;
        }
        #[cfg(not(feature = "png"))]
        if args.png {
            eprintln!("Cannot generate PNG file from SVG. Enable the `png` feature.")
        }
    } else {
        panic!("Failed to find spectrum {scan_index}");
    }

    Ok(())
}
