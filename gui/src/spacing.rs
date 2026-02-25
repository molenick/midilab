/// Golden-ratio spacing function
pub(crate) const fn spacing(n: i32) -> f32 {
    8.0 * powi_i32(PHI, n)
}

/// Pre-calculated golden ratio for const use
const PHI: f32 = 1.618_034;

const fn powi_i32(mut base: f32, mut exp: i32) -> f32 {
    let mut acc = 1.0;
    if exp < 0 {
        base = 1.0 / base;
        exp = -exp;
    }
    let mut e = exp as u32;
    while e > 0 {
        if e & 1 == 1 {
            acc *= base;
        }
        base *= base;
        e >>= 1;
    }
    acc
}
