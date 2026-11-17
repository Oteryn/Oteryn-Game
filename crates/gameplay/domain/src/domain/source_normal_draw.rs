//! The scoped donor callback law, separate from the existing World draw policy.
//! Crystal201189f src/utils/tools.cpp:442–452 and Canary47dfd51f equivalent:
//! N(0.5,0.25), reject outside[0,1], then lround onto the inclusive bounds.

/// Source callback arithmetic supports an inclusive interval of at most30000.
/// Callers retain their own current source/occurrence authority; this is pure DATA.
pub fn source_normal_draw(draw: u64, minimum: u32, maximum: u32) -> Option<u32> {
    let span = maximum.checked_sub(minimum)?;
    if span > 30_000 {
        return None;
    }
    if span == 0 {
        return Some(minimum);
    }
    let mut lower = 0;
    let mut upper = span;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        // The upper lround boundary is v=(middle+0.5)/span. On the
        // standardized normal axis z=4v-2, always inside[-2,2].
        let z = (4 * i128::from(middle) + 2 - 2 * i128::from(span)) * SCALE / i128::from(span);
        let numerator = normal_integral(z) + INTEGRAL_TWO;
        let denominator = 2 * INTEGRAL_TWO;
        let lhs = u128::from(draw) * u128::try_from(denominator).ok()?;
        let rhs = u128::try_from(numerator).ok()? << 64;
        // Equality belongs to the next integer: source lround rounds a
        // nonnegative half away from zero, rather than ties-to-even.
        if lhs < rhs {
            upper = middle;
        } else {
            lower = middle + 1;
        }
    }
    minimum.checked_add(lower)
}

const SCALE: i128 = 1_i128 << 60;
const INTEGRAL_TWO: i128 = normal_integral(2 * SCALE);

// The normalizing sqrt(2*pi) cancels in the truncated-CDF ratio:
// F(z)=(I(z)+I(2))/(2*I(2)), I(z)=integral_0^z exp(-t*t/2)dt.
// Integrate its alternating Taylor series in Q60. At|z|<=2 each
// un-divided term is at most4*SCALE; multiplication by z*z is at
// most16*SCALE^2<2^125, within i128. Forty terms put the analytic
// remainder below2e-39; integer rounding dominates at Q60 precision.
// No platform floating point/transcendental implementation enters replay.
const fn normal_integral(z: i128) -> i128 {
    if z < 0 {
        return -normal_integral(-z);
    }
    let square = z * z / SCALE;
    let mut term = z;
    let mut sum = z;
    let mut n = 1;
    while n <= 40 {
        term = -(term * square / SCALE) / (2 * n);
        sum += term / (2 * n + 1);
        n += 1;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_source_normal_cpp_cdf_quantiles() {
        for (draw, want) in [
            (0, 2500),
            (u64::MAX / 100, 2519),
            (u64::MAX / 10, 2602),
            (u64::MAX / 4, 2670),
            (u64::MAX / 2, 2750),
            (u64::MAX / 4 * 3, 2830),
            (u64::MAX / 10 * 9, 2898),
            (u64::MAX / 100 * 99, 2981),
            (u64::MAX, 3000),
        ] {
            assert_eq!(
                source_normal_draw(draw, 2500, 3000),
                Some(want),
                "sourceCDF draw{draw}"
            );
        }
    }
    #[test]
    fn srx3_source_normal_lround_half_and_checked_bounds() {
        assert_eq!(source_normal_draw(u64::MAX / 2, 300, 301), Some(300));
        assert_eq!(source_normal_draw(1_u64 << 63, 300, 301), Some(301));
        assert_eq!(source_normal_draw(0, 9, 8), None);
        assert_eq!(source_normal_draw(0, 0, 30001), None);
        assert_eq!(source_normal_draw(u64::MAX, 0, 30000), Some(30000));
        assert_eq!(source_normal_draw(0, u32::MAX, u32::MAX), Some(u32::MAX));
        assert_eq!(
            source_normal_draw(u64::MAX, u32::MAX - 30000, u32::MAX),
            Some(u32::MAX)
        );
    }
}
