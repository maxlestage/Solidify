/// `bigint`: an integer of any size (sign and magnitude, 32-bit limbs, lowest first).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
struct JsBigInt {
    neg: bool,
    mag: Vec<u32>,
}

#[allow(dead_code)]
impl JsBigInt {
    fn normalized(mut self) -> JsBigInt {
        while self.mag.last() == Some(&0) {
            self.mag.pop();
        }
        if self.mag.is_empty() {
            self.neg = false;
        }
        self
    }

    fn from_i64(v: i64) -> JsBigInt {
        let m = v.unsigned_abs();
        JsBigInt { neg: v < 0, mag: vec![m as u32, (m >> 32) as u32] }.normalized()
    }

    /// `BigInt(x)` of a number: it must be an integer.
    fn from_f64(x: f64) -> JsBigInt {
        if x.fract() != 0.0 || !x.is_finite() {
            panic!("RangeError: The number {} cannot be converted to a BigInt because it is not an integer", js_number_text(x));
        }
        let neg = x < 0.0;
        let mut a = x.abs();
        let mut mag = Vec::new();
        while a >= 1.0 {
            let limb = a % 4294967296.0;
            mag.push(limb as u32);
            a = (a - limb) / 4294967296.0;
        }
        JsBigInt { neg, mag }.normalized()
    }

    /// `BigInt("123")`, also `0x…`, `0o…`, `0b…`.
    fn parse(text: &str) -> JsBigInt {
        let t = text.trim();
        let (neg, t) = match t.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, t.strip_prefix('+').unwrap_or(t)),
        };
        let (radix, digits) = match t.get(..2).map(|p| p.to_ascii_lowercase()) {
            Some(p) if p == "0x" => (16, &t[2..]),
            Some(p) if p == "0o" => (8, &t[2..]),
            Some(p) if p == "0b" => (2, &t[2..]),
            _ => (10, t),
        };
        let mut out = JsBigInt::default();
        let r = JsBigInt::from_i64(radix as i64);
        for c in digits.chars().filter(|c| *c != '_') {
            let Some(d) = c.to_digit(radix) else {
                panic!("SyntaxError: Cannot convert {text} to a BigInt");
            };
            out = out.times(&r).plus(&JsBigInt::from_i64(d as i64));
        }
        if t.is_empty() {
            return JsBigInt::default();
        }
        JsBigInt { neg, mag: out.mag }.normalized()
    }

    fn is_zero(&self) -> bool {
        self.mag.is_empty()
    }

    fn cmp_mag(a: &[u32], b: &[u32]) -> std::cmp::Ordering {
        a.len().cmp(&b.len()).then_with(|| {
            for i in (0..a.len()).rev() {
                match a[i].cmp(&b[i]) {
                    std::cmp::Ordering::Equal => continue,
                    o => return o,
                }
            }
            std::cmp::Ordering::Equal
        })
    }

    fn add_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut out = Vec::with_capacity(a.len().max(b.len()) + 1);
        let mut carry = 0u64;
        for i in 0..a.len().max(b.len()) {
            let s = *a.get(i).unwrap_or(&0) as u64 + *b.get(i).unwrap_or(&0) as u64 + carry;
            out.push(s as u32);
            carry = s >> 32;
        }
        if carry > 0 {
            out.push(carry as u32);
        }
        out
    }

    /// `a - b` for `|a| >= |b|`.
    fn sub_mag(a: &[u32], b: &[u32]) -> Vec<u32> {
        let mut out = Vec::with_capacity(a.len());
        let mut borrow = 0i64;
        for i in 0..a.len() {
            let mut d = a[i] as i64 - *b.get(i).unwrap_or(&0) as i64 - borrow;
            borrow = 0;
            if d < 0 {
                d += 1 << 32;
                borrow = 1;
            }
            out.push(d as u32);
        }
        out
    }

    fn plus(&self, other: &JsBigInt) -> JsBigInt {
        if self.neg == other.neg {
            return JsBigInt { neg: self.neg, mag: Self::add_mag(&self.mag, &other.mag) }.normalized();
        }
        match Self::cmp_mag(&self.mag, &other.mag) {
            std::cmp::Ordering::Less => {
                JsBigInt { neg: other.neg, mag: Self::sub_mag(&other.mag, &self.mag) }.normalized()
            }
            _ => JsBigInt { neg: self.neg, mag: Self::sub_mag(&self.mag, &other.mag) }.normalized(),
        }
    }

    fn negated(&self) -> JsBigInt {
        JsBigInt { neg: !self.neg, mag: self.mag.clone() }.normalized()
    }

    fn minus(&self, other: &JsBigInt) -> JsBigInt {
        self.plus(&other.negated())
    }

    fn times(&self, other: &JsBigInt) -> JsBigInt {
        let mut out = vec![0u32; self.mag.len() + other.mag.len()];
        for (i, &a) in self.mag.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in other.mag.iter().enumerate() {
                let t = out[i + j] as u64 + a as u64 * b as u64 + carry;
                out[i + j] = t as u32;
                carry = t >> 32;
            }
            let mut k = i + other.mag.len();
            while carry > 0 {
                let t = out[k] as u64 + carry;
                out[k] = t as u32;
                carry = t >> 32;
                k += 1;
            }
        }
        JsBigInt { neg: self.neg != other.neg, mag: out }.normalized()
    }

    /// Quotient and remainder of magnitudes, bit by bit.
    fn divrem_mag(a: &[u32], b: &[u32]) -> (Vec<u32>, Vec<u32>) {
        if Self::cmp_mag(a, b) == std::cmp::Ordering::Less {
            return (Vec::new(), a.to_vec());
        }
        // A one-limb divisor: limb by limb.
        if b.len() == 1 {
            let d = b[0] as u64;
            let mut q = vec![0u32; a.len()];
            let mut r = 0u64;
            for i in (0..a.len()).rev() {
                let cur = (r << 32) | a[i] as u64;
                q[i] = (cur / d) as u32;
                r = cur % d;
            }
            return (q, vec![r as u32]);
        }
        let bits = a.len() * 32;
        let mut q = vec![0u32; a.len()];
        let mut r: Vec<u32> = Vec::new();
        for i in (0..bits).rev() {
            // r = r * 2 + bit i of a
            let mut carry = (a[i / 32] >> (i % 32)) & 1;
            for limb in r.iter_mut() {
                let next = *limb >> 31;
                *limb = (*limb << 1) | carry;
                carry = next;
            }
            if carry > 0 {
                r.push(carry);
            }
            if Self::cmp_mag(&r, b) != std::cmp::Ordering::Less {
                r = Self::sub_mag(&r, b);
                while r.last() == Some(&0) {
                    r.pop();
                }
                q[i / 32] |= 1 << (i % 32);
            }
        }
        (q, r)
    }

    fn div(&self, other: &JsBigInt) -> JsBigInt {
        if other.is_zero() {
            panic!("RangeError: Division by zero");
        }
        let (q, _) = Self::divrem_mag(&self.mag, &other.mag);
        JsBigInt { neg: self.neg != other.neg, mag: q }.normalized()
    }

    fn rem(&self, other: &JsBigInt) -> JsBigInt {
        if other.is_zero() {
            panic!("RangeError: Division by zero");
        }
        let (_, r) = Self::divrem_mag(&self.mag, &other.mag);
        JsBigInt { neg: self.neg, mag: r }.normalized()
    }

    fn pow(&self, exp: &JsBigInt) -> JsBigInt {
        if exp.neg {
            panic!("RangeError: Exponent must be non-negative");
        }
        let mut result = JsBigInt::from_i64(1);
        let mut base = self.clone();
        let mut e = exp.clone();
        let two = JsBigInt::from_i64(2);
        while !e.is_zero() {
            if e.mag[0] & 1 == 1 {
                result = result.times(&base);
            }
            e = e.div(&two);
            if !e.is_zero() {
                base = base.times(&base);
            }
        }
        result
    }

    /// Two's complement limbs, `n` long.
    fn twos(&self, n: usize) -> Vec<u32> {
        let mut m = self.mag.clone();
        m.resize(n, 0);
        if self.neg {
            for limb in m.iter_mut() {
                *limb = !*limb;
            }
            let mut carry = 1u64;
            for limb in m.iter_mut() {
                let t = *limb as u64 + carry;
                *limb = t as u32;
                carry = t >> 32;
            }
        }
        m
    }

    fn from_twos(mut m: Vec<u32>) -> JsBigInt {
        let neg = m.last().is_some_and(|l| l >> 31 == 1);
        if neg {
            for limb in m.iter_mut() {
                *limb = !*limb;
            }
            let mut carry = 1u64;
            for limb in m.iter_mut() {
                let t = *limb as u64 + carry;
                *limb = t as u32;
                carry = t >> 32;
            }
        }
        JsBigInt { neg, mag: m }.normalized()
    }

    /// `a & b`, `a | b`, `a ^ b`.
    fn bitwise(&self, other: &JsBigInt, op: char) -> JsBigInt {
        let n = self.mag.len().max(other.mag.len()) + 1;
        let (a, b) = (self.twos(n), other.twos(n));
        let m = a
            .iter()
            .zip(&b)
            .map(|(x, y)| match op {
                '&' => x & y,
                '|' => x | y,
                _ => x ^ y,
            })
            .collect();
        Self::from_twos(m)
    }

    /// `a << n` (`a >> n` for a negative `n`): floor division by a power of two.
    fn shift_left(&self, n: i64) -> JsBigInt {
        if n >= 0 {
            self.times(&JsBigInt::from_i64(2).pow(&JsBigInt::from_i64(n)))
        } else {
            let d = JsBigInt::from_i64(2).pow(&JsBigInt::from_i64(-n));
            let q = self.div(&d);
            // Toward minus infinity, like `>>`.
            if self.neg && !self.rem(&d).is_zero() {
                q.minus(&JsBigInt::from_i64(1))
            } else {
                q
            }
        }
    }

    fn to_f64(&self) -> f64 {
        let v = self.mag.iter().rev().fold(0.0, |acc, &l| acc * 4294967296.0 + l as f64);
        if self.neg { -v } else { v }
    }

    fn to_i64(&self) -> i64 {
        self.to_f64() as i64
    }

    fn to_string_radix(&self, radix: u32) -> String {
        if self.is_zero() {
            return "0".into();
        }
        let mut digits = Vec::new();
        let mut m = self.mag.clone();
        let r = vec![radix];
        while !m.is_empty() {
            let (q, rem) = Self::divrem_mag(&m, &r);
            digits.push(std::char::from_digit(*rem.first().unwrap_or(&0), radix).unwrap_or('0'));
            m = q;
            while m.last() == Some(&0) {
                m.pop();
            }
        }
        if self.neg {
            digits.push('-');
        }
        digits.iter().rev().collect()
    }
}

impl std::fmt::Display for JsBigInt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string_radix(10))
    }
}

impl PartialOrd for JsBigInt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JsBigInt {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.neg, other.neg) {
            (false, true) => std::cmp::Ordering::Greater,
            (true, false) => std::cmp::Ordering::Less,
            (false, false) => Self::cmp_mag(&self.mag, &other.mag),
            (true, true) => Self::cmp_mag(&other.mag, &self.mag),
        }
    }
}

impl JsInspect for JsBigInt {
    fn js_value(&self) -> JsValue {
        JsValue::Raw(format!("{self}n"))
    }
}
