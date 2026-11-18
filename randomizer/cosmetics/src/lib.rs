use ootrr_game::{bfres, byml, error, json, murmur, rng, romfs, rsizetable, tex, zs};

#[cfg(test)]
macro_rules! parts {
    ($run:ident, $($name:ident = $k:expr),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                $run($k)
            }
        )+
    };
}

pub mod colors;
pub mod cosmetic;
pub mod sfx;
pub mod slink;
pub mod title_logo;
