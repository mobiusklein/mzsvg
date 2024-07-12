use clap::Parser;
use mzpeaks::{feature::SimpleFeature, Time, MZ};
use std::io;

use mzdata::{
    prelude::*,
    MzMLReader,
};

use mzsvg::{util::{Dimensions, MZRange}, AsSeries, FeatureSVG, PlotSeries};

#[derive(Debug, Parser)]
struct App {
    #[arg(short = 't', long)]
    time_range: Option<MZRange>,

    #[arg(short='d', long="dimensions", default_value_t=Dimensions(600, 200))]
    dimensions: Dimensions,

    #[arg()]
    path: String,
}

pub fn main() -> io::Result<()>  {
    let args = App::parse();
    let mut reader = MzMLReader::open_path(args.path)?;

    let tic = reader.get_chromatogram_by_index(1).unwrap();
    let inten = tic.intensity()?;
    let time = tic.time()?;
    let mut f: SimpleFeature<MZ, Time> = SimpleFeature::empty(0.0);
    for (t, i) in time.into_iter().zip(inten.into_iter()) {
        f.push_raw(0.0, *t, *i)
    }

    let part = f.slice(f.find_time(53.0).0.unwrap()..f.find_time(54.0).0.unwrap());

    let mut fig = FeatureSVG::with_size(args.dimensions.0, args.dimensions.1);

    fig.axes_from(&part);
    fig.xticks.set_label(Time::name());

    if let Some(range) = args.time_range {
        fig.xlim(range);
    }

    let mut series = f.as_series();
    *series.color_mut() = "#167ed9".to_string();

    fig.add_series(series);

    fig.save(&"feature.svg")?;

    Ok(())
}