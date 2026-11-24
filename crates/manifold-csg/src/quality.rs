//! Circular tessellation settings.
//!
//! Manifold 4.0 replaced the global circular-quality setters with a
//! [`CircularQuality`] passed to each circular constructor
//! ([`Manifold::sphere`](crate::Manifold::sphere),
//! [`Manifold::cylinder`](crate::Manifold::cylinder),
//! [`Manifold::revolve`](crate::Manifold::revolve),
//! [`CrossSection::circle`](crate::CrossSection::circle) and
//! [`CrossSection::offset`](crate::CrossSection::offset)).

use manifold_csg_sys::*;

/// How finely circles are tessellated.
///
/// Those constructors take `impl Into<CircularQuality>`, so a plain segment
/// count still works: `Manifold::sphere(1.0, 32)` means
/// `CircularQuality { segments: 32, ..Default::default() }`, and `0` means
/// the default quality.
///
/// See upstream's `CircularQuality` in `common.h` for the full rules.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircularQuality {
    /// Number of segments in a full circle. Values below 3 apply no
    /// constraint (a sphere takes any positive count), so the angle and edge
    /// length below decide. A sphere rounds this up to a multiple of four.
    pub segments: i32,
    /// Minimum angle in degrees between consecutive segments.
    pub min_angle: f64,
    /// Minimum segment length.
    pub min_edge_length: f64,
}

impl Default for CircularQuality {
    /// Upstream's defaults: no fixed segment count, a 10 degree minimum angle
    /// and a minimum edge length of 1.
    fn default() -> Self {
        Self {
            segments: 0,
            min_angle: 10.0,
            min_edge_length: 1.0,
        }
    }
}

impl From<i32> for CircularQuality {
    /// A quality with this segment count and the default angle and edge
    /// length.
    fn from(segments: i32) -> Self {
        Self {
            segments,
            ..Self::default()
        }
    }
}

impl CircularQuality {
    /// The quality that gives `divisions` segments across an arc of
    /// `degrees`, e.g. for a partial [`revolve`](crate::Manifold::revolve).
    ///
    /// A full circle has at least 3 segments, so an arc over 120 degrees
    /// gets at least 2 divisions and one over 240 degrees at least 3.
    /// `divisions` below 1 counts as 1. A zero or non-finite `degrees` gives
    /// the default quality.
    #[must_use]
    pub fn for_divisions(divisions: i32, degrees: f64) -> Self {
        // SAFETY: takes plain values and returns a plain struct.
        Self::from_sys(unsafe { manifold_circular_quality_for_divisions(divisions, degrees) })
    }

    /// Number of segments a full circle of `radius` gets at this quality.
    #[must_use]
    pub fn segments_for(&self, radius: f64) -> i32 {
        let q = self.to_sys();
        // SAFETY: &q is valid for the call and only read.
        unsafe { manifold_circular_quality_segments_for(radius, &q) }
    }

    pub(crate) fn to_sys(self) -> ManifoldCircularQuality {
        ManifoldCircularQuality {
            segments: self.segments,
            min_angle: self.min_angle,
            min_edge_length: self.min_edge_length,
        }
    }

    fn from_sys(q: ManifoldCircularQuality) -> Self {
        Self {
            segments: q.segments,
            min_angle: q.min_angle,
            min_edge_length: q.min_edge_length,
        }
    }
}
