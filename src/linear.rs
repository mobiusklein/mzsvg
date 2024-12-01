use num_traits::Float;

#[derive(Debug, Default, Clone, Copy)]
pub struct CoordinateRange<T: Float> {
    pub start: T,
    pub end: T,
}

impl<T: Float> CoordinateRange<T> {
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }

    pub fn is_well_formed(&self) -> bool {
        (self.start.is_finite() && !self.start.is_nan()) && (self.end.is_finite() && !self.end.is_nan())
    }

    pub fn size(&self) -> T {
        self.end - self.start
    }

    pub fn transform(&self, value: T) -> f64 {
        ((value - self.start) / self.size()).to_f64().unwrap()
    }

    pub fn inverse_transform(&self, value: f64) -> T {
        let v = T::from(value).unwrap();
        (v * self.size()) + self.start
    }

    pub fn to_f64(&self) -> CoordinateRange<f64> {
        CoordinateRange::new(
            self.start.to_f64().unwrap(),
            self.end.to_f64().unwrap()
        )
    }

    #[allow(unused)]
    pub fn min(&self) -> T {
        self.start.min(self.end)
    }

    pub fn max(&self) -> T {
        self.start.max(self.end)
    }

    pub fn clamp(&self, value: T) -> T {
        let (min, max) = if self.start < self.end {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        };

        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }
}

impl<T: Float> From<(T, T)> for CoordinateRange<T> {
    fn from(value: (T, T)) -> Self {
        CoordinateRange::new(value.0, value.1)
    }
}

impl<T: Float> From<core::ops::Range<T>> for CoordinateRange<T> {
    fn from(value: core::ops::Range<T>) -> Self {
        CoordinateRange::new(value.start, value.end)
    }
}

impl<T: Float> From<core::ops::RangeTo<T>> for CoordinateRange<T> {
    fn from(value: core::ops::RangeTo<T>) -> Self {
        CoordinateRange::new(T::zero(), value.end)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Scale<T: Float> {
    pub domain: CoordinateRange<T>,
    pub range: CoordinateRange<T>,
}

#[allow(unused)]
impl<T: Float> Scale<T> {
    pub fn is_well_formed(&self) -> bool {
        self.domain.is_well_formed() && self.range.is_well_formed()
    }

    pub fn new(domain: CoordinateRange<T>, range: CoordinateRange<T>) -> Self {
        Self { domain, range }
    }

    pub fn transform(&self, value: T) -> T {
        let i = self.domain.transform(value);
        self.range.inverse_transform(i)
    }

    pub fn inverse_transform(&self, value: T) -> T {
        let i = self.range.transform(value);
        self.domain.inverse_transform(i)
    }
}


#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum SVGTransform {
    #[default]
    None,
    Translate(f64, f64),
    Scale(f64, f64),
    Rotate(f64),
    RotateAround(f64, f64, f64),
}

impl SVGTransform {
    pub fn to_svg(&self) -> String {
        match self {
            SVGTransform::None => "".into(),
            SVGTransform::Translate(x, y) => format!("translate({x} {y})"),
            SVGTransform::Scale(x, y) => format!("scale({x} {y})"),
            SVGTransform::Rotate(a) => format!("rotate({a})"),
            SVGTransform::RotateAround(a, x, y) => format!("rotate({a} {x} {y})"),
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_coordinate_range() {
        let c = CoordinateRange::from((0.0, 100.0));
        let p = c.transform(50.0);
        assert_eq!(p, 0.5);

        let p = c.transform(150.0);
        assert_eq!(p, 1.5);

        let c = CoordinateRange::from((20.0, 120.0));
        let p = c.transform(70.0);
        assert_eq!(p, 0.5);
    }

    #[test]
    fn test_coordinate_range_inverse() {
        let c = CoordinateRange::from((0.0, 160.0));

        let p = c.inverse_transform(0.5);
        assert_eq!(p, 80.0);

        let c = CoordinateRange::from((20.0, 120.0));
        let p = c.inverse_transform(0.5);
        assert_eq!(p, 70.0);
    }

    #[test]
    fn test_coordinate_range_hi_lo() {
        let c = CoordinateRange::from((100.0, 0.0));

        let p = c.transform(20.0);
        assert_eq!(p, 0.8);
    }
}