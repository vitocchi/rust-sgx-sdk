// Copyright (C) 2017-2019 Baidu, Inc. All Rights Reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions
// are met:
//
//  * Redistributions of source code must retain the above copyright
//    notice, this list of conditions and the following disclaimer.
//  * Redistributions in binary form must reproduce the above copyright
//    notice, this list of conditions and the following disclaimer in
//    the documentation and/or other materials provided with the
//    distribution.
//  * Neither the name of Baidu, Inc., nor the names of its
//    contributors may be used to endorse or promote products derived
//    from this software without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
// "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
// LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
// A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
// OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
// SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
// LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
// THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
// (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

//! Generating numbers between two others.

// this is surprisingly complicated to be both generic & correct

use std::num::Wrapping as w;

use Rng;
use distributions::{Sample, IndependentSample};

/// Sample values uniformly between two bounds.
///
/// This gives a uniform distribution (assuming the RNG used to sample
/// it is itself uniform & the `SampleRange` implementation for the
/// given type is correct), even for edge cases like `low = 0u8`,
/// `high = 170u8`, for which a naive modulo operation would return
/// numbers less than 85 with double the probability to those greater
/// than 85.
///
/// Types should attempt to sample in `[low, high)`, i.e., not
/// including `high`, but this may be very difficult. All the
/// primitive integer types satisfy this property, and the float types
/// normally satisfy it, but rounding may mean `high` can occur.
///
/// # Example
///
/// ```rust
/// use sgx_rand::distributions::{IndependentSample, Range};
///
/// fn main() {
///     let between = Range::new(10, 10000);
///     let mut rng = sgx_rand::thread_rng();
///     let mut sum = 0;
///     for _ in 0..1000 {
///         sum += between.ind_sample(&mut rng);
///     }
///     println!("{}", sum);
/// }
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Range<X> {
    low: X,
    range: X,
    accept_zone: X
}

impl<X: SampleRange + PartialOrd> Range<X> {
    /// Create a new `Range` instance that samples uniformly from
    /// `[low, high)`. Panics if `low >= high`.
    pub fn new(low: X, high: X) -> Range<X> {
        assert!(low < high, "Range::new called with `low >= high`");
        SampleRange::construct_range(low, high)
    }
}

impl<Sup: SampleRange> Sample<Sup> for Range<Sup> {
    #[inline]
    fn sample<R: Rng>(&mut self, rng: &mut R) -> Sup { self.ind_sample(rng) }
}
impl<Sup: SampleRange> IndependentSample<Sup> for Range<Sup> {
    fn ind_sample<R: Rng>(&self, rng: &mut R) -> Sup {
        SampleRange::sample_range(self, rng)
    }
}

/// The helper trait for types that have a sensible way to sample
/// uniformly between two values. This should not be used directly,
/// and is only to facilitate `Range`.
pub trait SampleRange : Sized {
    /// Construct the `Range` object that `sample_range`
    /// requires. This should not ever be called directly, only via
    /// `Range::new`, which will check that `low < high`, so this
    /// function doesn't have to repeat the check.
    fn construct_range(low: Self, high: Self) -> Range<Self>;

    /// Sample a value from the given `Range` with the given `Rng` as
    /// a source of randomness.
    fn sample_range<R: Rng>(r: &Range<Self>, rng: &mut R) -> Self;
}

macro_rules! integer_impl {
    ($ty:ty, $unsigned:ident) => {
        impl SampleRange for $ty {
            // we play free and fast with unsigned vs signed here
            // (when $ty is signed), but that's fine, since the
            // contract of this macro is for $ty and $unsigned to be
            // "bit-equal", so casting between them is a no-op & a
            // bijection.

            #[inline]
            fn construct_range(low: $ty, high: $ty) -> Range<$ty> {
                let range = (w(high as $unsigned) - w(low as $unsigned)).0;
                let unsigned_max: $unsigned = ::std::$unsigned::MAX;

                // this is the largest number that fits into $unsigned
                // that `range` divides evenly, so, if we've sampled
                // `n` uniformly from this region, then `n % range` is
                // uniform in [0, range)
                let zone = unsigned_max - unsigned_max % range;

                Range {
                    low: low,
                    range: range as $ty,
                    accept_zone: zone as $ty
                }
            }
            #[inline]
            fn sample_range<R: Rng>(r: &Range<$ty>, rng: &mut R) -> $ty {
                loop {
                    // rejection sample
                    let v = rng.gen::<$unsigned>();
                    // until we find something that fits into the
                    // region which r.range evenly divides (this will
                    // be uniformly distributed)
                    if v < r.accept_zone as $unsigned {
                        // and return it, with some adjustments
                        return (w(r.low) + w((v % r.range as $unsigned) as $ty)).0;
                    }
                }
            }
        }
    }
}

integer_impl! { i8, u8 }
integer_impl! { i16, u16 }
integer_impl! { i32, u32 }
integer_impl! { i64, u64 }
integer_impl! { isize, usize }
integer_impl! { u8, u8 }
integer_impl! { u16, u16 }
integer_impl! { u32, u32 }
integer_impl! { u64, u64 }
integer_impl! { usize, usize }

macro_rules! float_impl {
    ($ty:ty) => {
        impl SampleRange for $ty {
            fn construct_range(low: $ty, high: $ty) -> Range<$ty> {
                Range {
                    low: low,
                    range: high - low,
                    accept_zone: 0.0 // unused
                }
            }
            fn sample_range<R: Rng>(r: &Range<$ty>, rng: &mut R) -> $ty {
                r.low + r.range * rng.gen::<$ty>()
            }
        }
    }
}

float_impl! { f32 }
float_impl! { f64 }