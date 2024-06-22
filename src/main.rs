use std::error::Error;
use std::fmt::Display;
use std::io;
use std::num::ParseFloatError;
use std::ops::{Bound, Range, RangeBounds};
use std::path::PathBuf;
use std::str::FromStr;

use clap::Parser;

use mzdata;
use mzdata::io::MZFileReader;
use mzdata::prelude::*;
#[allow(unused)]
use mzdata::spectrum::{SignalContinuity, SpectrumLike};
use mzsvg::SpectrumSVG;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MZRange {
    pub start: Option<f64>,
    pub end: Option<f64>,
}

impl Display for MZRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let start = self.start.map(|s| s.to_string()).unwrap_or_default();
        let end = self.end.map(|s| s.to_string()).unwrap_or_default();
        write!(f, "{start}-{end}")
    }
}

impl MZRange {
    pub fn new(start: Option<f64>, end: Option<f64>) -> Self {
        Self { start, end }
    }
}

impl Default for MZRange {
    fn default() -> Self {
        Self {
            start: None,
            end: None,
        }
    }
}

#[derive(Debug)]
pub enum MZRangeParseError {
    MalformedStart(ParseFloatError),
    MalformedEnd(ParseFloatError),
}

impl Display for MZRangeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MZRangeParseError::MalformedStart(e) => {
                write!(f, "Failed to parse range start {e}")
            }
            MZRangeParseError::MalformedEnd(e) => {
                write!(f, "Failed to parse range end {e}")
            }
        }
    }
}

impl Error for MZRangeParseError {}

impl FromStr for MZRange {
    type Err = MZRangeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = if s.contains(' ') {
            s.split(' ')
        } else if s.contains(':') {
            s.split(':')
        } else if s.contains('-') {
            s.split('-')
        } else {
            s.split(' ')
        };
        let start_s = tokens.next().unwrap();
        let start_t = if start_s == "" {
            None
        } else {
            match start_s.parse() {
                Ok(val) => Some(val),
                Err(e) => return Err(MZRangeParseError::MalformedStart(e)),
            }
        };
        let end_s = tokens.next().unwrap();
        let end_t = if end_s == "" {
            None
        } else {
            match end_s.parse() {
                Ok(val) => Some(val),
                Err(e) => return Err(MZRangeParseError::MalformedEnd(e)),
            }
        };
        Ok(MZRange {
            start: start_t,
            end: end_t,
        })
    }
}

impl From<Range<f64>> for MZRange {
    fn from(value: Range<f64>) -> Self {
        Self::new(Some(value.start), Some(value.end))
    }
}

impl RangeBounds<f64> for MZRange {
    fn start_bound(&self) -> Bound<&f64> {
        if let Some(start) = self.start.as_ref() {
            Bound::Included(start)
        } else {
            Bound::Unbounded
        }
    }

    fn end_bound(&self) -> Bound<&f64> {
        if let Some(end) = self.end.as_ref() {
            Bound::Included(end)
        } else {
            Bound::Unbounded
        }
    }
}

impl From<(f64, f64)> for MZRange {
    fn from(value: (f64, f64)) -> Self {
        Self::new(Some(value.0), Some(value.1))
    }
}

#[derive(Debug, Clone, Copy)]
struct Dimensions(usize, usize);

impl Default for Dimensions {
    fn default() -> Self {
        Self(600, 200)
    }
}

impl Display for Dimensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.0, self.1)
    }
}

fn parse_dimensions(dims: &str) -> Result<Dimensions, std::num::ParseIntError> {
    let r: Result<Vec<usize>, std::num::ParseIntError> = dims.split("x").map(|s| s.parse::<usize>()).collect();
    let r = r?;

    let width = r[0];
    let height = r[1];
    Ok(Dimensions(width, height))
}

#[derive(Parser, Default, Debug)]
struct App {
    #[arg(help = "Path to MS data file to draw")]
    path: PathBuf,

    #[arg(short = 's', long = "scan-number")]
    scan_number: usize,

    #[arg(short='m', long="mz-range", value_parser=MZRange::from_str, value_name="BEGIN-END", default_value_t=MZRange::default())]
    mz_range: MZRange,

    #[arg(short='d', long="dimensions", default_value_t=Dimensions(600, 200), value_parser=parse_dimensions)]
    dimensions: Dimensions,

    #[arg(short = 'a', long = "aspect-ratio")]
    aspect_ratio: Option<f64>,

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
        eprintln!("{} @ MS{}", spectrum.id(), spectrum.ms_level());
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
        document.save(&"image.svg")?;

        #[cfg(feature = "pdf")]
        if args.pdf {
            document.save_pdf(&"image.pdf")?;
        }
        #[cfg(not(feature = "pdf"))]
        if args.pdf {
            eprintln!("Cannot generate PDF file from SVG. Enable the `pdf` feature.")
        }

        #[cfg(feature = "png")]
        if args.png {
            document.save_png(&"image.png")?;
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
