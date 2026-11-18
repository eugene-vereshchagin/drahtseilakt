// The 2D curve parameter (type 20ad345d:1afb25b1, CK2dCurve): the progression shape of Bezier Progression and
// Bezier Transform. Points sorted by x, Hermite segments whose tangents come from the points' TCB (tension,
// continuity, bias) values unless the point carries its own. Read from the parameter's sub-chunk, evaluated as
// the original does (a bisection on the segment's x). Hostile data: counts are capped, nothing panics.

use crate::chunk::{Chunk, Words};
use crate::error::{Error, Part, Result};

/// The curve parameter type.
pub const GUID: crate::behavior::Guid = crate::behavior::Guid(0x20ad_345d, 0x1afb_25b1);

/// Most points a curve may have.
pub const MAX_POINTS: usize = 4096;

const FLAG_USER_TANGENTS: u32 = 1;
const FLAG_LINEAR: u32 = 2;
const FLAG_SAVED: u32 = 0x1000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub tension: f32,
    pub continuity: f32,
    pub bias: f32,
    pub pos: [f32; 2],
    pub tan_in: [f32; 2],
    pub tan_out: [f32; 2],
    pub flags: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Curve2d {
    pub points: Vec<Point>,
}

impl Curve2d {
    /// A curve through these points (as saved: unsorted, tangents not computed): sorted by x, tangents computed.
    pub fn new(mut points: Vec<Point>) -> Curve2d {
        points.truncate(MAX_POINTS);
        points.sort_by(|a, b| {
            a.pos[0]
                .partial_cmp(&b.pos[0])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut c = Curve2d { points };
        c.update_tangents();
        c
    }

    /// The curve a 2D curve parameter's sub-chunk holds (current format, section 0x100). `None` for a chunk of
    /// the old format or without the section.
    pub fn read(chunk: &Chunk) -> Result<Option<Curve2d>> {
        let Some(mut w) = chunk.section(0x100) else {
            return Ok(None);
        };
        let count = w.u32()? as usize;
        if count > MAX_POINTS || count > w.remaining() {
            return Err(Error::Malformed {
                part: Part::Data,
                what: "curve point count",
            });
        }
        w.take(count)?;
        let _fitting = w.f32()?;
        let mut points = Vec::new();
        for _ in 0..count {
            let flags = w.u32()?;
            if flags & FLAG_SAVED == 0 {
                continue;
            }
            let mut f = || w.f32();
            let pos = [f()?, f()?];
            let (tension, continuity, bias) = (f()?, f()?, f()?);
            f()?;
            f()?;
            let tan_in = [f()?, f()?];
            let tan_out = [f()?, f()?];
            points.push(Point {
                tension,
                continuity,
                bias,
                pos,
                tan_in,
                tan_out,
                flags: flags & !FLAG_SAVED,
            });
        }
        Ok(Some(Curve2d::new(points)))
    }

    /// The curve of a parameter value saved as a sub-chunk: the 0xe section holds the curve's own chunk.
    pub fn from_parameter(sub: &Chunk) -> Result<Option<Curve2d>> {
        let Some(mut w) = sub.section(0xe) else {
            return Ok(None);
        };
        let inner = Words::sub_chunk(&mut w, sub.chunk_version)?;
        Curve2d::read(&inner)
    }

    /// The tangents of every point (CK2dCurve::UpdatePointsAndTangents).
    fn update_tangents(&mut self) {
        let n = self.points.len();
        if n < 2 {
            return;
        }
        let pos: Vec<[f32; 2]> = self.points.iter().map(|p| p.pos).collect();
        let diff = |a: Option<&[f32; 2]>, b: Option<&[f32; 2]>| match (a, b) {
            (Some(a), Some(b)) => [b[0] - a[0], b[1] - a[1]],
            _ => [0.0f32; 2],
        };
        for (i, p) in self.points.iter_mut().enumerate() {
            let (cur, next) = (pos.get(i), i.checked_add(1).and_then(|j| pos.get(j)));
            let prev = i.checked_sub(1).and_then(|j| pos.get(j));
            let (first, last) = (prev.is_none(), next.is_none());
            let dp = diff(prev, cur);
            let dn = diff(cur, next);
            if p.flags & FLAG_USER_TANGENTS == 0 {
                let (t, c, b) = (p.tension, p.continuity, p.bias);
                let oa = (1.0 + c) * (1.0 + b) * (1.0 - t);
                let ob = (1.0 - t) * (1.0 - c) * (1.0 - b);
                let ia = (1.0 + b) * (1.0 - t) * (1.0 - c);
                let ib = (1.0 + c) * (1.0 - t) * (1.0 - b);
                p.tan_out = [
                    (oa * dp[0] + ob * dn[0]) * 0.5,
                    (oa * dp[1] + ob * dn[1]) * 0.5,
                ];
                p.tan_in = [
                    (ia * dp[0] + ib * dn[0]) * 0.5,
                    (ia * dp[1] + ib * dn[1]) * 0.5,
                ];
            }
            let xp = if dp[0] == 0.0 { 1e-4 } else { dp[0] };
            let xn = if dn[0] == 0.0 { 1e-4 } else { dn[0] };
            rescale(&mut p.tan_in, if first { 1e-4 } else { xp });
            rescale(&mut p.tan_out, if last { 1e-3 } else { xn });
        }
    }

    /// y in [0, 1] at x (clamped to [0, 1]; CK2dCurve::GetY). 0 for a curve of fewer than two points or for an x
    /// beyond the last point.
    pub fn get_y(&self, x: f32) -> f32 {
        let x = if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) };
        let n = self.points.len();
        if n < 2 {
            return 0.0;
        }
        let Some(j) = self.points.iter().position(|p| x <= p.pos[0]) else {
            return 0.0;
        };
        let (Some(a), Some(b)) = (self.points.get(j.saturating_sub(1)), self.points.get(j)) else {
            return 0.0;
        };
        let y = if a.pos[0] == b.pos[0] {
            a.pos[1]
        } else if a.flags & FLAG_LINEAR == 0 {
            let mut s = (x - a.pos[0]) / (b.pos[0] - a.pos[0]);
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            let mut y = 0.0f32;
            let mut it = 0u32;
            loop {
                if it > 999 {
                    // the original leaves its loop here with the values of the last pass
                    break;
                }
                it = it.saturating_add(1);
                let h3 = (s - 2.0) * s + 1.0;
                let h2 = 3.0 - (s + s);
                let h1 = ((s + s) - 3.0) * s * s + 1.0;
                let px = h1 * a.pos[0]
                    + h2 * b.pos[0] * s * s
                    + h3 * a.tan_out[0] * s
                    + (s - 1.0) * b.tan_in[0] * s * s;
                y = h1 * a.pos[1]
                    + h2 * b.pos[1] * s * s
                    + h3 * a.tan_out[1] * s
                    + (s - 1.0) * b.tan_in[1] * s * s;
                s = (hi + lo) * 0.5;
                if px < x {
                    lo = s;
                }
                if x < px {
                    hi = s;
                }
                if (px - x).abs() as f64 <= 1e-5 || px.is_nan() {
                    break;
                }
            }
            y
        } else {
            (x - a.pos[0]) * (b.pos[1] - a.pos[1]) / (b.pos[0] - a.pos[0]) + a.pos[1]
        };
        if y.is_nan() { 0.0 } else { y.clamp(0.0, 1.0) }
    }
}

fn rescale(t: &mut [f32; 2], x: f32) {
    if t[0] != 0.0 {
        let s = x / t[0];
        t[0] *= s;
        t[1] *= s;
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        clippy::panic
    )]
    use super::*;
    use crate::behavior::{Saved, parameter};

    /// A sub-chunk as the parent's words hold it (parent chunk version `pv`, its own version `cv`).
    fn sub(pv: u8, cv: u8, words: &[u32]) -> Vec<u32> {
        let mut head = vec![0u32, u32::from(cv) << 16 | 1, words.len() as u32, 0, 0, 0];
        if pv > 4 {
            head.push(0);
        }
        let mut v = vec![(head.len() + words.len()) as u32];
        v.extend(head);
        v.extend_from_slice(words);
        v
    }

    fn f(x: f32) -> u32 {
        x.to_bits()
    }

    /// A test point: flags, position, in tangent, out tangent.
    type TestPoint = (u32, [f32; 2], [f32; 2], [f32; 2]);

    /// The curve chunk's words: two unused words per point, the fitting, then each point's flags and 11 floats.
    fn curve_words(points: &[TestPoint]) -> Vec<u32> {
        let mut w = vec![0x100, 0, points.len() as u32];
        w.extend(points.iter().map(|_| 0));
        w.push(f(0.0));
        for (flags, pos, tin, tout) in points {
            w.push(flags | 0x1000_0000);
            w.extend([f(pos[0]), f(pos[1]), f(0.0), f(0.0), f(0.0), f(0.0), f(0.0)]);
            w.extend([f(tin[0]), f(tin[1]), f(tout[0]), f(tout[1])]);
        }
        w
    }

    fn chunk_of(words: Vec<u32>, chunk_version: u8) -> Chunk {
        Chunk {
            data_version: 1,
            class_id: 0,
            chunk_version,
            options: 0,
            words,
        }
    }

    #[test]
    fn a_straight_segment_and_a_smoothstep_have_their_hand_computed_values() {
        // linear segment (0,0)-(1,1): y = x
        let c = Curve2d::new(vec![
            Point {
                pos: [0.0, 0.0],
                flags: FLAG_LINEAR,
                ..Point::default()
            },
            Point {
                pos: [1.0, 1.0],
                flags: FLAG_LINEAR,
                ..Point::default()
            },
        ]);
        assert_eq!(c.get_y(0.25), 0.25);
        assert_eq!(c.get_y(0.0), 0.0);
        assert_eq!(c.get_y(1.0), 1.0);
        // outside 0..1 is clamped; NaN reads as 0
        assert_eq!(c.get_y(7.0), 1.0);
        assert_eq!(c.get_y(-3.0), 0.0);
        assert_eq!(c.get_y(f32::NAN), 0.0);
        // user tangents (1, 0) on both ends: x(s) = s and y(s) = (3 - 2 s) s^2, the smoothstep
        let s = Curve2d::new(vec![
            Point {
                pos: [0.0, 0.0],
                tan_out: [1.0, 0.0],
                tan_in: [1.0, 0.0],
                flags: FLAG_USER_TANGENTS,
                ..Point::default()
            },
            Point {
                pos: [1.0, 1.0],
                tan_out: [1.0, 0.0],
                tan_in: [1.0, 0.0],
                flags: FLAG_USER_TANGENTS,
                ..Point::default()
            },
        ]);
        for (x, y) in [(0.25, 0.15625), (0.5, 0.5), (0.75, 0.84375)] {
            assert!((s.get_y(x) - y).abs() < 1e-4, "{x}: {}", s.get_y(x));
        }
        // fewer than two points: 0
        assert_eq!(Curve2d::new(vec![Point::default()]).get_y(0.5), 0.0);
        assert_eq!(Curve2d::default().get_y(0.5), 0.0);
    }

    #[test]
    fn a_curve_is_read_from_a_parameter_saved_as_nested_sub_chunks() {
        let curve = curve_words(&[
            (FLAG_LINEAR, [0.0, 0.0], [0.0, 0.0], [1.0, 1.0]),
            (FLAG_LINEAR, [1.0, 0.5], [1.0, 0.5], [0.0, 0.0]),
        ]);
        for pv in [4u8, 7] {
            // the parameter's chunk: 0x40 = type guid, mode 0, then the type's sub-chunk holding 0xe = the curve's
            let mut inner = vec![0xe, 0];
            inner.extend(sub(7, 7, &curve));
            let mut w = vec![0x40, 0, GUID.0, GUID.1, 0];
            w.extend(sub(pv, 7, &inner));
            let p = parameter(&chunk_of(w, pv)).unwrap();
            let Saved::Curve(c) = p.value else {
                panic!("a curve, got {:?}", p.value)
            };
            assert_eq!(c.points.len(), 2);
            // (0,0)-(1,0.5), linear: y(0.5) = 0.25
            assert_eq!(c.get_y(0.5), 0.25);
        }
    }

    #[test]
    fn hostile_curve_data_is_refused_or_ignored() {
        // an impossible point count, a truncated point list, a sub-chunk bigger than its parent
        let mut bad = vec![0x100, 0, 0xffff_ffff];
        assert!(Curve2d::read(&chunk_of(bad.clone(), 7)).is_err());
        bad = vec![0x100, 0, 3, 0, 0, 0, f(0.0), 0x1000_0000, f(0.0)];
        assert!(Curve2d::read(&chunk_of(bad, 7)).is_err());
        assert_eq!(
            Curve2d::read(&chunk_of(vec![0x200, 0, 1], 7)).unwrap(),
            None
        );
        let mut w = vec![0x40, 0, GUID.0, GUID.1, 0, 9999, 1, 2, 3];
        w.extend([0; 4]);
        let p = parameter(&chunk_of(w, 7)).unwrap();
        assert_eq!(p.value, Saved::SubChunk);
        // far-out coordinates never panic
        let c = Curve2d::new(vec![
            Point {
                pos: [f32::NAN, 1.0],
                ..Point::default()
            },
            Point {
                pos: [f32::INFINITY, f32::NAN],
                ..Point::default()
            },
        ]);
        let _ = c.get_y(0.5);
    }
}
