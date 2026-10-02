use bitflags::bitflags;
use std::collections::HashMap;

macro_rules! bitflags_with_strings {
    ($name:ident: $type:ty { $($variant:ident = $value:expr),* $(,)? }) => {
        bitflags! {
            #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
            pub struct $name: $type {
                $(const $variant = $value;)*
            }
        }

        impl $name {
            // pub fn to_str(self) -> Option<&'static str> {
            //     match self {
            //         $(Self::$variant => Some(stringify!($variant)),)*
            //         _ => None,
            //     }
            // }

            pub fn from_str(s: &str) -> Option<Self> {
                let mapping: HashMap<&str, Self> = [
                    $( (stringify!($variant), Self::$variant) ),*
                ].iter().cloned().collect();

                mapping.get(s).copied()
            }
        }
    };
}

bitflags_with_strings! {
    Furniture: u32 {
        None = 0,

        BedRoll = 1 << 0,
        BedSingle = 1 << 1,
        BedDouble = 1 << 2,

        Wall = 1 << 3,
        Railing = 1 << 4,

        CraftCookingPot = 1 << 5,
        CraftAlchemy = 1 << 6,
        CraftEnchanting = 1 << 7,
        CraftSmithing = 1 << 8,
        CraftAnvil = 1 << 9,
        CraftWorkbench = 1 << 10,
        CraftGrindstone = 1 << 11,

        Table = 1 << 12,
        TableCounter = 1 << 13,

        Chair = 1 << 14,			// No arm, high back
        ChairCommon = 1 << 15,	// Common chair
        ChairWood = 1 << 16,		// Wooden Chair
        ChairBar = 1 << 17,		// Bar stool
        ChairNoble = 1 << 18,		// Noble Chair
        ChairMisc = 1 << 19,		// Unspecified

        Bench = 1 << 20,			// With back
        BenchNoble = 1 << 21,		// Noble Bench (no back, with arm)
        BenchMisc = 1 << 20,		// No specification on back or arm

        Throne = 1 << 22,
        ThroneRiften = 1 << 23,
        ThroneNordic = 1 << 24,

        XCross = 1 << 25,
        Pillory = 1 << 26,
        Pole = 1 << 27,
        Wheel = 1 << 28,
    }
}

pub fn as_furnitre(list: &Vec<String>) -> Furniture {
    list.iter().fold(Furniture::None, |acc, s| {
        acc | Furniture::from_str(s).unwrap_or(Furniture::None)
    })
}
