#[macro_export]
macro_rules! convert_u32 {
    ( $t:ident, $x:expr ) => {
        let $t = $x[0] as u32 | ($x[1] as u32) << 8 | ($x[2] as u32) << 16 | ($x[3] as u32) << 24;

        #[allow(unused_assignments)]
        {
            $x = &$x[4..];
        }
    };
}
#[macro_export]
macro_rules! convert_u64 {
    ( $t:ident, $x:expr ) => {
        let $t = $x[0] as u64
            | ($x[1] as u64) << 8
            | ($x[2] as u64) << 16
            | ($x[3] as u64) << 24
            | ($x[4] as u64) << 32
            | ($x[5] as u64) << 40
            | ($x[6] as u64) << 48
            | ($x[7] as u64) << 56;

        #[allow(unused_assignments)]
        {
            $x = &$x[8..];
        }
    };
}
#[macro_export]
macro_rules! convert_u16 {
    ( $t:ident, $x:expr ) => {
        let $t = $x[0] as u16 | ($x[1] as u16) << 8;

        #[allow(unused_assignments)]
        {
            $x = &$x[2..];
        }
    };
}

#[macro_export]
macro_rules! convert_u8 {
    ( $t:ident, $x:expr ) => {
        let $t = $x[0];

        #[allow(unused_assignments)]
        {
            $x = &$x[1..];
        }
    };
}

#[macro_export]
macro_rules! convert_single_u32 {
    ( $t:ident, $x:expr ) => {
        let $t = $x[0] as u32 | ($x[1] as u32) << 8 | ($x[2] as u32) << 16 | ($x[3] as u32) << 24;
    };
}

#[macro_export]
macro_rules! convert_single_u16 {
    ( $t:ident, $x:expr ) => {
        let $t = $x[0] as u16 | ($x[1] as u16) << 8;
    };
}

#[macro_export]
macro_rules! convert_buffer {
    ( $t:ident, $x:expr, $len:tt) => {
        let $t = $x[0..$len];

        #[allow(unused_assignments)]
        {
            $x = &$x[$len..];
        }
    };
}

#[macro_export]
macro_rules! convert_to_string {
    ( $t:ident, $x:expr, $len:expr ) => {
        let $t = std::str::from_utf8(&$x[0..$len]).unwrap().to_string();

        #[allow(unused_assignments)]
        {
            $x = &$x[$len..];
        }
    };
}
