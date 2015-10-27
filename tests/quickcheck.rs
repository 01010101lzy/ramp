#![feature(augmented_assignments)]

#![feature(plugin)]
#![plugin(quickcheck_macros)]

extern crate gmp;
extern crate quickcheck;

extern crate ramp;

use ramp::Int;
use gmp::mpz::Mpz;
use quickcheck::{Gen, Arbitrary, TestResult};
use std::fmt::Write;

#[cfg(feature = "full-quickcheck")]
const RANGE_MULT: usize = 200;
#[cfg(not(feature = "full-quickcheck"))]
const RANGE_MULT: usize = 20;

// a hex string representing some integer, to be randomly generated by
// quickcheck.
#[derive(Debug, Clone)]
struct BigIntStr(String);

impl BigIntStr {
    // Parse the string into a ramp::Int and a GMP mpz.
    fn parse(&self) -> (Int, Mpz) {
        (Int::from_str_radix(&self.0, 16).unwrap(),
         Mpz::from_str_radix(&self.0, 16).unwrap())
    }
}

impl Arbitrary for BigIntStr {
    fn arbitrary<G: Gen>(g: &mut G) -> BigIntStr {
        let raw_size = g.size();

        let size = std::cmp::max(g.gen_range(raw_size / RANGE_MULT, raw_size * RANGE_MULT),
                                 1);

        // negative numbers are rarer
        let neg = g.gen::<u8>() % 4 == 0;
        let mut string = String::with_capacity(neg as usize + size);

        if neg {
            string.push_str("-")
        }
        // shouldn't start with zero
        let mut first = 0;
        while first == 0 {
            first = g.gen::<u8>() % 16;
        }
        write!(&mut string, "{:x}", first).unwrap();

        for _ in 0..(size - 1) {
            let digit = g.gen::<u8>() % 16;
            write!(&mut string, "{:x}", digit).unwrap();
        }
        BigIntStr(string)
    }

    fn shrink(&self) -> Box<Iterator<Item = Self>> {
        // shrink the "number" by just truncating the string from the
        // end.
        let mut string = self.clone();
        let iter = (0..)
            .map(move |_| {
                // small numbers strink slowly.
                let rate = match string.0.len() {
                    0 => 0,
                    1...10 => 1,
                    11...100 => 5,
                    100...1000 => 25,
                    _ => 125
                };
                for _ in 0..rate {
                    string.0.pop();
                }
                string.clone()
            })
            .take_while(|s| s.0 != "" && s.0 != "-");
        Box::new(iter)
    }
}

// compare a Ramp int and a GMP int via their string representations.
macro_rules! eq {
    ($($r: expr, $g: expr);*) => {
        ::quickcheck::TestResult::from_bool($($r.to_str_radix(16, false) == $g.to_str_radix(16))&&*)
    }
}

// hex parsing/printing is a fundamental assumption of these tests, so
// let's just double check it works
#[quickcheck]
fn to_str_hex_roundtrip(a: BigIntStr) {
    let (ar, _) = a.parse();
    let t = ar.to_str_radix(16, false);
    assert_eq!(t, a.0);
}

// methods
#[quickcheck]
fn abs(a: BigIntStr) -> TestResult {
    let (ar, ag) = a.parse();
    eq!(ar.abs(), ag.abs())
}

#[quickcheck]
fn abs_cmp(a: BigIntStr, b: BigIntStr) {
    let (ar, _) = a.parse();
    let (br, _) = b.parse();

    assert_eq!(ar.abs_cmp(&-&ar),
               std::cmp::Ordering::Equal);
    assert_eq!(br.abs_cmp(&-&br),
               std::cmp::Ordering::Equal);

    assert_eq!(ar.abs_cmp(&br),
               ar.abs().cmp(&br.abs()));

}

#[quickcheck]
fn divmod(a: BigIntStr, b: BigIntStr) -> TestResult {
    let (ar, ag) = a.parse();
    let (br, bg) = b.parse();

    let (qr, rr) = ar.divmod(&br);
    let qg = &ag / &bg;
    let rg = ag % bg;

    eq!(qr, qg;
        rr, rg)
}

#[quickcheck]
fn pow(a: BigIntStr, b: u32) -> TestResult {
    if b > 100 {
        return TestResult::discard();
    }
    let (ar, ag) = a.parse();

    eq!(ar.pow(b as usize), ag.pow(b))
}

#[quickcheck]
fn square(a: BigIntStr) -> TestResult {
    let (ar, ag) = a.parse();
    eq!(ar.square(), ag.pow(2))
}

#[quickcheck]
fn dsquare(a: BigIntStr) -> TestResult {
    let (ar, ag) = a.parse();
    eq!(ar.dsquare(), ag.pow(2))
}

#[quickcheck]
fn negate(a: BigIntStr) -> TestResult {
    let (mut ar, ag) = a.parse();
    ar.negate();
    eq!(ar, -ag)
}

#[quickcheck]
fn is_even(a: BigIntStr) {
    let (ar, ag) = a.parse();

    assert_eq!(ar.is_even(), !ag.tstbit(0));
}

#[quickcheck]
fn trailing_zeros(a: BigIntStr) {
    let (ar, ag) = a.parse();

    let bit = (0..).position(|idx| ag.tstbit(idx)).unwrap();
    assert_eq!(ar.trailing_zeros() as usize, bit);
}

#[quickcheck]
fn count_ones(a: BigIntStr) {
    let (ar, ag) = a.parse();

    assert_eq!(ar.count_ones(),
               ag.popcount());
}

// operators

macro_rules! expr {
    ($e: expr) => { $e }
}
macro_rules! test_binop {
    ($($name: ident: $op: tt, $assign: tt, $allow_zero: expr, $primitives: ident;)*) => {
        $(mod $name {
            #![allow(unused_imports)]
            use ::BigIntStr;
            use quickcheck::TestResult;
            use ramp::ll::limb::{Limb, BaseInt};
            use gmp::mpz::Mpz;

            #[quickcheck]
            fn int_int(a: BigIntStr, b: BigIntStr) -> TestResult {
                let (ar, ag) = a.parse();
                let (br, bg) = b.parse();
                eq!(ar $op br, ag $op bg)
            }

            #[quickcheck]
            fn intref_int(a: BigIntStr, b: BigIntStr) -> TestResult {
                let (ar, ag) = a.parse();
                let (br, bg) = b.parse();

                eq!(&ar $op br, ag $op bg)
            }
            #[quickcheck]
            fn int_intref(a: BigIntStr, b: BigIntStr) -> TestResult {
                let (ar, ag) = a.parse();
                let (br, bg) = b.parse();

                eq!(ar $op &br, ag $op bg)
            }
            #[quickcheck]
            fn intref_intref(a: BigIntStr, b: BigIntStr) -> TestResult {
                let (ar, ag) = a.parse();
                let (br, bg) = b.parse();

                eq!(&ar $op &br, ag $op bg)
            }

            #[cfg($primitives())]
            #[quickcheck]
            fn int_limb(a: BigIntStr, b: BaseInt) -> TestResult {
                if !$allow_zero && b == 0 {
                    return TestResult::discard()
                }
                let (ar, ag) = a.parse();
                let bg = b as u64;

                eq!(ar $op Limb(b), ag $op bg)
            }
            #[cfg($primitives())]
            #[quickcheck]
            fn int_baseint(a: BigIntStr, b: BaseInt) -> TestResult {
                if !$allow_zero && b == 0 {
                    return TestResult::discard()
                }
                let (ar, ag) = a.parse();
                let bg = b as u64;

                eq!(ar $op b, ag $op bg)
            }
            #[cfg($primitives())]
            #[quickcheck]
            fn int_i32(a: BigIntStr, b: i32) -> TestResult {
                if !$allow_zero && b == 0 {
                    return TestResult::discard()
                }
                let (ar, ag) = a.parse();
                let bg = Mpz::from(b);

                eq!(ar $op b, ag $op bg)
            }
            #[cfg($primitives())]
            #[quickcheck]
            fn int_usize(a: BigIntStr, b: usize) -> TestResult {
                if !$allow_zero && b == 0 {
                    return TestResult::discard()
                }
                let (ar, ag) = a.parse();
                let bg = b as u64;

                eq!(ar $op b, ag $op bg)
            }

            #[cfg($primitives())]
            #[quickcheck]
            fn baseint_int(a: BaseInt, b: BigIntStr) -> TestResult {
                let ag = Mpz::from(a);
                let (br, bg) = b.parse();

                eq!(a $op br, ag $op bg)
            }
            #[cfg($primitives())]
            #[quickcheck]
            fn i32_int(a: i32, b: BigIntStr) -> TestResult {
                let ag = Mpz::from(a);
                let (br, bg) = b.parse();

                eq!(a $op br, ag $op bg)
            }
            #[cfg($primitives())]
            #[quickcheck]
            fn usize_int(a: usize, b: BigIntStr) -> TestResult {
                let ag = Mpz::from(a as u64);
                let (br, bg) = b.parse();

                eq!(a $op br, ag $op bg)
            }

            #[quickcheck]
            fn assign_int(a: BigIntStr, b: BigIntStr) -> TestResult {
                let (mut ar, ag) = a.parse();
                let (br, bg) = b.parse();
                expr!(ar $assign br);
                eq!(ar, ag $op bg)
            }
            #[quickcheck]
            fn assign_intref(a: BigIntStr, b: BigIntStr) -> TestResult {
                let (mut ar, ag) = a.parse();
                let (br, bg) = b.parse();
                expr!(ar $assign &br);
                eq!(ar, ag $op bg)
            }

            #[cfg($primitives())]
            #[quickcheck]
            fn assign_limb(a: BigIntStr, b: BaseInt) -> TestResult {
                if !$allow_zero && b == 0 {
                    return TestResult::discard()
                }
                let (mut ar, ag) = a.parse();
                let bg = b as u64;

                expr!(ar $assign Limb(b));
                eq!(ar, ag $op bg)
            }
            #[cfg($primitives())]
            #[quickcheck]
            fn assign_baseint(a: BigIntStr, b: BaseInt) -> TestResult {
                if !$allow_zero && b == 0 {
                    return TestResult::discard()
                }
                let (mut ar, ag) = a.parse();
                let bg = b as u64;

                expr!(ar $assign b);
                eq!(ar, ag $op bg)
            }
            #[cfg($primitives())]
            #[quickcheck]
            fn assign_i32(a: BigIntStr, b: i32) -> TestResult {
                if !$allow_zero && b == 0 {
                    return TestResult::discard()
                }
                let (mut ar, ag) = a.parse();
                let bg = Mpz::from(b);

                expr!(ar $assign b);
                eq!(ar, ag $op bg)
            }
            #[cfg($primitives())]
            #[quickcheck]
            fn assign_usize(a: BigIntStr, b: usize) -> TestResult {
                if !$allow_zero && b == 0 {
                    return TestResult::discard()
                }
                let (mut ar, ag) = a.parse();
                let bg = b as u64;

                expr!(ar $assign b);
                eq!(ar, ag $op bg)
            }
        })*
    }
}

test_binop! {
    add: +, +=, true, all;
    sub: -, -=, true, all;
    mul: *, *=, true, all;
    div: /, /=, false, all;
    // FIXME(#24): rem gives incorrect results
    // rem: %, %=, false, all;
    bitand: &, &=, true, any;
    bitor: |, |=, true, any;
    bitxor: ^, ^=, true, any;
}

mod neg {
    use ::BigIntStr;
    use quickcheck::TestResult;

    #[quickcheck]
    fn int(a: BigIntStr) -> TestResult {
        let (ar, ag) = a.parse();
        eq!(-ar, -ag)
    }
    #[quickcheck]
    fn intref(a: BigIntStr) -> TestResult {
        let (ar, ag) = a.parse();
        eq!(-&ar, -ag)
    }
}


macro_rules! test_shiftop {
    ($($name: ident: $op: tt, $assign: tt;)*) => {
        $(mod $name {
            use ::BigIntStr;
            use quickcheck::TestResult;

            #[quickcheck]
            fn int(a: BigIntStr, b: u16) -> TestResult {
                let (ar, ag) = a.parse();
                let b = b as usize;

                eq!(ar $op b, ag $op b)
            }

            #[quickcheck]
            fn intref(a: BigIntStr, b: u16) -> TestResult {
                let (ar, ag) = a.parse();
                let b = b as usize;

                eq!(&ar $op b, ag $op b)
            }

            #[quickcheck]
            fn assign(a: BigIntStr, b: u16) -> TestResult {
                let (mut ar, ag) = a.parse();
                let b = b as usize;

                expr!(ar $assign b);
                eq!(ar, ag $op b)
            }
        })*
    }
}

test_shiftop! {
    shl: <<, <<=;
    // FIXME(#27): currently >> doesn't match primitives/GMP for negative values
    // shr: >>, >>=;
}

macro_rules! test_cmpop {
    ($($method: ident;)*) => {
        mod cmp {
            mod cmp {
                // special, because Ord doesn't work with primitives
                use ::BigIntStr;

                #[quickcheck]
                fn int_int(a: BigIntStr, b: BigIntStr) {
                    let (ar, ag) = a.parse();
                    let (br, bg) = b.parse();

                    assert_eq!(ar.cmp(&ar),
                               ag.cmp(&ag));
                    assert_eq!(br.cmp(&br),
                               bg.cmp(&bg));

                    assert_eq!(ar.cmp(&br),
                               ag.cmp(&bg));
                }
            }

            $(mod $method {
                use ::BigIntStr;
                use ramp::ll::limb::{Limb, BaseInt};
                use gmp::mpz::Mpz;

                #[quickcheck]
                fn int_int(a: BigIntStr, b: BigIntStr) {
                    let (ar, ag) = a.parse();
                    let (br, bg) = b.parse();

                    assert_eq!(ar.$method(&ar),
                               ag.$method(&ag));
                    assert_eq!(br.$method(&br),
                               bg.$method(&bg));

                    assert_eq!(ar.$method(&br),
                               ag.$method(&bg));
                }

                #[quickcheck]
                fn int_limb(a: BigIntStr, b: BaseInt) {
                    let (ar, ag) = a.parse();
                    let bg = Mpz::from(b);

                    assert_eq!(ar.$method(&Limb(b)),
                               ag.$method(&bg));
                }
                #[quickcheck]
                fn int_i32(a: BigIntStr, b: i32) {
                    let (ar, ag) = a.parse();
                    let bg = Mpz::from(b);

                    assert_eq!(ar.$method(&b),
                               ag.$method(&bg));
                }
                #[quickcheck]
                fn int_usize(a: BigIntStr, b: usize) {
                    let (ar, ag) = a.parse();
                    let bg = Mpz::from(b as u64);

                    assert_eq!(ar.$method(&b),
                               ag.$method(&bg));
                }
            })*
        }
    }
}

test_cmpop! {
    eq;
    ne;
    partial_cmp;
    // cmp; // in macro
    lt;
    le;
    gt;
    ge;
}

// conversions

macro_rules! test_from {
    ($($prim: ident;)*) => {
        mod from {
            use ramp::Int;

            $(#[quickcheck]
              fn $prim(x: $prim) {
                  let a = Int::from(x);
                  assert_eq!(a.to_string(),
                             x.to_string());

                  assert_eq!($prim::from(&a),
                             x);
              })*
        }
    }
}

test_from! {
    i8; i16; i32; i64; isize;
    u8; u16; u32; u64; usize;
}

// stringification

#[quickcheck]
fn to_str_radix(a: BigIntStr, b: u8) -> TestResult {
    // fold, to avoid discarding too many times, but without a bias
    let b = b % 64;
    if b < 2 || b > 36 { return TestResult::discard() }
    let (ar, ag) = a.parse();

    TestResult::from_bool(ar.to_str_radix(b, false) == ag.to_str_radix(b))
}

// decimal is the most common non-trivial (non-power-of-two) base, so
// lets devote some extra attention there.
#[quickcheck]
fn to_str_decimal(a: BigIntStr) {
    let (ar, ag) = a.parse();

    assert_eq!(ar.to_str_radix(10, false),
               ag.to_str_radix(10));
    assert_eq!(ar.to_string(),
               ag.to_string());
}

#[quickcheck]
fn write_radix(a: BigIntStr, b: u8) -> TestResult {
    // fold, to avoid discarding too many times, but without a bias
    let b = b % 64;
    if b < 2 || b > 36 { return TestResult::discard() }
    let (ar, ag) = a.parse();

    let mut v = Vec::new();
    ar.write_radix(&mut v, b, false).unwrap();
    TestResult::from_bool(v == ag.to_str_radix(b).into_bytes())
}

#[quickcheck]
fn from_str_radix(a: BigIntStr, b: u8) -> TestResult {
    // fold, to avoid discarding too many times, but without a bias
    let b = b % 64;
    if b < 2 || b > 36 { return TestResult::discard() }

    let (ar, ag) = a.parse();

    let s = ag.to_str_radix(b);

    let sr = Int::from_str_radix(&s, b);
    TestResult::from_bool(sr.ok() == Some(ar))
}

// focus efforts, as per to_str_decimal
#[quickcheck]
fn from_str_decimal(a: BigIntStr) {
    let (ar, ag) = a.parse();

    let s = ag.to_str_radix(10);

    let sr = Int::from_str_radix(&s, 10).unwrap();
    assert_eq!(sr, ar);

    let sr2 = s.parse::<Int>().unwrap();
    assert_eq!(sr2, ar);
}

mod format {
    use ::BigIntStr;

    #[quickcheck]
    fn display(a: BigIntStr) {
        let (ar, ag) = a.parse();

        assert_eq!(format!("{}", ar),
                   ag.to_str_radix(10))
    }
    #[quickcheck]
    fn debug(a: BigIntStr) {
        let (ar, ag) = a.parse();

        assert_eq!(format!("{:?}", ar),
                   ag.to_str_radix(10))
    }
    #[quickcheck]
    fn binary(a: BigIntStr) {
        let (ar, ag) = a.parse();

        assert_eq!(format!("{:b}", ar),
                   ag.to_str_radix(2))
    }
    #[quickcheck]
    fn octal(a: BigIntStr) {
        let (ar, ag) = a.parse();

        assert_eq!(format!("{:o}", ar),
                   ag.to_str_radix(8))
    }
    #[quickcheck]
    fn lowerhex(a: BigIntStr) {
        let (ar, ag) = a.parse();

        assert_eq!(format!("{:x}", ar),
                   ag.to_str_radix(16))
    }
    #[quickcheck]
    fn upperhex(a: BigIntStr) {
        let (ar, ag) = a.parse();

        assert_eq!(format!("{:X}", ar),
                   ag.to_str_radix(16).to_uppercase())
    }
}
