use nalgebra::{self, Affine2, OPoint, RealField};
use num_traits::Float;


#[derive(Debug, Clone)]
pub struct AffineTransform<F: Float + RealField + Copy> {
    matrix: Affine2<F>,
}

impl<F: Float + RealField> Default for AffineTransform<F> {
    fn default() -> Self {
        Self::new(Affine2::identity())
    }
}

#[allow(unused)]
impl<F: Float + RealField + Copy> AffineTransform<F> {
    pub fn new(matrix: Affine2<F>) -> Self {
        Self { matrix }
    }

    pub fn identity() -> Self {
        Self::new(Affine2::identity())
    }

    pub fn transform_point(&self, pt: (F, F)) -> (F, F) {
        let pt = OPoint::from_slice(&[pt.0, pt.1]);
        let tpt = self.matrix.transform_point(&pt);
        let x = tpt.coords[0];
        let y = tpt.coords[1];
        (x, y)
    }

    pub fn inverse_transform_point(&self, pt: (F, F)) -> (F, F) {
        let pt = OPoint::from_slice(&[pt.0, pt.1]);
        let tpt = self.matrix.inverse_transform_point(&pt);
        let x = tpt.coords[0];
        let y = tpt.coords[1];
        (x, y)
    }

    pub fn transform_vector(&self, vec: &[(F, F)]) -> Vec<(F, F)> {
        vec.iter().map(|pt| self.transform_point(*pt)).collect()
    }

    pub fn inverse_transform_vector(&self, vec: &[(F, F)]) -> Vec<(F, F)> {
        vec.iter().map(|pt| self.inverse_transform_point(*pt)).collect()
    }

    pub fn translate(&mut self, x: F, y: F) -> &mut Self {
        let m = self.matrix.matrix_mut_unchecked();
        m[(0, 2)] += x;
        m[(1, 2)] += y;
        self
    }

    pub fn scale(&mut self, x: F, y: F) -> &mut Self {
        let m = self.matrix.matrix_mut_unchecked();
        m[(0, 0)] *= x;
        m[(1, 1)] *= y;
        self
    }

    pub fn rotate_rad(&mut self, theta: F) -> &mut Self {
        let cos_theta = Float::cos(theta);
        let sin_theta = Float::sin(theta);
        let m = self.matrix.matrix_mut_unchecked();
        m[(0, 0)] *= cos_theta;
        m[(1, 1)] *= cos_theta;
        m[(1, 0)] *= sin_theta;
        m[(0, 1)] *= -sin_theta;
        self
    }

    pub fn rotate_deg(&mut self, degrees: F) -> &mut Self {
        let theta = Float::to_radians(degrees);
        self.rotate_rad(theta)
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_translate() {
        let mut t = AffineTransform::identity();
        let pt = (1.0, 2.0);
        let pt2 = t.transform_point(pt);
        assert_eq!(pt, pt2);

        t.translate(3.0, 0.0);
        let pt2 = t.transform_point(pt);

        assert_eq!((4.0, 2.0), pt2);

        let pt3 = t.inverse_transform_point(pt2);
        assert_eq!(pt, pt3);
    }
}