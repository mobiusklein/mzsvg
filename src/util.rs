use std::{error::Error, fmt::Display, num::{ParseFloatError, ParseIntError}, ops::{Bound, Range, RangeBounds}, str::FromStr};



#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MZRange {
    pub start: Option<f64>,
    pub end: Option<f64>,
}

impl Display for MZRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let start = self.start.map(|s| s.to_string()).unwrap_or_default();
        let end = self.end.map(|s| s.to_string()).unwrap_or_default();
        write!(f, "{start}-{end}")
    }
}

impl MZRange {
    pub fn new(start: Option<f64>, end: Option<f64>) -> Self {
        Self { start, end }
    }
}

impl Default for MZRange {
    fn default() -> Self {
        Self {
            start: None,
            end: None,
        }
    }
}

#[derive(Debug)]
pub enum MZRangeParseError {
    MalformedStart(ParseFloatError),
    MalformedEnd(ParseFloatError),
}

impl Display for MZRangeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MZRangeParseError::MalformedStart(e) => {
                write!(f, "Failed to parse range start {e}")
            }
            MZRangeParseError::MalformedEnd(e) => {
                write!(f, "Failed to parse range end {e}")
            }
        }
    }
}

impl Error for MZRangeParseError {}

impl FromStr for MZRange {
    type Err = MZRangeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut tokens = if s.contains(' ') {
            s.split(' ')
        } else if s.contains(':') {
            s.split(':')
        } else if s.contains('-') {
            s.split('-')
        } else {
            s.split(' ')
        };
        let start_s = tokens.next().unwrap();
        let start_t = if start_s == "" {
            None
        } else {
            match start_s.parse() {
                Ok(val) => Some(val),
                Err(e) => return Err(MZRangeParseError::MalformedStart(e)),
            }
        };
        let end_s = tokens.next().unwrap();
        let end_t = if end_s == "" {
            None
        } else {
            match end_s.parse() {
                Ok(val) => Some(val),
                Err(e) => return Err(MZRangeParseError::MalformedEnd(e)),
            }
        };
        Ok(MZRange {
            start: start_t,
            end: end_t,
        })
    }
}

impl From<Range<f64>> for MZRange {
    fn from(value: Range<f64>) -> Self {
        Self::new(Some(value.start), Some(value.end))
    }
}

impl RangeBounds<f64> for MZRange {
    fn start_bound(&self) -> Bound<&f64> {
        if let Some(start) = self.start.as_ref() {
            Bound::Included(start)
        } else {
            Bound::Unbounded
        }
    }

    fn end_bound(&self) -> Bound<&f64> {
        if let Some(end) = self.end.as_ref() {
            Bound::Included(end)
        } else {
            Bound::Unbounded
        }
    }
}

impl From<(f64, f64)> for MZRange {
    fn from(value: (f64, f64)) -> Self {
        Self::new(Some(value.0), Some(value.1))
    }
}


#[derive(Debug, Clone, Copy)]
pub struct Dimensions(pub usize, pub usize);

impl Default for Dimensions {
    fn default() -> Self {
        Self(600, 200)
    }
}

impl Display for Dimensions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.0, self.1)
    }
}

impl FromStr for Dimensions {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_dimensions(s)
    }
}

pub fn parse_dimensions(dims: &str) -> Result<Dimensions, ParseIntError> {
    let r: Result<Vec<usize>, ParseIntError> = dims.split("x").map(|s| s.parse::<usize>()).collect();
    let r = r?;

    if r.is_empty() {
        return Err("".parse::<usize>().unwrap_err())
    }

    let dim = if r.len() == 1 {
        let width = r[0];
        Dimensions(width, width)
    } else {
        let width = r[0];
        let height = r[1];
        Dimensions(width, height)
    };
    Ok(dim)
}