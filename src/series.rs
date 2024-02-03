use std::{fmt::Display, marker::PhantomData};

use num_traits::Float;
use svg::node::element::{path::Data, Group, Path};

use mzpeaks::{prelude::*, CentroidLike, MZPeakSetType, MassPeakSetType};

use crate::axes::{XAxis, YAxis};

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

// fn slice_x(self, start: X, end: X) -> Self;
// fn slice_y(self, start: Y, end: Y) -> Self;
pub trait PlotSeries<X: Float + Display, Y: Display + Float> {
    fn description(&self) -> &SeriesDescription;
    fn to_svg(&self, xaxis: &XAxis<X>, yaxis: &YAxis<Y>) -> Group;

    fn slice_x(&mut self, start: X, end: X);
    fn slice_y(&mut self, start: Y, end: Y);
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

#[derive(Debug, Default, Clone, PartialEq)]
pub struct ContinuousSeries<X: Float + Display, Y: Float + Display> {
    pub points: Vec<(X, Y)>,
    pub description: SeriesDescription,
}

impl<X: Float + Display, Y: Float + Display> PlotSeries<X, Y> for ContinuousSeries<X, Y> {
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn to_svg(&self, xaxis: &XAxis<X>, yaxis: &YAxis<Y>) -> Group {
        self.to_svg(xaxis, yaxis)
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

impl<X: Float + Display, Y: Float + Display> ContinuousSeries<X, Y> {
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

    pub fn to_svg(&self, xaxis: &XAxis<X>, yaxis: &YAxis<Y>) -> Group {
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
            .fold(Data::new(), |mut state, (i, (mz, inten))| {
                if i == 0 {
                    state = state.move_to((xaxis.transform(min_mz), yaxis.transform(Y::zero())));
                }
                state.line_to((
                    xaxis.transform(*mz),
                    yaxis.transform((*inten / max_intens) * Y::from(100.0).unwrap()),
                ))
            })
            .close();
        let path = Path::new()
            .set("fill", "none")
            .set("stroke", self.description.color.clone())
            .set("stroke-width", 5)
            .set("d", path_data);
        let group = Group::new();
        group.add(path)
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
pub struct CentroidSeries<X: Float + Display, Y: Float + Display, T: CentroidLike + Clone + 'static>
{
    pub peaks: MZPeakSetType<T>,
    pub description: SeriesDescription,
    _x: PhantomData<X>,
    _y: PhantomData<Y>,
}

impl<X: Float + Display, Y: Float + Display, T: CentroidLike + Clone + 'static> PlotSeries<X, Y>
    for CentroidSeries<X, Y, T>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn to_svg(&self, xaxis: &XAxis<X>, yaxis: &YAxis<Y>) -> Group {
        self.to_svg(xaxis, yaxis)
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

impl<X: Float + Display, Y: Float + Display, T: CentroidLike + Clone + 'static>
    CentroidSeries<X, Y, T>
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

    pub fn to_svg(&self, xaxis: &XAxis<X>, yaxis: &YAxis<Y>) -> Group {
        let points = peaks_to_arrays(self.peaks.iter());
        let proxy = ContinuousSeries::new(points, self.description.clone());
        let group = proxy.to_svg(xaxis, yaxis);
        group
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DeconvolutedCentroidSeries<
    X: Float + Display,
    Y: Float + Display,
    T: DeconvolutedCentroidLike + Clone + MZLocated + 'static,
> {
    pub peaks: MassPeakSetType<T>,
    pub description: SeriesDescription,
    _x: PhantomData<X>,
    _y: PhantomData<Y>,
}

impl<
        X: Float + Display,
        Y: Float + Display,
        T: DeconvolutedCentroidLike + Clone + MZLocated + 'static,
    > PlotSeries<X, Y> for DeconvolutedCentroidSeries<X, Y, T>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn to_svg(&self, xaxis: &XAxis<X>, yaxis: &YAxis<Y>) -> Group {
        self.to_svg(xaxis, yaxis)
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
        X: Float + Display,
        Y: Float + Display,
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

    pub fn to_svg(&self, xaxis: &XAxis<X>, yaxis: &YAxis<Y>) -> Group {
        let mut peaks_sorted: Vec<_> = self.peaks.iter().cloned().collect();
        peaks_sorted.sort_by(|a, b| a.mz().total_cmp(&b.mz()));
        let points = peaks_to_arrays(peaks_sorted.iter());
        let proxy = ContinuousSeries::new(points, self.description.clone());
        let group = proxy.to_svg(xaxis, yaxis);
        group
    }
}
