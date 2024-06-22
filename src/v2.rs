#![allow(unused)]
mod chart;
mod chart_regions;
mod series;

pub use chart::SpectrumSVG;
pub use chart_regions::{AxisOrientation, AxisProps, AxisTickLabelStyle, Canvas};
pub use series::{
    peaks_to_arrays, AsSeries, CentroidSeries, ContinuousSeries, DeconvolutedCentroidSeries,
    LineSeries, PlotSeries, SeriesDescription,
};
