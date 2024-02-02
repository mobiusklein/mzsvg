use std::fmt::{Display, LowerExp};

use num_traits::Float;
use svg::node::element::{path::Data, Group, Path, Text};

use crate::linear::CoordinateRange;

const DEFAULT_FONT_FAMILY: &'static str = "Times New Roman";

#[derive(Debug, Clone)]
pub struct YAxis<T: Float> {
    pub coordinates: CoordinateRange<T>,
    pub label: String,
    pub scale: f64,
}

impl<T: Float + Display> YAxis<T> {
    pub fn new(coordinates: CoordinateRange<T>, label: String, scale: f64) -> Self {
        Self {
            coordinates,
            label,
            scale,
        }
    }

    pub fn transform(&self, value: T) -> f64 {
        let proj = self.coordinates.transform(value);
        proj * self.scale
    }

    #[allow(unused)]
    pub fn inverse(&self, value: f64) -> T {
        (self.coordinates.inverse_transform(value / self.scale))
    }

    pub fn start(&self) -> T {
        self.coordinates.start
    }

    #[allow(unused)]
    pub fn end(&self) -> T {
        self.coordinates.end
    }

    pub fn to_svg<X: Float + Display>(&self, ticks: &AxisLabelOptions, xaxis: &XAxis<X>) -> Group {
        let steps = self.coordinates.size() / T::from(ticks.tick_count).unwrap();

        let mut tick_positions: Vec<_> = (0..ticks.tick_count)
            .into_iter()
            .map(|i| self.coordinates.start + T::from(i).unwrap() * steps)
            .collect();
        tick_positions.push(self.coordinates.end);

        let xcoord = xaxis.transform(X::from(xaxis.start()).unwrap());

        // let axis_label_coord =
        //     xaxis.transform(X::from(xaxis.start() * X::from(0.85).unwrap()).unwrap());

        let axis_label_coord = xaxis.transform(
            xaxis.start() - X::from(xaxis.coordinates.size() * X::from(0.04).unwrap() ).unwrap(),
        );

        let tick_length = xaxis.transform(
            xaxis.start() - X::from(xaxis.coordinates.size() * X::from(0.005).unwrap() ).unwrap(),
        );

        let ystart = self.transform(self.start());
        let state = Data::new().move_to((xcoord, ystart));
        let it = tick_positions.iter();
        let (state, tick_labels) = it.fold(
            (state, Vec::new()),
            |(state, mut labels), next| {
                let raw_next = next;
                let next = self.transform(*next);

                let state = state
                    .line_to((xcoord, next))
                    .line_to((tick_length, next))
                    .line_to((xcoord, next));

                let label = ticks
                    .tick_style
                    .format(&raw_next.to_f64().unwrap(), &self.coordinates.to_f64());

                labels.push(
                    Text::new()
                        .add(svg::node::Text::new(label))
                        .set("x", tick_length)
                        .set("y", next)
                        .set("font-size", ticks.tick_font_size)
                        .set("text-anchor", "end")
                        .set("dominant-baseline", "middle")
                        .set("font-family", DEFAULT_FONT_FAMILY)
                );

                (state, labels)
            },
        );

        let state = state
            .line_to((xcoord, self.transform(self.coordinates.end)))
            .close();

        let axis_label = Text::new()
            .add(svg::node::Text::new(&self.label))
            .set("x", 0)
            .set("y", 0)
            .set(
                "transform",
                format!(
                    "translate({}, {})rotate(-90)",
                    axis_label_coord,
                    self.transform(
                        self.coordinates.size() / T::from(2.0).unwrap() + self.coordinates.max(),
                    )
                ),
            )
            .set("font-size", ticks.label_font_size)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "hanging")
            .set("font-family", DEFAULT_FONT_FAMILY);

        let path = Path::new()
            .set("fill", "none")
            .set("stroke", "black")
            .set("stroke-width", 2.5)
            .set("d", state);
        let mut group = Group::new()
            .add(path)
            .add(axis_label)
            .set("class", "y-axis");
        group = tick_labels
            .into_iter()
            .fold(group, |group, label| group.add(label));
        group
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisTickLabelStyle {
    Precision(usize),
    #[allow(unused)]
    SciNot(usize),
    Percentile(usize),
}

impl AxisTickLabelStyle {
    pub fn format<F: Float + Display + LowerExp>(
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

impl Default for AxisTickLabelStyle {
    fn default() -> Self {
        AxisTickLabelStyle::Precision(2)
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
pub struct XAxis<T: Float> {
    pub coordinates: CoordinateRange<T>,
    pub label: String,
    pub scale: f64,
}

impl<T: Float> XAxis<T> {
    pub fn new(coordinates: CoordinateRange<T>, label: String, scale: f64) -> Self {
        Self {
            coordinates,
            label,
            scale,
        }
    }

    pub fn transform(&self, value: T) -> f64 {
        self.coordinates.transform(value) * self.scale
    }

    #[allow(unused)]
    pub fn inverse(&self, value: f64) -> T {
        self.coordinates.inverse_transform(value / self.scale)
    }

    pub fn start(&self) -> T {
        self.coordinates.start
    }

    pub fn end(&self) -> T {
        self.coordinates.end
    }

    pub fn to_svg<Y: Float + Display>(&self, ticks: &AxisLabelOptions, yaxis: &YAxis<Y>) -> Group {
        let steps = self.coordinates.size() / T::from(ticks.tick_count).unwrap();

        let mut tick_positions: Vec<_> = (0..ticks.tick_count)
            .into_iter()
            .map(|i| self.coordinates.start + T::from(i).unwrap() * steps)
            .collect();
        tick_positions.push(self.coordinates.end);

        let ycoord = yaxis.transform(Y::from(-0.1).unwrap());

        let tick_length = yaxis.transform(
            Y::one() - Y::from(yaxis.coordinates.max() * Y::from(0.01).unwrap()).unwrap(),
        );
        let axis_label_coord = yaxis.transform(
            Y::one() - Y::from(yaxis.coordinates.max() * Y::from(0.03).unwrap()).unwrap(),
        );
        let xstart = self.transform(self.coordinates.start);
        let state = Data::new().move_to((xstart, ycoord));
        let it = tick_positions.iter();
        let (state, tick_labels) = it.fold(
            (state, Vec::new()),
            |(state, mut labels), next| {
                let raw_next = next;
                let next = self.transform(*next);

                let state = state
                    .line_to((next, ycoord))
                    .line_to((next, tick_length))
                    .line_to((next, ycoord));

                let label = ticks
                    .tick_style
                    .format(&raw_next.to_f64().unwrap(), &self.coordinates.to_f64());

                labels.push(
                    Text::new()
                        .add(svg::node::Text::new(label))
                        .set("x", next)
                        .set("y", tick_length)
                        .set("font-size", ticks.tick_font_size)
                        .set("text-anchor", "middle")
                        .set("dominant-baseline", "hanging")
                        .set("font-family", DEFAULT_FONT_FAMILY)
                );

                (state, labels)
            },
        );

        let axis_label = Text::new()
            .add(svg::node::Text::new(&self.label))
            .set(
                "x",
                self.transform(
                    self.coordinates.size() / T::from(2.0).unwrap() + self.coordinates.start,
                ),
            )
            .set("y", axis_label_coord)
            .set("font-size", ticks.label_font_size)
            .set("text-anchor", "middle")
            .set("dominant-baseline", "hanging")
            .set("font-family", DEFAULT_FONT_FAMILY);

        let state = state
            .line_to((self.transform(self.coordinates.end), ycoord))
            .close();

        let path = Path::new()
            .set("fill", "none")
            .set("stroke", "black")
            .set("stroke-width", 2.5)
            .set("d", state);
        let mut group = Group::new()
            .add(path)
            .add(axis_label)
            .set("class", "x-axis");
        group = tick_labels
            .into_iter()
            .fold(group, |group, label| group.add(label));
        group
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_yaxis() {
        let ax = YAxis::new((..100.0).into(), "Rel. Intensity".to_string(), 1.0);

        let v100 = ax.transform(100.0);
        let r100 = ax.inverse(v100);

        eprintln!("{v100} {r100}");
        let v50 = ax.transform(50.0);
        let r50 = ax.inverse(v50);
        eprintln!("{v50} {r50}");
    }

    #[test]
    fn test_xaxis() {}
}
