use std::fmt::{Display, LowerExp};

use num_traits::Float;

use svg::node::element::{path::Data as PathData, Group, Line, Path, Text};

use crate::linear::{CoordinateRange, Scale};

pub trait RenderCoordinate: Float + Display + LowerExp {}

impl<T: Float + Display + LowerExp> RenderCoordinate for T {}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisOrientation {
    Top,
    Right,
    Bottom,
    Left,
}

impl AxisOrientation {
    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }

    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Sides {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}


#[derive(Debug, Clone)]
pub struct DrawBox<X: RenderCoordinate, Y: RenderCoordinate> {
    pub width: usize,
    pub height: usize,
    pub x_axis: XAxis<X>,
    pub y_axis: YAxis<Y>,
    pub groups: Vec<Group>,
    pub origin: (X, Y),
}

#[allow(unused)]
impl<X: RenderCoordinate, Y: RenderCoordinate> DrawBox<X, Y> {
    pub fn new(width: usize, height: usize, groups: Vec<Group>, origin: (X, Y)) -> Self {
        let domain = CoordinateRange::new(X::zero(), X::from(width).unwrap());
        let range = domain.clone();
        let x_axis = XAxis::new(Scale::new(domain, range));

        let domain = CoordinateRange::new(Y::zero(), Y::from(height).unwrap());
        let range = domain.clone();
        let y_axis = YAxis::new(Scale::new(domain, range));

        Self { width, height, x_axis, y_axis, groups, origin }
    }

    pub fn update_scales(&mut self, x_range: CoordinateRange<X>, y_range: CoordinateRange<Y>) {
        self.x_axis.scale.domain = x_range;
        self.y_axis.scale.domain = y_range;
    }

    pub fn transform(&self, x: X, y: Y) -> (f64, f64) {
        (
            self.x_axis.scale.transform(x).to_f64().unwrap(),
            self.y_axis.scale.transform(y).to_f64().unwrap(),
        )
    }

    pub fn push_layer(&mut self, group: Group) {
        self.groups.push(group)
    }

    pub fn to_svg(&self) -> Group {
        let group = self.groups.iter().fold(
            Group::new().set("class", "draw-box"),
            |holder, series| holder.add(series.clone()),
        );
        group
    }

}


#[derive(Debug, Clone)]
pub struct Canvas<X: RenderCoordinate, Y: RenderCoordinate> {
    pub width: usize,
    pub height: usize,
    pub x_axis: XAxis<X>,
    pub y_axis: YAxis<Y>,
    pub groups: Vec<Group>,
    pub subplot_offset: Option<(X, Y)>
}

impl<X: RenderCoordinate, Y: RenderCoordinate> Canvas<X, Y> {
    pub fn new(width: usize, height: usize) -> Self {
        let domain = CoordinateRange::new(X::zero(), X::from(width).unwrap());
        let range = domain.clone();
        let x_axis = XAxis::new(Scale::new(domain, range));

        let domain = CoordinateRange::new(Y::zero(), Y::from(height).unwrap());
        let range = domain.clone();
        let y_axis = YAxis::new(Scale::new(domain, range));

        Self {
            width,
            height,
            x_axis,
            y_axis,
            groups: Vec::new(),
            subplot_offset: None
        }
    }

    pub fn update_scales(&mut self, x_range: CoordinateRange<X>, y_range: CoordinateRange<Y>) {
        self.x_axis.scale.domain = x_range;
        self.y_axis.scale.domain = y_range;
    }

    pub fn transform(&self, x: X, y: Y) -> (f64, f64) {
        (
            self.x_axis.scale.transform(x).to_f64().unwrap(),
            self.y_axis.scale.transform(y).to_f64().unwrap(),
        )
    }

    pub fn push_layer(&mut self, group: Group) {
        self.groups.push(group)
    }

    pub fn to_svg(&self, x_axis_props: &AxisProps<X>, y_axis_props: &AxisProps<Y>) -> Group {
        let data = self.groups.iter().fold(
            Group::new().set("class", "data-canvas"),
            |holder, series| holder.add(series.clone()),
        );

        let group = Group::new()
            .set(
                "transform",
                format!(
                    "translate({}, {})",
                    y_axis_props.tick_spacing() * 6.0,
                    x_axis_props.tick_spacing() * 4.0
                ),
            )
            .set("class", "canvas")
            .add(data)
            .add(x_axis_props.to_svg(&self.x_axis.scale, &self))
            .add(y_axis_props.to_svg(&self.y_axis.scale, &self));

        group
    }
}

#[derive(Debug, Clone, Copy)]
pub struct XAxis<T: Float> {
    pub scale: Scale<T>,
}

impl<T: Float> XAxis<T> {
    pub fn new(scale: Scale<T>) -> Self {
        Self { scale }
    }

    pub fn domain(&self) -> &CoordinateRange<T> {
        &self.scale.domain
    }

    pub fn range(&self) -> &CoordinateRange<T> {
        &self.scale.range
    }
}

#[derive(Debug, Clone, Copy)]
pub struct YAxis<T: Float> {
    pub scale: Scale<T>,
}

impl<T: Float> YAxis<T> {
    pub fn new(scale: Scale<T>) -> Self {
        Self { scale }
    }

    pub fn domain(&self) -> &CoordinateRange<T> {
        &self.scale.domain
    }

    pub fn range(&self) -> &CoordinateRange<T> {
        &self.scale.range
    }
}

fn translate_x<T: Float>(x: T) -> String {
    let x = x.to_f64().unwrap();
    format!("translate({x},0)")
}

fn translate_y<T: Float>(y: T) -> String {
    let y = y.to_f64().unwrap();
    format!("translate(0, {y})")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisTickLabelStyle {
    Precision(usize),
    #[allow(unused)]
    SciNot(usize),
    Percentile(usize),
}

impl Default for AxisTickLabelStyle {
    fn default() -> Self {
        Self::Precision(2)
    }
}

impl AxisTickLabelStyle {
    pub fn format<F: RenderCoordinate>(
        &self,
        value: &F,
        scale: &CoordinateRange<F>,
    ) -> String {
        match self {
            AxisTickLabelStyle::Precision(p) => format!("{1:.*}", p, value),
            AxisTickLabelStyle::SciNot(p) => format!("{1:.*e}", p, value),
            AxisTickLabelStyle::Percentile(p) => {
                let percent = (*value / scale.max()).to_f64().unwrap() * 100.0;
                format!("{1:.*}%", p, percent)
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AxisLabelOptions {
    pub tick_count: usize,
    pub tick_font_size: f64,
    pub label_font_size: f64,
    pub tick_style: AxisTickLabelStyle,
}

#[derive(Debug, Clone)]
pub struct AxisProps<T: RenderCoordinate> {
    pub tick_padding: f64,
    pub tick_size_outer: f64,
    pub tick_size_inner: f64,
    pub tick_format: AxisTickLabelStyle,
    pub axis_orientation: AxisOrientation,
    pub tick_values: Option<Vec<T>>,
    pub label: Option<String>,
    pub id: Option<String>,
    pub tick_label_size: Option<f64>,
    pub axis_label_size: Option<f64>,
}

pub const DEFAULT_TICK_LABEL_SIZE: f64 = 10.0;
pub const DEFAULT_AXIS_LABEL_SIZE: f64 = 14.0;

impl<T: RenderCoordinate> AxisProps<T> {
    pub fn new(axis_orientation: AxisOrientation) -> Self {
        Self {
            tick_format: AxisTickLabelStyle::Precision(2),
            tick_padding: 3.0,
            tick_size_outer: 6.0,
            tick_size_inner: 6.0,
            axis_orientation,
            tick_values: None,
            label: None,
            id: None,
            tick_label_size: None,
            axis_label_size: None,
        }
    }

    pub fn label<S: ToString>(mut self, label: S) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn set_label<S: ToString>(&mut self, label: S) {
        self.label = Some(label.to_string());
    }

    pub fn tick_format(mut self, style: AxisTickLabelStyle) -> Self {
        self.tick_format = style;
        self
    }

    pub fn id<S: ToString>(mut self, id: S) -> Self {
        self.id = Some(id.to_string());
        self
    }

    pub fn transform(&self, x: T) -> String {
        match self.axis_orientation {
            AxisOrientation::Left | AxisOrientation::Right => translate_y(x),
            _ => translate_x(x),
        }
    }

    pub fn tick_spacing(&self) -> f64 {
        self.tick_size_outer + self.tick_size_inner.max(0.0) + self.tick_padding
    }

    pub fn to_svg<X: RenderCoordinate, Y: RenderCoordinate>(
        &self,
        scale: &Scale<T>,
        canvas: &Canvas<X, Y>,
    ) -> Group {
        let values = self.tick_values.clone().unwrap_or_else(|| {
            if scale.domain.start <= scale.domain.end {
                let span = scale.domain.end - scale.domain.start;
                let step = span.to_f64().unwrap() / 5.0;
                (0..6)
                    .into_iter()
                    .map(|i| scale.domain.start + T::from(step * i as f64).unwrap())
                    .collect()
            } else {
                let span = scale.domain.start - scale.domain.end;
                let step = span.to_f64().unwrap() / 5.0;
                (0..6)
                    .into_iter()
                    .map(|i| scale.domain.end + T::from(step * i as f64).unwrap())
                    .collect()
            }
        });

        let spacing = self.tick_spacing();
        let range0 = scale.range.min().to_f64().unwrap() - 1.0;
        let range1 = scale.range.max().to_f64().unwrap() + 1.0;

        let mut container = Group::new().set("fill", "none").set(
            "font-size",
            self.tick_label_size.unwrap_or(DEFAULT_TICK_LABEL_SIZE),
        );

        if let Some(id) = self.id.as_ref() {
            container = container.set("class", id.to_string());
        }

        container = match self.axis_orientation {
            AxisOrientation::Right => container.set("text-anchor", "start"),
            AxisOrientation::Top | AxisOrientation::Bottom => {
                container.set("text-anchor", "middle")
            }
            AxisOrientation::Left => container.set("text-anchor", "end"),
        };

        let path = if self.axis_orientation.is_horizontal() {
            PathData::default()
                .move_to((range0, 0))
                .line_by((range1, 0))
        } else {
            PathData::default()
                .move_to((0, range0))
                .line_by((0, range1))
        };
        let path = Path::new()
            .set("fill", "none")
            .set("stroke", "black")
            .set("stroke-width", 0.75)
            .set("class", "domain")
            .set("d", path);
        container = container.add(path);

        container = values
            .iter()
            .enumerate()
            .map(|(_, v)| {
                let range_v = scale.transform(*v).to_f64().unwrap();
                // eprintln!("Tick {i} {0} -> {range_v}", v.to_f64().unwrap());
                let tick_container = Group::new().set("class", "tick").set(
                    "transform",
                    if self.axis_orientation.is_horizontal() {
                        translate_x(range_v)
                    } else {
                        translate_y(range_v)
                    },
                );
                let line = Line::new().set("stroke", "black").set("stroke-width", 0.75);
                let line = match self.axis_orientation {
                    AxisOrientation::Top => line.set("y2", -self.tick_size_inner),
                    AxisOrientation::Right => line.set("x2", self.tick_size_inner),
                    AxisOrientation::Bottom => line.set("y2", self.tick_size_inner),
                    AxisOrientation::Left => line.set("x2", -self.tick_size_inner),
                };
                let label =
                    Text::new(self.tick_format.format(v, &scale.domain)).set("fill", "black");
                let label = match self.axis_orientation {
                    AxisOrientation::Top => label.set("y", -spacing).set("dy", "-0.32em"),
                    AxisOrientation::Right => label.set("x", spacing).set("dy", "0.32em"),
                    AxisOrientation::Bottom => label.set("y", spacing).set("dy", "0.32em"),
                    AxisOrientation::Left => label.set("x", -spacing).set("dy", "0.32em"),
                };
                tick_container.add(label).add(line)
            })
            .fold(container, |container, tick| container.add(tick));

        match self.axis_orientation {
            AxisOrientation::Top => {}
            AxisOrientation::Bottom => {
                container = container.set("transform", translate_y(canvas.height as f64))
            }
            AxisOrientation::Right => todo!(),
            AxisOrientation::Left => {}
        }

        if let Some(label) = self.label.as_ref() {
            let midpoint = (scale.range.max() - scale.range.min()) / T::from(2.0).unwrap();
            let group = Group::new().set(
                "transform",
                if self.axis_orientation.is_horizontal() {
                    translate_x(midpoint)
                } else {
                    translate_y(midpoint) + "rotate(-90)"
                },
            );
            container = container.add(
                group.add(match self.axis_orientation {
                    AxisOrientation::Top => todo!(),
                    AxisOrientation::Right => todo!(),
                    AxisOrientation::Bottom => Text::new(label)
                        .set("y", spacing * 2.5)
                        .set("fill", "black")
                        .set(
                            "font-size",
                            self.axis_label_size.unwrap_or(DEFAULT_AXIS_LABEL_SIZE),
                        )
                        .set("text-anchor", "middle"),
                    AxisOrientation::Left => Text::new(label)
                        .set("y", spacing * -4.0)
                        .set("fill", "black")
                        .set(
                            "font-size",
                            self.axis_label_size.unwrap_or(DEFAULT_AXIS_LABEL_SIZE),
                        )
                        .set("text-anchor", "middle"),
                }),
            );
        }

        container
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum HorizontalAlignment {
    Start,
    #[default]
    Middle,
    End,
}

impl HorizontalAlignment {
    pub fn render(&self) -> &'static str {
        match self {
            HorizontalAlignment::Start => "start",
            HorizontalAlignment::Middle => "middle",
            HorizontalAlignment::End => "end",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextProps {
    pub text_size: f64,
    pub horizontal_alignment: HorizontalAlignment,
    pub font_family: String,
    pub color: String
}

impl TextProps {
    pub fn text(&self, text: String) -> Text {
        Text::new(text)
            .set("font-family", self.font_family.clone())
            .set("text-anchor", self.horizontal_alignment.render())
            .set("font-size", format!("{}em", self.text_size))
            .set("fill", self.color.clone())
    }
}

impl Default for TextProps {
    fn default() -> Self {
        TextProps {
            text_size: 1.0,
            horizontal_alignment: HorizontalAlignment::Middle,
            font_family: "serif".to_string(),
            color: "black".to_string()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_make_canvas() {
        let mut canvas: Canvas<f64, f32> = Canvas::new(400, 150);
        canvas.update_scales(
            CoordinateRange::new(0.0, 1000.0),
            CoordinateRange::new(100.0, 0.0),
        );
        assert_eq!(canvas.width, 400);
        assert_eq!(canvas.height, 150);

        assert_eq!(canvas.x_axis.scale.domain.max(), 1000.0);

        assert_eq!(canvas.y_axis.scale.domain.max(), 100.0);
    }

    #[test]
    fn test_axis_props() {
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

        canvas.to_svg(&props, &props2);
    }
}
