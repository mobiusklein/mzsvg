use std::collections::HashMap;
use std::io::prelude::*;
use std::ops::Bound;
use std::path::Path;
use std::{fs, io, ops::RangeBounds};

use num_traits::Float;

use mzdata::{
    self,
    prelude::*,
    spectrum::{BinaryArrayMap, MultiLayerSpectrum, SignalContinuity},
};

use mzpeaks::{
    feature::FeatureLike,
    {CentroidLike, DeconvolutedCentroidLike, MZLocated, MZPeakSetType, MassPeakSetType},
};
use svg::node::element::{Group, Style as CSSStyle};
use svg::{Document, Node};

use super::chart_regions::{AxisOrientation, AxisProps, AxisTickLabelStyle, Canvas};
use super::series::{
    CentroidSeries, ColorCycle, ContinuousSeries, DeconvolutedCentroidSeries, PlotSeries,
    SeriesDescription,
};

use crate::{AsSeries, CoordinateRange};

trait SVGCanvas {
    fn get_canvas(&self) -> &Canvas<f64, f32>;

    fn make_document(&self) -> Document;

    fn to_string(&self) -> String {
        self.make_document().to_string()
    }

    fn write<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        let doc = self.make_document();
        svg::write(stream, &doc)?;
        Ok(())
    }

    fn save<P: AsRef<Path>>(&self, path: &P) -> io::Result<()> {
        let mut fh = io::BufWriter::new(fs::File::create(path)?);
        self.write(&mut fh)?;
        Ok(())
    }

    #[cfg(feature = "png")]
    fn write_png<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        use std::sync::Arc;

        let mut buf = Vec::new();
        self.write(&mut buf)?;
        let mut fontdb = resvg::usvg::fontdb::Database::new();
        fontdb.load_system_fonts();

        fontdb.set_serif_family("Times New Roman");
        fontdb.set_sans_serif_family("Arial");
        fontdb.set_cursive_family("Comic Sans MS");
        fontdb.set_fantasy_family("Impact");
        fontdb.set_monospace_family("Courier New");

        let svg_opts = resvg::usvg::Options {
            fontdb: Arc::new(fontdb),
            ..Default::default()
        };

        let tree = resvg::usvg::Tree::from_data(&buf, &svg_opts).unwrap();

        let resolution_scale = 3.0;

        let size = tree
            .size()
            .to_int_size()
            .scale_by(resolution_scale)
            .unwrap();
        let mut pixmap =
            resvg::tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32).unwrap();
        pixmap.fill(resvg::tiny_skia::Color::WHITE);

        let ts = resvg::tiny_skia::Transform::from_scale(resolution_scale, resolution_scale);

        resvg::render(&tree, ts, &mut pixmap.as_mut());

        stream.write_all(&pixmap.encode_png().unwrap())?;
        Ok(())
    }

    #[cfg(feature = "png")]
    fn save_png<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut outfh = io::BufWriter::new(fs::File::create(path)?);
        self.write_png(&mut outfh)
    }

    #[cfg(feature = "pdf")]
    fn write_pdf<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        use std::sync::Arc;

        let mut buf = Vec::new();
        self.write(&mut buf)?;

        let conv_opts = svg2pdf::ConversionOptions::default();
        let mut page_opts = svg2pdf::PageOptions::default();
        page_opts.dpi = 180.0;

        let mut fontdb = fontdb::Database::new();
        fontdb.load_system_fonts();

        fontdb.set_serif_family("Times New Roman");
        fontdb.set_sans_serif_family("Arial");
        fontdb.set_cursive_family("Comic Sans MS");
        fontdb.set_fantasy_family("Impact");
        fontdb.set_monospace_family("Courier New");

        let svg_opts = svg2pdf::usvg::Options {
            fontdb: Arc::new(fontdb),
            ..Default::default()
        };

        let tree = svg2pdf::usvg::Tree::from_data(&buf, &svg_opts).unwrap();
        let pdf = svg2pdf::to_pdf(&tree, conv_opts, page_opts);
        stream.write_all(&pdf)?;
        Ok(())
    }

    #[cfg(feature = "pdf")]
    fn save_pdf<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut outfh = io::BufWriter::new(fs::File::create(path)?);
        self.write_pdf(&mut outfh)
    }
}

#[derive(Debug, Clone)]
pub struct SpectrumSVG {
    pub canvas: Canvas<f64, f32>,
    pub colors: ColorCycle,
    pub xticks: AxisProps<f64>,
    pub yticks: AxisProps<f32>,
    pub x_range: Option<CoordinateRange<f64>>,
    pub y_range: Option<CoordinateRange<f32>>,
    pub finished: bool,
    pub series: HashMap<String, Vec<SeriesDescription>>,
    pub custom_css: Option<String>,
}

impl Default for SpectrumSVG {
    fn default() -> Self {
        Self {
            canvas: Canvas::new(1400, 600),
            colors: Default::default(),
            xticks: AxisProps::new(AxisOrientation::Bottom)
                .label("m/z")
                .id("x-axis"),
            yticks: AxisProps::new(AxisOrientation::Left)
                .label("Intensity")
                .tick_format(AxisTickLabelStyle::Percentile(2))
                .id("y-axis"),
            x_range: Default::default(),
            y_range: Default::default(),
            finished: false,
            series: HashMap::new(),
            custom_css: None,
        }
    }
}

impl SVGCanvas for SpectrumSVG {
    fn get_canvas(&self) -> &Canvas<f64, f32> {
        &self.canvas
    }

    fn make_document(&self) -> Document {
        self.make_document()
    }
}

impl SpectrumSVG {
    pub fn with_size(width: usize, height: usize) -> Self {
        Self::new(Canvas::new(width, height))
    }

    pub fn new(canvas: Canvas<f64, f32>) -> Self {
        let inst = Self {
            canvas,
            ..Default::default()
        };
        inst
    }

    pub fn canvas_mut(&mut self) -> &mut Canvas<f64, f32> {
        &mut self.canvas
    }

    pub fn add_raw(&mut self, group: Group) {
        self.canvas.push_layer(group);
    }

    pub fn axes_from<
        C: CentroidLike + Default + Clone,
        D: DeconvolutedCentroidLike + Default + Clone + MZLocated,
    >(
        &mut self,
        spectrum: &MultiLayerSpectrum<C, D>,
    ) -> &mut Self {
        let max_int = spectrum.peaks().base_peak().intensity;
        if self.y_range.is_none() {
            self.y_range = Some(CoordinateRange::new(max_int, 0.0));
        } else {
            let y = self.y_range.as_mut().unwrap();
            y.start = y.start.max(max_int);
        }

        let (min_mz, max_mz) = spectrum
            .acquisition()
            .first_scan()
            .map(|s| {
                s.scan_windows
                    .iter()
                    .fold((f64::infinity(), -f64::infinity()), |(min, max), w| {
                        (
                            (w.lower_bound as f64).min(min),
                            (w.upper_bound as f64).max(max),
                        )
                    })
            })
            .unwrap_or_else(|| (50.0, 2000.0));
        if self.x_range.is_none() {
            let xaxis = CoordinateRange::new(min_mz * 0.95, max_mz * 1.05);
            self.x_range = Some(xaxis);
        } else {
            let x = self.x_range.as_mut().unwrap();
            x.start = x.start.min(min_mz);
            x.end = x.end.max(max_mz);
        }

        self.canvas
            .update_scales(self.x_range.clone().unwrap(), self.y_range.clone().unwrap());

        self
    }

    pub fn xlim(&mut self, xlim: impl RangeBounds<f64>) -> &mut Self {
        let axis = self.x_range.as_mut().unwrap();
        match xlim.start_bound() {
            Bound::Included(v) => axis.start = *v,
            Bound::Excluded(v) => axis.start = *v,
            Bound::Unbounded => {}
        }

        match xlim.end_bound() {
            Bound::Included(v) => axis.end = *v,
            Bound::Excluded(v) => axis.end = *v,
            Bound::Unbounded => {}
        }

        self.canvas
            .update_scales(self.x_range.clone().unwrap(), self.y_range.clone().unwrap());

        self
    }

    pub fn add_series(&mut self, mut series: impl PlotSeries<f64, f32>) {
        let descr = series.description();
        let tag = self.add_series_description(descr.clone());
        series.set_tag(tag);
        self.draw_series(series);
    }

    fn add_series_description(&mut self, descr: SeriesDescription) -> String {
        let tag = descr.series_type();
        let bucket = self.series.entry(tag).or_default();
        bucket.push(descr);
        bucket.len().to_string()
    }

    pub fn draw_profile(&mut self, arrays: &BinaryArrayMap) {
        let mzs = arrays.mzs().unwrap();
        let intensities = arrays.intensities().unwrap();

        let mut series = ContinuousSeries::from_iterators(
            mzs.iter().copied(),
            intensities.iter().copied(),
            SeriesDescription::from("profile".to_string()).with_color(self.colors.next().unwrap()),
        );
        series.slice_x(
            self.x_range.as_ref().unwrap().start,
            self.x_range.as_ref().unwrap().end,
        );

        let sgroup = series.to_svg(&self.canvas);
        self.canvas.push_layer(sgroup);
    }

    pub fn draw_centroids<C: CentroidLike + Default + Clone + 'static>(
        &mut self,
        peaks: &MZPeakSetType<C>,
    ) {
        let mut series = CentroidSeries::from_iterator(
            peaks.iter().cloned(),
            SeriesDescription::from("centroid".to_string()),
        );

        *series.color_mut() = self.colors.next().unwrap();

        self.add_series(series);
    }

    pub fn draw_deconvoluted_centroids<
        D: DeconvolutedCentroidLike + Default + Clone + MZLocated + 'static,
    >(
        &mut self,
        peaks: &MassPeakSetType<D>,
    ) {
        let mut series = DeconvolutedCentroidSeries::from_iterator(
            peaks.iter().cloned(),
            SeriesDescription::from("deconvoluted-centroid".to_string()),
        );
        *series.color_mut() = self.colors.next().unwrap();
        self.add_series(series);
    }

    pub fn add_as_series(&mut self, t: &impl AsSeries<f64, f32>) {
        let mut series = t.as_series();
        series.description_mut().color = self.colors.next().unwrap();
        self.add_series(series)
    }

    fn draw_series<S: PlotSeries<f64, f32>>(&mut self, mut series: S) {
        series.slice_x(
            self.x_range.as_ref().unwrap().start,
            self.x_range.as_ref().unwrap().end,
        );

        let sgroup = series.to_svg(&self.canvas);
        self.canvas.push_layer(sgroup)
    }

    pub fn draw_spectrum<
        C: CentroidLike + Default + Clone + 'static,
        D: DeconvolutedCentroidLike + Default + Clone + MZLocated + 'static,
    >(
        &mut self,
        spectrum: &MultiLayerSpectrum<C, D>,
    ) {
        if self.x_range.is_none() {
            self.axes_from(spectrum);
        }

        if spectrum.signal_continuity() == SignalContinuity::Profile {
            let arrays = spectrum.raw_arrays().unwrap();
            self.add_as_series(arrays);
        }

        if let Some(peaks) = spectrum.peaks.as_ref() {
            self.draw_centroids(peaks);
        }

        if let Some(peaks) = spectrum.deconvoluted_peaks.as_ref() {
            self.draw_deconvoluted_centroids(peaks);
        }

        if let Some(precursor) = spectrum.precursor() {
            if precursor.ion().intensity > 0.0 {
                self.add_as_series(precursor);
            }
        }
    }

    pub fn finish(&mut self) {
        if self.finished {
            return;
        };
        self.finished = true;
    }

    fn make_document(&self) -> Document {
        let mut document = Document::new();
        if let Some(css) = self.custom_css.as_ref() {
            let style = CSSStyle::new(css.to_string());
            document.append(style);
        }
        document.append(self.canvas.to_svg(&self.xticks, &self.yticks));
        document
    }

    pub fn to_string(&self) -> String {
        self.make_document().to_string()
    }

    pub fn write<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        SVGCanvas::write(self, stream)
    }

    pub fn save<P: AsRef<Path>>(&self, path: &P) -> io::Result<()> {
        SVGCanvas::save(self, path)
    }

    #[cfg(feature = "png")]
    pub fn write_png<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        SVGCanvas::write_png(self, stream)
    }

    #[cfg(feature = "png")]
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        SVGCanvas::save_png(self, path)
    }

    #[cfg(feature = "pdf")]
    pub fn write_pdf<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        SVGCanvas::write_pdf(self, stream)
    }

    #[cfg(feature = "pdf")]
    pub fn save_pdf<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        SVGCanvas::save_pdf(self, path)
    }
}

#[derive(Debug, Clone)]
pub struct FeatureSVG {
    pub canvas: Canvas<f64, f32>,
    pub colors: ColorCycle,
    pub xticks: AxisProps<f64>,
    pub yticks: AxisProps<f32>,
    pub x_range: Option<CoordinateRange<f64>>,
    pub y_range: Option<CoordinateRange<f32>>,
    pub finished: bool,
    pub series: HashMap<String, Vec<SeriesDescription>>,
    pub custom_css: Option<String>,
}

impl SVGCanvas for FeatureSVG {
    fn get_canvas(&self) -> &Canvas<f64, f32> {
        &self.canvas
    }

    fn make_document(&self) -> Document {
        self.make_document()
    }
}

impl FeatureSVG {
    pub fn with_size(width: usize, height: usize) -> Self {
        Self::new(Canvas::new(width, height))
    }

    pub fn new(canvas: Canvas<f64, f32>) -> Self {
        let inst = Self {
            canvas,
            ..Default::default()
        };
        inst
    }

    pub fn canvas_mut(&mut self) -> &mut Canvas<f64, f32> {
        &mut self.canvas
    }

    pub fn add_raw(&mut self, group: Group) {
        self.canvas.push_layer(group);
    }

    pub fn axes_from<X, Y, T: FeatureLike<X, Y>>(&mut self, feature: &T) -> &mut Self {
        let max_int = feature
            .iter()
            .map(|(_, _, z)| *z)
            .max_by(|a, b| a.total_cmp(b))
            .unwrap();

        if self.y_range.is_none() {
            self.y_range = Some(CoordinateRange::new(max_int, 0.0));
        } else {
            let y = self.y_range.as_mut().unwrap();
            y.start = y.start.max(max_int);
        }

        let start_time = feature.start_time().unwrap_or_default();
        let end_time = feature.end_time().unwrap_or_default();
        if self.x_range.is_none() {
            let xaxis = CoordinateRange::new(start_time * 0.95, end_time * 1.05);
            self.x_range = Some(xaxis);
        } else {
            let x = self.x_range.as_mut().unwrap();
            x.start = x.start.min(start_time);
            x.end = x.end.max(end_time);
        }

        self.canvas
            .update_scales(self.x_range.clone().unwrap(), self.y_range.clone().unwrap());

        self
    }

    pub fn xlim(&mut self, xlim: impl RangeBounds<f64>) -> &mut Self {
        let axis = self.x_range.as_mut().unwrap();
        match xlim.start_bound() {
            Bound::Included(v) => axis.start = *v,
            Bound::Excluded(v) => axis.start = *v,
            Bound::Unbounded => {}
        }

        match xlim.end_bound() {
            Bound::Included(v) => axis.end = *v,
            Bound::Excluded(v) => axis.end = *v,
            Bound::Unbounded => {}
        }

        self.canvas
            .update_scales(self.x_range.clone().unwrap(), self.y_range.clone().unwrap());

        self
    }

    pub fn finish(&mut self) {
        if self.finished {
            return;
        };
        self.finished = true;
    }

    fn make_document(&self) -> Document {
        let mut document = Document::new();
        if let Some(css) = self.custom_css.as_ref() {
            let style = CSSStyle::new(css.to_string());
            document.append(style);
        }
        document.append(self.canvas.to_svg(&self.xticks, &self.yticks));
        document
    }

    pub fn add_series(&mut self, mut series: impl PlotSeries<f64, f32>) {
        let descr = series.description();
        let tag = self.add_series_description(descr.clone());
        series.set_tag(tag);
        self.draw_series(series);
    }

    fn add_series_description(&mut self, descr: SeriesDescription) -> String {
        let tag = descr.series_type();
        let bucket = self.series.entry(tag).or_default();
        bucket.push(descr);
        bucket.len().to_string()
    }

    fn draw_series<S: PlotSeries<f64, f32>>(&mut self, mut series: S) {
        series.slice_x(
            self.x_range.as_ref().unwrap().start,
            self.x_range.as_ref().unwrap().end,
        );

        let sgroup = series.to_svg(&self.canvas);
        self.canvas.push_layer(sgroup)
    }

    pub fn add_as_series(&mut self, t: &impl AsSeries<f64, f32>) {
        let mut series = t.as_series();
        series.description_mut().color = self.colors.next().unwrap();
        self.add_series(series)
    }

    pub fn to_string(&self) -> String {
        self.make_document().to_string()
    }

    pub fn write<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        SVGCanvas::write(self, stream)
    }

    pub fn save<P: AsRef<Path>>(&self, path: &P) -> io::Result<()> {
        SVGCanvas::save(self, path)
    }

    #[cfg(feature = "png")]
    pub fn write_png<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        SVGCanvas::write_png(self, stream)
    }

    #[cfg(feature = "png")]
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        SVGCanvas::save_png(self, path)
    }

    #[cfg(feature = "pdf")]
    pub fn write_pdf<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        SVGCanvas::write_pdf(self, stream)
    }

    #[cfg(feature = "pdf")]
    pub fn save_pdf<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        SVGCanvas::save_pdf(self, path)
    }
}

impl Default for FeatureSVG {
    fn default() -> Self {
        Self {
            canvas: Canvas::new(1400, 600),
            colors: Default::default(),
            xticks: AxisProps::new(AxisOrientation::Bottom)
                .label("Time")
                .id("x-axis"),
            yticks: AxisProps::new(AxisOrientation::Left)
                .label("Intensity")
                .tick_format(AxisTickLabelStyle::SciNot(2))
                .id("y-axis"),
            x_range: Default::default(),
            y_range: Default::default(),
            finished: false,
            series: HashMap::new(),
            custom_css: None,
        }
    }
}
