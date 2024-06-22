use std::{
    fmt::{Display, LowerExp},
    marker::PhantomData,
};

use mzpeaks::{
    CentroidLike, DeconvolutedCentroidLike, IntensityMeasurement, MZLocated, MZPeakSetType,
    MassPeakSetType,
};
use num_traits::Float;

use svg::{
    node::element::{path::Data as PathData, Group, Line, Path, Polyline, Text},
    Node,
};

use super::chart_regions::Canvas;

const DEFAULT_COLOR_CYCLE: &'static [&'static str] = &[
    "steelblue",
    "blueviolet",
    "midnightblue",
    "lightseagreen",
    "limegreen",
    "goldenrod",
    "firebrick",
    "crimson",
];

#[derive(Debug, Clone)]
pub struct ColorCycle {
    colors: Vec<String>,
    index: usize,
}

impl Default for ColorCycle {
    fn default() -> Self {
        Self {
            colors: DEFAULT_COLOR_CYCLE.iter().map(|s| s.to_string()).collect(),
            index: 0,
        }
    }
}

impl Iterator for ColorCycle {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index + 1 >= self.colors.len() {
            self.index = 0;
        }
        let value = self.colors.get(self.index).and_then(|s| Some(s.clone()));
        self.index += 1;
        value
    }
}


pub trait PlotSeries<X: Float + Display + LowerExp, Y: Display + Float + LowerExp> {
    fn description(&self) -> &SeriesDescription;
    fn to_svg(&self, xaxis: &Canvas<X, Y>) -> Group;

    fn slice_x(&mut self, start: X, end: X);
    fn slice_y(&mut self, start: Y, end: Y);
}

pub trait AsSeries<X: Float + Display + LowerExp, Y: Display + Float + LowerExp> {
    type Series: PlotSeries<X, Y>;

    fn as_series(&self) -> Self::Series;
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SeriesDescription {
    pub label: String,
    pub color: String,
}

impl SeriesDescription {
    pub fn new(label: String, color: String) -> Self {
        Self { label, color }
    }

    pub fn with_color(mut self, color: String) -> Self {
        self.color = color;
        self
    }
}

impl From<String> for SeriesDescription {
    fn from(value: String) -> Self {
        SeriesDescription::new(value, "black".to_string())
    }
}

impl From<&str> for SeriesDescription {
    fn from(value: &str) -> Self {
        SeriesDescription::new(value.to_string(), "black".to_string())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LineSeries<X: Float + Display + LowerExp, Y: Float + Display + LowerExp> {
    pub points: Vec<(X, Y)>,
    pub description: SeriesDescription,
}

impl<X: Float + Display + LowerExp, Y: Float + Display + LowerExp> LineSeries<X, Y> {
    pub fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        let min_mz = self
            .points
            .iter()
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap()
            .0;
        let max_intens = Y::from(100.0).unwrap();
        let mut line = Polyline::new();

        let path_data: Vec<_> = self
            .points
            .iter()
            .enumerate()
            .map(|(i, (mz, inten))| {
                format!(
                    "{},{}",
                    canvas.x_axis.scale.transform(*mz).to_f64().unwrap(),
                    canvas.y_axis.scale.transform(*inten).to_f64().unwrap(),
                )
            })
            .collect();
        let points = path_data.join(" ");

        let path = Polyline::new()
            .set("points", points)
            .set("fill", "none")
            .set("stroke", self.description.color.clone())
            .set("stroke-width", 1);
        let group = Group::new();
        group.add(path)
    }
}

impl<X: Float + Display + LowerExp, Y: Float + Display + LowerExp> PlotSeries<X, Y>
    for LineSeries<X, Y>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        self.to_svg(canvas)
    }

    fn slice_x(&mut self, start: X, end: X) {
        let points = self
            .points
            .iter()
            .copied()
            .filter(|(x, _)| (x >= &start) && (x <= &end))
            .collect();
        self.points = points;
    }

    fn slice_y(&mut self, start: Y, end: Y) {
        let points = self
            .points
            .iter()
            .copied()
            .filter(|(_, y)| (y >= &start) && (y <= &end))
            .collect();
        self.points = points;
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ContinuousSeries<X: Float + Display + LowerExp, Y: Float + Display + LowerExp> {
    pub points: Vec<(X, Y)>,
    pub description: SeriesDescription,
}

impl<X: Float + Display + LowerExp, Y: Float + Display + LowerExp> ContinuousSeries<X, Y> {
    pub fn new(points: Vec<(X, Y)>, description: SeriesDescription) -> Self {
        Self {
            points,
            description,
        }
    }

    pub fn from_iterators(
        xiter: impl Iterator<Item = X>,
        yiter: impl Iterator<Item = Y>,
        description: SeriesDescription,
    ) -> Self {
        Self {
            points: xiter.zip(yiter).collect(),
            description,
        }
    }

    pub fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        let min_mz = self
            .points
            .iter()
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .unwrap()
            .0;
        let max_intens = Y::from(100.0).unwrap();
        let path_data = self
            .points
            .iter()
            .enumerate()
            .fold(PathData::new(), |mut state, (i, (mz, inten))| {
                if i == 0 {
                    state = state.move_to((
                        canvas.x_axis.scale.transform(min_mz).to_f64().unwrap(),
                        canvas.y_axis.scale.transform(Y::zero()).to_f64().unwrap(),
                    ));
                }
                state.line_to((
                    canvas.x_axis.scale.transform(*mz).to_f64().unwrap(),
                    canvas.y_axis.scale.transform(*inten).to_f64().unwrap(),
                ))
            })
            .close();
        let path = Path::new()
            .set("fill", "none")
            .set("stroke", self.description.color.clone())
            .set("stroke-width", 1)
            .set("d", path_data);
        let group = Group::new();
        group.add(path).set("class", self.description.label.clone())
    }
}

impl<X: Float + Display + LowerExp, Y: Float + Display + LowerExp> PlotSeries<X, Y>
    for ContinuousSeries<X, Y>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        self.to_svg(canvas)
    }

    fn slice_x(&mut self, start: X, end: X) {
        let points = self
            .points
            .iter()
            .copied()
            .filter(|(x, _)| (x >= &start) && (x <= &end))
            .collect();
        self.points = points;
    }

    fn slice_y(&mut self, start: Y, end: Y) {
        let points = self
            .points
            .iter()
            .copied()
            .filter(|(_, y)| (y >= &start) && (y <= &end))
            .collect();
        self.points = points;
    }
}

mod mzdata_continuum {
    use mzdata::spectrum::BinaryArrayMap;

    use super::*;

    impl AsSeries<f64, f32> for BinaryArrayMap {
        type Series = ContinuousSeries<f64, f32>;

        fn as_series(&self) -> Self::Series {
            let mzs = self.mzs().unwrap();
            let intensities = self.intensities().unwrap();

            ContinuousSeries::from_iterators(
                mzs.iter().copied(),
                intensities.iter().copied(),
                "Profile".into(),
            )
        }
    }
}

pub fn peaks_to_arrays<
    'transient,
    'lifespan: 'transient,
    P: MZLocated + IntensityMeasurement + 'static,
    I: Iterator<Item = &'transient P>,
    X: Float,
    Y: Float,
>(
    peaks: I,
) -> Vec<(X, Y)> {
    let mut points: Vec<(X, Y)> = Vec::new();

    let xd = X::from(0.0001).unwrap();
    let yz = Y::zero();
    for peak in peaks {
        let mz = X::from(peak.mz()).unwrap();
        let intens = Y::from(peak.intensity()).unwrap();
        points.push((mz - xd, yz));
        points.push((mz, intens));
        points.push((mz + xd, yz));
    }
    points
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CentroidSeries<
    X: Float + Display + LowerExp,
    Y: Float + Display + LowerExp,
    T: CentroidLike + Clone + 'static,
> {
    pub peaks: MZPeakSetType<T>,
    pub description: SeriesDescription,
    _x: PhantomData<X>,
    _y: PhantomData<Y>,
}

impl<
        X: Float + Display + LowerExp,
        Y: Float + Display + LowerExp,
        T: CentroidLike + Clone + 'static,
    > PlotSeries<X, Y> for CentroidSeries<X, Y, T>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        self.to_svg(canvas)
    }

    fn slice_x(&mut self, start: X, end: X) {
        let points = self
            .peaks
            .iter()
            .filter(|p| (X::from(p.mz()).unwrap() >= start) && (X::from(p.mz()).unwrap() <= end))
            .cloned()
            .collect();
        self.peaks = points;
    }

    fn slice_y(&mut self, start: Y, end: Y) {
        let points = self
            .peaks
            .iter()
            .filter(|p| {
                (Y::from(p.intensity()).unwrap() >= start)
                    && (Y::from(p.intensity()).unwrap() <= end)
            })
            .cloned()
            .collect();
        self.peaks = points;
    }
}

impl<
        X: Float + Display + LowerExp,
        Y: Float + Display + LowerExp,
        T: CentroidLike + Clone + 'static,
    > CentroidSeries<X, Y, T>
{
    pub fn new(peaks: MZPeakSetType<T>, description: SeriesDescription) -> Self {
        Self {
            peaks,
            description,
            _x: PhantomData,
            _y: PhantomData,
        }
    }

    pub fn from_iterator(peaks: impl Iterator<Item = T>, description: SeriesDescription) -> Self {
        let peaks = peaks.collect();
        Self::new(peaks, description)
    }

    pub fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        let points = peaks_to_arrays(self.peaks.iter());
        let proxy = ContinuousSeries::new(points, self.description.clone());
        let group = proxy.to_svg(canvas);
        group
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DeconvolutedCentroidSeries<
    X: Float + Display + LowerExp,
    Y: Float + Display + LowerExp,
    T: DeconvolutedCentroidLike + Clone + MZLocated + 'static,
> {
    pub peaks: MassPeakSetType<T>,
    pub description: SeriesDescription,
    _x: PhantomData<X>,
    _y: PhantomData<Y>,
}

impl<
        X: Float + Display + LowerExp,
        Y: Float + Display + LowerExp,
        T: DeconvolutedCentroidLike + Clone + MZLocated + 'static,
    > PlotSeries<X, Y> for DeconvolutedCentroidSeries<X, Y, T>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        self.to_svg(canvas)
    }

    fn slice_x(&mut self, start: X, end: X) {
        let points = self
            .peaks
            .iter()
            .filter(|p| (X::from(p.mz()).unwrap() >= start) && (X::from(p.mz()).unwrap() <= end))
            .cloned()
            .collect();
        self.peaks = points;
    }

    fn slice_y(&mut self, start: Y, end: Y) {
        let points = self
            .peaks
            .iter()
            .filter(|p| {
                (Y::from(p.intensity()).unwrap() >= start)
                    && (Y::from(p.intensity()).unwrap() <= end)
            })
            .cloned()
            .collect();
        self.peaks = points;
    }
}

impl<
        X: Float + Display + LowerExp,
        Y: Float + Display + LowerExp,
        T: DeconvolutedCentroidLike + Clone + 'static + MZLocated,
    > DeconvolutedCentroidSeries<X, Y, T>
{
    pub fn new(peaks: MassPeakSetType<T>, description: SeriesDescription) -> Self {
        Self {
            peaks,
            description,
            _x: PhantomData,
            _y: PhantomData,
        }
    }

    pub fn from_iterator(peaks: impl Iterator<Item = T>, description: SeriesDescription) -> Self {
        let peaks = peaks.collect();
        Self::new(peaks, description)
    }

    pub fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        let mut peaks_sorted: Vec<_> = self.peaks.iter().cloned().collect();
        peaks_sorted.sort_by(|a, b| a.mz().total_cmp(&b.mz()));
        let points = peaks_to_arrays(peaks_sorted.iter());
        let proxy = ContinuousSeries::new(points, self.description.clone());
        let group = proxy.to_svg(canvas);
        group
    }
}

#[cfg(test)]
mod test {
    use crate::{
        v2::chart_regions::{AxisOrientation, AxisProps},
        CoordinateRange,
    };

    use super::*;

    #[test]
    fn test_polyline() {
        let mut canvas: Canvas<f64, f32> = Canvas::new(600, 200);
        canvas.update_scales(
            CoordinateRange::new(0.0, 1000.0),
            CoordinateRange::new(10000.0, 0.0),
        );

        let mut props: AxisProps<f64> = AxisProps::new(AxisOrientation::Bottom);
        props.tick_values = Some(vec![0.0, 200.0, 400.0, 600.0, 800.0, 1000.0]);
        props.label = Some("m/z".to_string());

        let mut props2: AxisProps<f32> = AxisProps::new(AxisOrientation::Left);
        props2.tick_values = Some(vec![10000.0, 7500.05, 5000.0, 2500.0, 0.0]);
        props2.label = Some("Intensity".to_string());

        let series = LineSeries {
            points: vec![(250.0, 7000.5), (350.0, 150.0), (571.0, 4000.0)],
            description: "test".into(),
        };

        canvas.groups.push(series.to_svg(&canvas));

        let doc = canvas.to_svg(&props, &props2);

        std::fs::write("test.svg", doc.to_string()).unwrap();
    }
}
