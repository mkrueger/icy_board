/// BasicReal is a four byte number in BASIC MKS$ format.
pub fn basicreal_to_u32(n: u32) -> u32 {
    let exponent = (n >> 24) as i16;
    if exponent == 0 || exponent == 0x80 {
        return 0;
    }
    let is_negative = (n & 0x80_0000) != 0;

    let value = (n & 0x7F_FFFF) as i32 | 0x80_0000;

    let exponent = exponent - 152;
    let result = if exponent < 0 {
        value.wrapping_shr((-exponent) as u32)
    } else {
        value.wrapping_shl(exponent as u32)
    };

    (if is_negative {
        if result == i32::MIN {
            return 1;
        }
        -result
    } else {
        result
    }) as u32
}

pub fn u32_to_basicreal(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let n = n as i32;
    let is_negative = n < 0;
    let mut value = if is_negative { (-n) as u32 } else { n as u32 };

    let mut exponent = 152u8;

    if value < 0x7F_FFFF {
        while value & 0x80_0000 == 0 {
            value <<= 1;
            exponent = exponent.wrapping_sub(1);
        }
    } else {
        while value & 0xFF00_0000 != 0 {
            value >>= 1;
            exponent = exponent.wrapping_add(1);
        }
    }

    let mut result = value & 0x7F_FFFF;
    if is_negative {
        result |= 0x80_0000;
    }
    result | ((exponent as u32) << 24)
}

#[cfg(test)]
mod tests {
    use crate::util::basic_real::{basicreal_to_u32, u32_to_basicreal};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_basicreal_conversion() {
        for i in -100..100 {
            assert_eq!(i as u32, basicreal_to_u32(u32_to_basicreal(i as u32)));
        }
    }
}
