use clap::Parser;
use std::io;

use mzdata::{
    prelude::*,
    spectrum::{average_spectra, SignalContinuity},
    MZReader,
};

use mzsvg::{util::{parse_dimensions, Dimensions, MZRange}, ContinuousSeries, SeriesDescription, SpectrumSVG};

#[derive(Debug, Parser)]
struct App {
    #[arg(short = 'm', long)]
    mz_range: Option<MZRange>,

    #[arg(short='d', long="dimensions", default_value_t=Dimensions(600, 200), value_parser=parse_dimensions)]
    dimensions: Dimensions,

    #[arg()]
    path: String,
    #[arg()]
    indices: Vec<usize>,
}

pub fn main() -> io::Result<()> {
    let args = App::parse();
    let mut reader = MZReader::open_path(&args.path)?;
    let spectra: Vec<_> = args
        .indices
        .iter()
        .copied()
        .flat_map(|i| reader.get_spectrum_by_index(i))
        .map(|mut s| {
            if s.signal_continuity() == SignalContinuity::Centroid {
                s.try_build_centroids().unwrap();
                s.reprofile_with_shape(0.001, 0.01).unwrap();
                s.description_mut().signal_continuity = SignalContinuity::Profile;
            }
            s
        })
        .collect();
    let mut fig = SpectrumSVG::with_size(1200, 800);

    for s in spectra.iter() {
        fig.axes_from(s);
    }

    if let Some(mz_range) = args.mz_range {
        fig.xlim(mz_range);
    }

    eprintln!("{:?}", fig.x_range.as_ref().unwrap());

    for s in spectra.iter() {
        eprintln!("Drawing {}", s.id());

        fig.draw_spectrum(s);
    }

    let avg = average_spectra(&spectra, 0.001);
    let series = ContinuousSeries::new(
        avg.mz_array
            .into_iter()
            .copied()
            .zip(avg.intensity_array.into_iter().copied())
            .collect(),
        SeriesDescription::new("average".into(), "red".into()),
    );
    fig.add_series(series);
    fig.custom_css = Some(r#"
.average {
    stroke-dasharray: 4 1;
}

.centroid {
    stroke-width: 0.5 !important;
}
    "#.to_string());

    fig.save(&"overlay.svg")?;
    fig.save_png(&"overlay.png")?;
    Ok(())
}
