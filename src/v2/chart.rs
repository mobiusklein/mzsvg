use std::io::prelude::*;
use std::ops::Bound;
use std::path::Path;
use std::{fs, io, ops::RangeBounds};

use svg::node::element::Group;

use num_traits::Float;

use mzdata::{
    self,
    prelude::*,
    spectrum::{BinaryArrayMap, MultiLayerSpectrum, SignalContinuity},
};

use mzpeaks::{CentroidLike, DeconvolutedCentroidLike, MZLocated, MZPeakSetType, MassPeakSetType};

use super::chart_regions::{AxisOrientation, AxisProps, AxisTickLabelStyle, Canvas};
use super::series::{
    CentroidSeries, ColorCycle, ContinuousSeries, DeconvolutedCentroidSeries, PlotSeries,
    SeriesDescription,
};

use crate::{AnnotationSeries, CoordinateRange, LineSeries, TextProps};

#[derive(Debug, Clone)]
pub struct SpectrumSVG {
    pub canvas: Canvas<f64, f32>,
    pub colors: ColorCycle,
    pub xticks: AxisProps<f64>,
    pub yticks: AxisProps<f32>,
    pub x_range: Option<CoordinateRange<f64>>,
    pub y_range: Option<CoordinateRange<f32>>,
    pub finished: bool,
    groups: Vec<Group>,
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
            groups: Vec::new(),
        }
    }
}

impl SpectrumSVG {
    pub fn with_size(width: usize, height: usize) -> Self {
        Self::new(Canvas::new(width, height))
    }

    pub fn new(canvas: Canvas<f64, f32>) -> Self {
        let inst = Self {
            canvas,
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
            groups: Default::default(),
            finished: false,
        };
        inst
    }

    pub fn axes_from<
        C: CentroidLike + Default + Clone + 'static,
        D: DeconvolutedCentroidLike + Default + Clone + MZLocated + 'static,
    >(
        &mut self,
        spectrum: &MultiLayerSpectrum<C, D>,
    ) -> &mut Self {
        if self.y_range.is_none() {
            let tic = spectrum.peaks().base_peak().intensity;
            self.y_range = Some(CoordinateRange::new(tic, 0.0));
        }

        if self.x_range.is_none() {
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

            let xaxis = CoordinateRange::new(min_mz * 0.95, max_mz * 1.05);
            self.x_range = Some(xaxis);
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
            self.x_range.as_ref().unwrap().start,
            self.x_range.as_ref().unwrap().end,
        );

        let sgroup = series.to_svg(&self.canvas);
        self.canvas.groups.push(sgroup);
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
            self.x_range.as_ref().unwrap().start,
            self.x_range.as_ref().unwrap().end,
        );

        let sgroup = series.to_svg(&self.canvas);
        self.canvas.groups.push(sgroup);
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
            self.x_range.as_ref().unwrap().start,
            self.x_range.as_ref().unwrap().end,
        );

        let sgroup = series.to_svg(&self.canvas);
        self.canvas.groups.push(sgroup);
    }

    pub fn draw_series<S: PlotSeries<f64, f32>>(&mut self, mut series: S) {
        series.slice_x(
            self.x_range.as_ref().unwrap().start,
            self.x_range.as_ref().unwrap().end,
        );

        let sgroup = series.to_svg(&self.canvas);
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

        if let Some(precursor) = spectrum.precursor() {
            let x = precursor.ion().mz();
            let y = precursor
                .ion()
                .intensity
                .min(self.y_range.clone().unwrap().max())
                * 0.95;
            let z = precursor.ion().charge().unwrap_or(0);
            let s = format!("{x:0.2}, {z}");
            let pts = vec![(x, y, s)];
            let mut text_props = TextProps::default();
            text_props.text_size = 0.8;
            text_props.color = "skyblue".into();

            let annot = AnnotationSeries::new(pts, "Precursor".into(), text_props);
            let mut group = annot.to_svg(&self.canvas);
            group = group.set("stroke", "black").set("stroke-width", "0.1pt");
            self.canvas.groups.push(group);

            group = LineSeries::new(vec![(x, 0.0), (x, y)], "Precursor-Line".into()).to_svg(&self.canvas);
            group = group.set("stroke-dasharray", 4).set("stroke", "black").set("stroke-width", "0.5pt");
            self.canvas.groups.push(group)
        }
    }

    pub fn finish(&mut self) {
        if self.finished {
            return;
        };
        self.finished = true;
    }

    pub fn write<W: Write>(&self, stream: &mut W) -> io::Result<()> {
        svg::write(stream, &self.canvas.to_svg(&self.xticks, &self.yticks))?;
        Ok(())
    }

    pub fn save<P: AsRef<Path>>(&self, path: &P) -> io::Result<()> {
        svg::save(path, &self.canvas.to_svg(&self.xticks, &self.yticks))?;
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
