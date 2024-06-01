use std::ops::Bound;
use std::{io, ops::RangeBounds, fs};
use std::io::prelude::*;
use std::path::Path;
use std::mem;

use svg::node::element::Group;
use svg::{Document, Node};

use num_traits::Float;

use mzdata::{
    self,
    prelude::*,
    spectrum::{BinaryArrayMap, MultiLayerSpectrum, SignalContinuity},
};

use mzpeaks::{CentroidLike, DeconvolutedCentroidLike, MZLocated, MZPeakSetType, MassPeakSetType};

use crate::axes::{AxisLabelOptions, AxisTickLabelStyle, XAxis, YAxis};
use crate::series::{
    CentroidSeries, ColorCycle, ContinuousSeries, DeconvolutedCentroidSeries, SeriesDescription, PlotSeries
};

#[derive(Debug, Clone)]
pub struct SpectrumSVG {
    pub document: Document,
    pub intensity_scale: f64,
    pub mz_scale: f64,
    pub colors: ColorCycle,
    pub xticks: AxisLabelOptions,
    pub yticks: AxisLabelOptions,
    pub finished: bool,
    xaxis: Option<XAxis<f64>>,
    yaxis: Option<YAxis<f32>>,
    groups: Vec<Group>
}


const DEFAULT_X_SCALE: f64 = 8500.0;
const DEFAULT_Y_SCALE: f64 = 4000.0;
// const DEFAULT_ASPECT_RATIO: f64 = DEFAULT_X_SCALE / DEFAULT_Y_SCALE;


impl Default for SpectrumSVG {
    fn default() -> Self {
        Self {
            document: Document::new(),
            intensity_scale: DEFAULT_Y_SCALE,
            mz_scale: DEFAULT_X_SCALE,
            colors: Default::default(),
            xticks: AxisLabelOptions {
                tick_count: 7,
                tick_font_size: 80.0,
                label_font_size: 120.0,
                tick_style: AxisTickLabelStyle::Precision(2),
            },
            yticks: AxisLabelOptions {
                tick_count: 5,
                tick_font_size: 80.0,
                label_font_size: 120.0,
                tick_style: AxisTickLabelStyle::Percentile(1),
            },
            xaxis: None,
            yaxis: None,
            finished: false,
            groups: Vec::new()
        }
    }
}

impl SpectrumSVG {
    pub fn new(
        document: Document,
        aspect_ratio: f64,
        colors: ColorCycle,
        xticks: AxisLabelOptions,
        yticks: AxisLabelOptions,
        xaxis: Option<XAxis<f64>>,
        yaxis: Option<YAxis<f32>>,
        groups: Vec<Group>
    ) -> Self {
        let mut inst = Self {
            document,
            intensity_scale: DEFAULT_Y_SCALE,
            mz_scale: DEFAULT_X_SCALE,
            colors,
            xticks,
            yticks,
            xaxis,
            yaxis,
            groups,
            finished: false

        };
        inst.set_aspect_ratio(aspect_ratio);
        inst
    }

    pub fn aspect_ratio(&self) -> f64 {
        self.mz_scale / self.intensity_scale
    }

    pub fn set_aspect_ratio(&mut self, ratio: f64) {
        self.intensity_scale = self.mz_scale / ratio;
        if let Some(yaxis) = self.yaxis.as_mut() {
            yaxis.scale = self.intensity_scale;
        }
    }

    pub fn axes_from<
        C: CentroidLike + Default + Clone + 'static,
        D: DeconvolutedCentroidLike + Default + Clone + MZLocated + 'static,
    >(
        &mut self,
        spectrum: &MultiLayerSpectrum<C, D>,
    ) -> &mut Self {
        if self.yaxis.is_none() {
            let tic = spectrum.peaks().base_peak().intensity;
            let yaxis = YAxis::new(
                (tic..0.0).into(),
                "Rel. Intensity".to_string(),
                self.intensity_scale,
            );
            self.yaxis = Some(yaxis);
        }

        if self.xaxis.is_none() {
            let (min_mz, max_mz) = spectrum
                .acquisition()
                .first_scan()
                .map(|s| {
                    s.scan_windows.iter().fold(
                        (f64::infinity(), -f64::infinity()),
                        |(min, max), w| {
                            (
                                (w.lower_bound as f64).min(min),
                                (w.upper_bound as f64).max(max),
                            )
                        },
                    )
                })
                .unwrap_or_else(|| (50.0, 2000.0));

            let xaxis = XAxis::new(
                (min_mz * 0.95..max_mz * 1.05).into(),
                "m/z".to_string(),
                self.mz_scale,
            );
            self.xaxis = Some(xaxis);
        }

        self
    }

    pub fn xlim(&mut self, xlim: impl RangeBounds<f64>) -> &mut Self {
        let axis = self.xaxis.as_mut().unwrap();
        match xlim.start_bound() {
            Bound::Included(v) => axis.coordinates.start = *v,
            Bound::Excluded(v) => axis.coordinates.start = *v,
            Bound::Unbounded => {},
        }

        match xlim.end_bound() {
            Bound::Included(v) => axis.coordinates.end = *v,
            Bound::Excluded(v) => axis.coordinates.end = *v,
            Bound::Unbounded => {},
        }
        self
    }

    pub fn draw_profile(&mut self, arrays: &BinaryArrayMap) {
        let mzs = arrays.mzs().unwrap();
        let intensities = arrays.intensities().unwrap();

        let mut series = ContinuousSeries::from_iterators(
            mzs.iter().copied(),
            intensities.iter().copied(),
            SeriesDescription::from("Profile".to_string()).with_color(self.colors.next().unwrap()),
        );
        series.slice_x(
            self.xaxis.as_ref().unwrap().start(),
            self.xaxis.as_ref().unwrap().end(),
        );

        let xaxis = self.xaxis.as_ref().unwrap();
        let yaxis = self.yaxis.as_ref().unwrap();

        let sgroup = series.to_svg(&xaxis, &yaxis);
        self.groups.push(sgroup);
    }

    pub fn draw_centroids<C: CentroidLike + Default + Clone + 'static>(
        &mut self,
        peaks: &MZPeakSetType<C>,
    ) {
        let mut series = CentroidSeries::from_iterator(
            peaks.iter().cloned(),
            SeriesDescription::from("Centroid".to_string()).with_color(self.colors.next().unwrap()),
        );
        series.slice_x(
            self.xaxis.as_ref().unwrap().start(),
            self.xaxis.as_ref().unwrap().end(),
        );

        let xaxis = self.xaxis.as_ref().unwrap();
        let yaxis = self.yaxis.as_ref().unwrap();

        let sgroup = series.to_svg(&xaxis, &yaxis);
        self.groups.push(sgroup);
    }

    pub fn draw_deconvoluted_centroids<
        D: DeconvolutedCentroidLike + Default + Clone + MZLocated + 'static,
    >(
        &mut self,
        peaks: &MassPeakSetType<D>,
    ) {
        let mut series = DeconvolutedCentroidSeries::from_iterator(
            peaks.iter().cloned(),
            SeriesDescription::from("Deconvolved".to_string())
                .with_color(self.colors.next().unwrap()),
        );
        series.slice_x(
            self.xaxis.as_ref().unwrap().start(),
            self.xaxis.as_ref().unwrap().end(),
        );

        let xaxis = self.xaxis.as_ref().unwrap();
        let yaxis = self.yaxis.as_ref().unwrap();

        let sgroup = series.to_svg(&xaxis, &yaxis);
        self.groups.push(sgroup)
    }

    pub fn draw_series<S: PlotSeries<f64, f32>>(&mut self, mut series: S) {
        series.slice_x(
            self.xaxis.as_ref().unwrap().start(),
            self.xaxis.as_ref().unwrap().end(),
        );

        let xaxis = self.xaxis.as_ref().unwrap();
        let yaxis = self.yaxis.as_ref().unwrap();

        let sgroup = series.to_svg(&xaxis, &yaxis);
        self.groups.push(sgroup)
    }

    pub fn draw_spectrum<
        C: CentroidLike + Default + Clone + 'static,
        D: DeconvolutedCentroidLike + Default + Clone + MZLocated + 'static,
    >(
        &mut self,
        spectrum: &MultiLayerSpectrum<C, D>,
    ) {
        self.axes_from(&spectrum);

        if spectrum.signal_continuity() == SignalContinuity::Profile {
            let arrays = spectrum.raw_arrays().unwrap();

            self.draw_profile(&arrays);
        }

        if let Some(peaks) = spectrum.peaks.as_ref() {
            self.draw_centroids(peaks);
        }

        if let Some(peaks) = spectrum.deconvoluted_peaks.as_ref() {
            self.draw_deconvoluted_centroids(peaks);
        }
    }

    pub fn finish(&mut self) {
        if self.finished {
            return
        };
        self.finished = true;
        let mut container = Group::new().set(
            "transform",
            format!(
                "scale(0.9,0.9)translate({}, {})",
                self.mz_scale * 0.1,
                self.intensity_scale * 0.05
            ),
        );
        let groups = mem::take(&mut self.groups);
        let inner_container = Group::new().set("class", "canvas");
        let inner_container = groups
            .into_iter()
            .fold(inner_container, |container, group| container.add(group));

        let xaxis = self.xaxis.as_ref().unwrap();
        let yaxis = self.yaxis.as_ref().unwrap();

        let xgroup = xaxis.to_svg(&self.xticks, &yaxis);
        let ygroup = yaxis.to_svg(&self.yticks, &xaxis);
        container.append(inner_container);
        container.append(xgroup);
        container.append(ygroup);

        self.document = self
            .document
            .clone()
            .add(container)
            .set(
                "viewBox",
                (0.0, 0.0, self.mz_scale * 1.08, self.intensity_scale * 1.04),
            )
            .set("preserveAspectRatio", "xMidYMid meet");
    }

    pub fn write<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        svg::write(stream, &self.document)?;
        Ok(())
    }

    pub fn save<P: AsRef<Path>>(&self, path: &P) -> io::Result<()> {
        svg::save(path, &self.document)?;
        Ok(())
    }

    #[cfg(feature = "png")]
    pub fn write_png<W: Write>(&self, stream: &mut W) -> io::Result<()> {
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

        let size = tree.size().to_int_size();
        let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width() as u32, size.height() as u32).unwrap();
        pixmap.fill(resvg::tiny_skia::Color::WHITE);
        resvg::render(&tree, resvg::tiny_skia::Transform::identity(), &mut pixmap.as_mut());

        stream.write_all(&pixmap.encode_png().unwrap())?;
        Ok(())
    }

    #[cfg(feature = "png")]
    pub fn save_png<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut outfh = io::BufWriter::new(fs::File::create(path)?);
        self.write_png(&mut outfh)
    }

    #[cfg(feature = "pdf")]
    pub fn write_pdf<W: Write>(&self, stream: &mut W) -> io::Result<()> {
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
    pub fn save_pdf<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut outfh = io::BufWriter::new(fs::File::create(path)?);
        self.write_pdf(&mut outfh)
    }
}
