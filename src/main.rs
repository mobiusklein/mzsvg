use std::env;
use std::error::Error;
use std::fmt::Display;
use std::io;
use std::num::ParseFloatError;
use std::ops::{Bound, Range, RangeBounds};
use std::str::FromStr;

use mzdata;

#[allow(unused)]
use mzdata::spectrum::{SignalContinuity, SpectrumLike};
use mzsvg::SpectrumSVG;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MZRange {
    pub start: Option<f64>,
    pub end: Option<f64>,
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

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);

    let path = args.next().expect("Please pass an MS data file path");
    let scan_index: usize = args
        .next()
        .expect("Please pass a scan number")
        .parse::<usize>()
        .unwrap_or_else(|e| {
            panic!("Failed to parse scan number: {e}");
        });

    let mz_range = args
        .next()
        .map_or_else(MZRange::default, |x| x.parse().unwrap());

    let mut document = SpectrumSVG::default();

    if let Some(mz_scale_arg) = args.next() {
        if let Ok(mz_scale) = mz_scale_arg.parse() {
            document.mz_scale = mz_scale;
        }
    }

    if let Some(intensity_scale_arg) = args.next() {
        if let Ok(intensity_scale) = intensity_scale_arg.parse() {
            document.intensity_scale = intensity_scale;
        }
    }

    let mut reader = mzdata::io::open_file(path)?;
    if let Some(mut spectrum) = reader.get_spectrum_by_index(scan_index) {
        let _has_deconv = spectrum.try_build_deconvoluted_centroids().is_ok();
        let _has_centroid = spectrum.try_build_centroids().is_ok();
        eprintln!("{} @ MS{}", spectrum.id(), spectrum.ms_level());
        document.axes_from(&spectrum).xlim(mz_range);
        document.draw_spectrum(&spectrum);

        // if _has_centroid && spectrum.signal_continuity() == SignalContinuity::Centroid {
        //     if let Ok(()) = spectrum.reprofile_with_shape(0.005, 0.025) {
        //         eprintln!("Drawing reprofiled spectrum");
        //         document.draw_profile(spectrum.arrays.as_ref().unwrap());
        //     }
        // }
        document.finish();
        document.save(&"image.svg")?;
    } else {
        panic!("Failed to find spectrum {scan_index}");
    }

    Ok(())
}
