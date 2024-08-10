mod chart;
mod chart_regions;
mod series;

pub use chart::{SpectrumSVG, FeatureSVG};
pub use chart_regions::{AxisOrientation, AxisProps, AxisTickLabelStyle, Canvas, TextProps};
pub use series::{
    peaks_to_arrays, AsSeries, CentroidSeries, ContinuousSeries, DeconvolutedCentroidSeries,
    LineSeries, PlotSeries, SeriesDescription, AnnotationSeries, TraceSeries, ColorCycle,
    ScatterSeries, DEFAULT_COLOR_CYCLE, PrecursorSeries
};
