use std::io;
use std::path::PathBuf;

use clap::Parser;

use mzdata;
use mzdata::prelude::*;
use mzdata::spectrum::{SignalContinuity, SpectrumLike};

use mzsvg::util::{Dimensions, MZRange};
use mzsvg::{v2::AxisTickLabelStyle, SpectrumSVG};

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

    #[arg(
        short = 'o',
        long = "output-path",
        default_value = "image.svg",
        help = "Where to save the image to."
    )]
    output_path: String,

    #[arg(
        short = 'r',
        long = "reprofile",
        default_value_t = false,
        help = "Reprofile spectra which are centroided"
    )]
    reprofile: bool,

    #[arg(long, help = "Apply noise reduction with this scale")]
    denoise: Option<f32>,

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
    let spectrum = mzdata::mz_read!(path.as_path(), reader => {
        reader.get_spectrum_by_index(scan_index)
    })?;

    if let Some(mut spectrum) = spectrum {
        let _has_deconv = spectrum.try_build_deconvoluted_centroids().is_ok();
        let has_centroid = spectrum.try_build_centroids().is_ok();

        if spectrum.signal_continuity() == SignalContinuity::Profile {
            if let Some(scale) = args.denoise {
                spectrum.denoise(scale).unwrap();
            }
        }

        let peaks = spectrum.peaks();
        let (raw_start_mz, raw_end_mz) = peaks.mz_range();
        let raw_base_peak_intensity = peaks.base_peak().intensity;
        let start_mz = args.mz_range.start.unwrap_or(raw_start_mz);
        let end_mz = args.mz_range.end.unwrap_or(raw_end_mz);

        let ymax_in_range = peaks
            .iter()
            .filter(|p| start_mz <= p.mz && p.mz <= end_mz)
            .map(|p| p.intensity)
            .reduce(|a, b| a.max(b))
            .unwrap_or_default();

        document.axes_from(&spectrum).xlim(args.mz_range);
        document.yticks.tick_format = AxisTickLabelStyle::Percentile {
            precision: 2,
            maximum: Some(raw_base_peak_intensity as f64),
        };

        if args.mz_range.start.is_some() || args.mz_range.end.is_some() {
            document.ylim(0.0..ymax_in_range);
        }

        document.draw_spectrum(&spectrum);

        if has_centroid
            && spectrum.signal_continuity() == SignalContinuity::Centroid
            && args.reprofile
        {
            if let Ok(()) = spectrum.reprofile_with_shape(0.001, 0.025) {
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
