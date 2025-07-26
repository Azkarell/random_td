#[macro_export]
macro_rules! def_enum {
    (
        $(#[$attr:meta])*
        $vis:vis $name:ident => $ty:ty {
            $($variant:ident => $val:expr),+
            $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name($ty);

        impl $name {
            $(
                pub const $variant: Self = Self($val);
            )+

            pub const VARIANTS: &'static [Self] = &[$(Self::$variant),+];

            pub const fn get(&self) -> $ty {
                self.0
            }
        }
    };
}
