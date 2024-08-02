use std::marker::PhantomData;

use mzdata::spectrum::{Precursor, PrecursorSelection};
use mzpeaks::{
    feature::{ChargedFeature, Feature, FeatureLike, SimpleFeature},
    CentroidLike, CentroidPeak, DeconvolutedCentroidLike, DeconvolutedPeak, DeconvolutedPeakSet,
    IntensityMeasurement, MZLocated, MZPeakSetType, MassPeakSetType, PeakSet,
};
use num_traits::Float;

use svg::node::element::{path::Data as PathData, Circle, Group, Path, Polyline};

use super::chart_regions::{Canvas, RenderCoordinate, TextProps};

pub const DEFAULT_COLOR_CYCLE: &'static [&'static str] = &[
    "black",
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

pub trait PlotSeries<X: RenderCoordinate, Y: RenderCoordinate> {
    fn description(&self) -> &SeriesDescription;

    fn description_mut(&mut self) -> &mut SeriesDescription;

    fn series_type(&self) -> String {
        self.description().series_type()
    }

    fn series_id(&self) -> String {
        self.description().id()
    }

    fn set_tag(&mut self, tag: String) {
        self.description_mut().tag = tag
    }

    fn color(&self) -> String {
        self.description().color.clone()
    }

    fn color_mut(&mut self) -> &mut String {
        &mut self.description_mut().color
    }

    fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group;

    fn slice_x(&mut self, start: X, end: X);
    fn slice_y(&mut self, start: Y, end: Y);
}

pub trait AsSeries<X: RenderCoordinate, Y: RenderCoordinate> {
    type Series: PlotSeries<X, Y>;

    fn as_series(&self) -> Self::Series;
}

impl<X: RenderCoordinate, Y: RenderCoordinate, T: AsSeries<X, Y>> AsSeries<X, Y> for &T {
    type Series = <T as AsSeries<X, Y>>::Series;

    fn as_series(&self) -> Self::Series {
        (*self).as_series()
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SeriesDescription {
    pub label: String,
    pub color: String,
    pub tag: String,
}

impl SeriesDescription {
    pub fn new(label: String, color: String) -> Self {
        Self {
            label,
            color,
            tag: String::new(),
        }
    }

    pub fn with_color(mut self, color: String) -> Self {
        self.color = color;
        self
    }

    pub fn series_type(&self) -> String {
        self.label.to_string()
    }

    pub fn id(&self) -> String {
        format!("{}-{}", self.label, self.tag)
    }
}

impl From<String> for SeriesDescription {
    fn from(value: String) -> Self {
        SeriesDescription::new(value, "black".to_string())
    }
}

impl From<&str> for SeriesDescription {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LineSeries<X: RenderCoordinate, Y: RenderCoordinate> {
    pub points: Vec<(X, Y)>,
    pub description: SeriesDescription,
}

impl<X: RenderCoordinate, Y: RenderCoordinate> LineSeries<X, Y> {
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
        let path_data: Vec<_> = self
            .points
            .iter()
            .enumerate()
            .map(|(_, (mz, inten))| {
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
        group
            .add(path)
            .set("class", self.description.label.clone())
            .set("id", self.description.id())
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate> PlotSeries<X, Y> for LineSeries<X, Y> {
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn description_mut(&mut self) -> &mut SeriesDescription {
        &mut self.description
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
pub struct ContinuousSeries<X: RenderCoordinate, Y: RenderCoordinate> {
    pub points: Vec<(X, Y)>,
    pub description: SeriesDescription,
}

impl<X: RenderCoordinate, Y: RenderCoordinate> ContinuousSeries<X, Y> {
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
            .copied()
            .unwrap_or((X::zero(), Y::zero()))
            .0;
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
                state.line_to(canvas.transform(*mz, *inten))
            })
            .close();
        let path = Path::new().set("fill", "none").set("d", path_data);
        let group = Group::new();
        group
            .add(path)
            .set("stroke", self.description.color.clone())
            .set("stroke-width", 1)
            .set("class", self.series_type())
            .set("id", self.series_id())
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate> PlotSeries<X, Y> for ContinuousSeries<X, Y> {
    fn description(&self) -> &SeriesDescription {
        &self.description
    }
    fn description_mut(&mut self) -> &mut SeriesDescription {
        &mut self.description
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
pub struct AnnotationSeries<X: RenderCoordinate, Y: RenderCoordinate> {
    pub points: Vec<(X, Y, String)>,
    pub description: SeriesDescription,
    pub text_props: TextProps,
}

impl<X: RenderCoordinate, Y: RenderCoordinate> PlotSeries<X, Y> for AnnotationSeries<X, Y> {
    fn description(&self) -> &SeriesDescription {
        &self.description
    }
    fn description_mut(&mut self) -> &mut SeriesDescription {
        &mut self.description
    }
    fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        self.to_svg(canvas)
    }

    fn slice_x(&mut self, start: X, end: X) {
        let points = self
            .points
            .iter()
            .filter(|(x, _, _)| (x >= &start) && (x <= &end))
            .cloned()
            .collect();
        self.points = points;
    }

    fn slice_y(&mut self, start: Y, end: Y) {
        let points = self
            .points
            .iter()
            .filter(|(_, y, _)| (y >= &start) && (y <= &end))
            .cloned()
            .collect();
        self.points = points;
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate> AnnotationSeries<X, Y> {
    pub fn new(
        points: Vec<(X, Y, String)>,
        description: SeriesDescription,
        text_props: TextProps,
    ) -> Self {
        Self {
            points,
            description,
            text_props,
        }
    }

    pub fn from_iterators(
        xiter: impl Iterator<Item = X>,
        yiter: impl Iterator<Item = Y>,
        text_iter: impl Iterator<Item = String>,
        description: SeriesDescription,
    ) -> Self {
        Self {
            points: xiter
                .zip(yiter)
                .zip(text_iter)
                .map(|((x, y), text)| (x, y, text))
                .collect(),
            description,
            text_props: TextProps::default(),
        }
    }

    pub fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        let mut group = Group::new();
        for (x, y, text) in self.points.iter() {
            let (x2, y2) = canvas.transform(*x, *y);
            group = group.add(
                Group::new()
                    .set("transform", format!("translate({}, {})", x2, y2))
                    .add(self.text_props.text(text.clone())),
            )
        }
        group
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
                "profile".into(),
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
    X: RenderCoordinate,
    Y: RenderCoordinate,
    T: CentroidLike + Clone + 'static,
> {
    pub peaks: MZPeakSetType<T>,
    pub description: SeriesDescription,
    _x: PhantomData<X>,
    _y: PhantomData<Y>,
}

impl<X: RenderCoordinate, Y: RenderCoordinate, T: CentroidLike + Clone + 'static> PlotSeries<X, Y>
    for CentroidSeries<X, Y, T>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }
    fn description_mut(&mut self) -> &mut SeriesDescription {
        &mut self.description
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

impl<X: RenderCoordinate, Y: RenderCoordinate, T: CentroidLike + Clone + 'static>
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

    pub fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        let points = peaks_to_arrays(self.peaks.iter());
        let proxy = ContinuousSeries::new(points, self.description.clone());
        let group = proxy.to_svg(canvas);
        group
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate> AsSeries<X, Y> for PeakSet {
    type Series = CentroidSeries<X, Y, CentroidPeak>;

    fn as_series(&self) -> Self::Series {
        CentroidSeries::from_iterator(self.iter().cloned(), "centroid".into())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DeconvolutedCentroidSeries<
    X: RenderCoordinate,
    Y: RenderCoordinate,
    T: DeconvolutedCentroidLike + Clone + MZLocated + 'static,
> {
    pub peaks: MassPeakSetType<T>,
    pub description: SeriesDescription,
    _x: PhantomData<X>,
    _y: PhantomData<Y>,
}

impl<
        X: RenderCoordinate,
        Y: RenderCoordinate,
        T: DeconvolutedCentroidLike + Clone + MZLocated + 'static,
    > PlotSeries<X, Y> for DeconvolutedCentroidSeries<X, Y, T>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }
    fn description_mut(&mut self) -> &mut SeriesDescription {
        &mut self.description
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
        X: RenderCoordinate,
        Y: RenderCoordinate,
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

impl<X: RenderCoordinate, Y: RenderCoordinate> AsSeries<X, Y> for DeconvolutedPeakSet {
    type Series = DeconvolutedCentroidSeries<X, Y, DeconvolutedPeak>;

    fn as_series(&self) -> Self::Series {
        Self::Series::from_iterator(self.iter().cloned(), "deconvoluted-centroid".into())
    }
}

#[derive(Debug, Clone)]
pub struct PrecursorSeries<X: RenderCoordinate, Y: RenderCoordinate> {
    mz: X,
    intensity: Y,
    charge: Option<i32>,
    in_frame: bool,
    description: SeriesDescription,
}

impl<X: RenderCoordinate, Y: RenderCoordinate> PlotSeries<X, Y> for PrecursorSeries<X, Y> {
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn description_mut(&mut self) -> &mut SeriesDescription {
        &mut self.description
    }

    fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        let root = Group::new();
        if !self.in_frame {
            return root;
        }
        let x = self.mz;
        let y = self.intensity.min(canvas.y_axis.domain().max()) * Y::from(0.95).unwrap();
        let z = self.charge.unwrap_or(0);
        let s = format!("{x:0.2}, {z}");
        let pts = vec![(x, y, s)];

        let mut text_props = TextProps::default();
        text_props.text_size = 0.8;
        text_props.color = "skyblue".into();

        let annot = AnnotationSeries::new(pts, "precursor-label".into(), text_props);
        let annot_group = annot
            .to_svg(&canvas)
            .set("stroke", "black")
            .set("stroke-width", "0.1pt");

        let line_group = LineSeries::new(vec![(x, Y::zero()), (x, y)], "precursor-line".into())
            .to_svg(&canvas)
            .set("stroke-dasharray", 4)
            .set("stroke", self.description.color.clone())
            .set("stroke-width", "0.5pt");

        root.add(annot_group)
            .add(line_group)
            .set("class", "precursor")
            .set("id", self.description.id())
    }

    fn slice_x(&mut self, start: X, end: X) {
        self.in_frame = start <= self.mz && self.mz <= end;
    }

    fn slice_y(&mut self, start: Y, end: Y) {
        self.intensity = start.max(end);
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate> PrecursorSeries<X, Y> {
    pub fn new(mz: X, intensity: Y, charge: Option<i32>, description: SeriesDescription) -> Self {
        Self {
            mz,
            intensity,
            charge,
            description,
            in_frame: true,
        }
    }

    pub fn from_precursor(precursor: &impl PrecursorSelection) -> Self {
        let ion = precursor.ion();
        Self::new(
            X::from(ion.mz).unwrap(),
            Y::from(ion.intensity).unwrap(),
            ion.charge,
            "precursor".into(),
        )
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate> AsSeries<X, Y> for Precursor {
    type Series = PrecursorSeries<X, Y>;

    fn as_series(&self) -> Self::Series {
        Self::Series::from_precursor(self)
    }
}

pub struct TraceSeries<X: RenderCoordinate, Y: RenderCoordinate, C1, C2, F: FeatureLike<C1, C2>> {
    pub feature: F,
    points: Vec<(X, Y)>,
    pub description: SeriesDescription,
    _c1: PhantomData<C1>,
    _c2: PhantomData<C2>,
    _x: PhantomData<X>,
    _y: PhantomData<Y>,
}

impl<X: RenderCoordinate, Y: RenderCoordinate, C1, C2, F: FeatureLike<C1, C2>>
    TraceSeries<X, Y, C1, C2, F>
{
    pub fn new(feature: F, description: SeriesDescription) -> Self {
        let points: Vec<(X, Y)> = feature
            .iter()
            .map(|(_, time, inten)| (X::from(*time).unwrap(), Y::from(*inten).unwrap()))
            .collect();

        Self {
            feature,
            description,
            points,
            _c1: PhantomData,
            _c2: PhantomData,
            _x: PhantomData,
            _y: PhantomData,
        }
    }

    pub fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        let start_time = self
            .points
            .iter()
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .copied()
            .unwrap_or((X::zero(), Y::zero()))
            .0;
        let end_time = self
            .points
            .iter()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .copied()
            .unwrap_or((X::zero(), Y::zero()))
            .0;
        let path_data = self
            .points
            .iter()
            .enumerate()
            .fold(PathData::new(), |mut state, (i, (time, inten))| {
                if i == 0 {
                    state = state.move_to((
                        canvas.x_axis.scale.transform(start_time).to_f64().unwrap(),
                        canvas.y_axis.scale.transform(Y::zero()).to_f64().unwrap(),
                    ));
                }
                state.line_to(canvas.transform(X::from(*time).unwrap(), Y::from(*inten).unwrap()))
            })
            .line_to((
                canvas.x_axis.scale.transform(end_time).to_f64().unwrap(),
                canvas.y_axis.scale.transform(Y::zero()).to_f64().unwrap(),
            ))
            .line_to((
                canvas.x_axis.scale.transform(start_time).to_f64().unwrap(),
                canvas.y_axis.scale.transform(Y::zero()).to_f64().unwrap(),
            ))
            .close();
        let path = Path::new()
            .set("fill", self.color())
            .set("d", path_data.clone())
            .set("fill-opacity", "75%");
        // let path2 = Path::new().set("fill", "none").set("d", path_data.clone());
        let group = Group::new();
        group
            .add(path)
            // .add(path2)
            .set("stroke", "black")
            .set("stroke-width", 1)
            .set("class", self.series_type())
            .set("id", self.series_id())
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate, C1, C2, F: FeatureLike<C1, C2>> PlotSeries<X, Y>
    for TraceSeries<X, Y, C1, C2, F>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn description_mut(&mut self) -> &mut SeriesDescription {
        &mut self.description
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

impl<X: RenderCoordinate, Y: RenderCoordinate, C1: Clone, C2: Clone> AsSeries<X, Y>
    for Feature<C1, C2>
where
    Feature<C1, C2>: FeatureLike<C1, C2>,
{
    type Series = TraceSeries<X, Y, C1, C2, Feature<C1, C2>>;

    fn as_series(&self) -> Self::Series {
        Self::Series::new((*self).clone(), SeriesDescription::from("feature"))
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate, C1: Clone, C2: Clone> AsSeries<X, Y>
    for ChargedFeature<C1, C2>
where
    ChargedFeature<C1, C2>: FeatureLike<C1, C2>,
{
    type Series = TraceSeries<X, Y, C1, C2, ChargedFeature<C1, C2>>;

    fn as_series(&self) -> Self::Series {
        Self::Series::new((*self).clone(), SeriesDescription::from("charged-feature"))
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate, C1: Clone, C2: Clone> AsSeries<X, Y>
    for SimpleFeature<C1, C2>
where
    SimpleFeature<C1, C2>: FeatureLike<C1, C2>,
{
    type Series = TraceSeries<X, Y, C1, C2, SimpleFeature<C1, C2>>;

    fn as_series(&self) -> Self::Series {
        Self::Series::new((*self).clone(), SeriesDescription::from("simple-feature"))
    }
}

pub struct ScatterSeries<X: RenderCoordinate, Y: RenderCoordinate, R: Into<svg::node::Value> + Clone> {
    pub points: Vec<(X, Y, R)>,
    pub description: SeriesDescription,
}

impl<X: RenderCoordinate, Y: RenderCoordinate, R: Into<svg::node::Value> + Clone> PlotSeries<X, Y>
    for ScatterSeries<X, Y, R>
{
    fn description(&self) -> &SeriesDescription {
        &self.description
    }

    fn description_mut(&mut self) -> &mut SeriesDescription {
        &mut self.description
    }

    fn to_svg(&self, canvas: &Canvas<X, Y>) -> Group {
        self.points.iter().fold(Group::new(), |group, (x, y, r)| {
            group.add(
                Circle::new()
                    .set("cx", canvas.x_axis.scale.transform(*x).to_f64().unwrap())
                    .set("cy", canvas.y_axis.scale.transform(*y).to_f64().unwrap())
                    .set("r", r.clone())
            )
        })
        .set("class", self.series_type())
        .set("id", self.series_id())
        .set("fill", self.color())
        .set("stroke", "black")
    }

    fn slice_x(&mut self, start: X, end: X) {
        self.points = std::mem::take(&mut self.points)
            .into_iter()
            .filter(|(x, ..)| *x >= start && *x <= end)
            .collect();
    }

    fn slice_y(&mut self, start: Y, end: Y) {
        self.points = std::mem::take(&mut self.points)
            .into_iter()
            .filter(|(_, y, ..)| *y >= start && *y <= end)
            .collect();
    }
}

impl<X: RenderCoordinate, Y: RenderCoordinate, R: Into<svg::node::Value> + Clone> ScatterSeries<X, Y, R> {
    pub fn new(points: Vec<(X, Y, R)>, description: SeriesDescription) -> Self {
        Self {
            points,
            description,
        }
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

        let _ = canvas.to_svg(&props, &props2);
    }

    #[test]
    fn test_scatter() {
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

        let series = ScatterSeries {
            points: vec![(250.0, 7000.5, 25), (350.0, 150.0, 50), (571.0, 4000.0, 50)],
            description: "test".into(),
        };

        canvas.groups.push(series.to_svg(&canvas));

        let doc = canvas.to_svg(&props, &props2);
        eprintln!("{}", doc.to_string())
    }
}
