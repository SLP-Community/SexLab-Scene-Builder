//! Grouped furniture type options for the scene furniture picker.

pub struct FurnitureGroup {
    pub label: &'static str,
    pub options: &'static [(&'static str, &'static str)], // (human label, flag value)
}

pub const FURNITURE_NONE: (&str, &str) = ("None", "None");

pub const FURNITURE_GROUPS: &[FurnitureGroup] = &[
    FurnitureGroup {
        label: "Beds",
        options: &[
            ("Bed Roll", "BedRoll"),
            ("Bed (Single)", "BedSingle"),
            ("Bed (Double)", "BedDouble"),
        ],
    },
    FurnitureGroup {
        label: "Walls",
        options: &[("Wall", "Wall"), ("Railing", "Railing")],
    },
    FurnitureGroup {
        label: "Crafting",
        options: &[
            ("Cooking Pot", "CraftCookingPot"),
            ("Alchemy Table", "CraftAlchemy"),
            ("Enchanting Table", "CraftEnchanting"),
            ("Smithing Table", "CraftSmithing"),
            ("Anvil", "CraftAnvil"),
            ("Workbench", "CraftWorkbench"),
            ("Grindstone", "CraftGrindstone"),
        ],
    },
    FurnitureGroup {
        label: "Tables",
        options: &[("Common Table", "Table"), ("Bar Counter", "TableCounter")],
    },
    FurnitureGroup {
        label: "Chairs",
        options: &[
            ("Chair (No Armrest, High back)", "Chair"),
            ("Common Chair", "ChairCommon"),
            ("Wooden Chair", "ChairWood"),
            ("Bar Chair", "ChairBar"),
            ("Noble Chair", "ChairNoble"),
            ("Chair (Other)", "ChairMisc"),
        ],
    },
    FurnitureGroup {
        label: "Benches",
        options: &[
            ("Common Bench", "Bench"),
            ("Noble Bench", "BenchNoble"),
            ("Bench (Other)", "BenchMisc"),
        ],
    },
    FurnitureGroup {
        label: "Thrones",
        options: &[
            ("Throne", "Throne"),
            ("Riften Throne", "ThroneRiften"),
            ("Nordic Throne", "ThroneNordic"),
        ],
    },
    FurnitureGroup {
        label: "Contraptions",
        options: &[
            ("XCross", "XCross"),
            ("Pillory", "Pillory"),
            ("Pole", "Pole"),
            ("Wheel", "Wheel"),
        ],
    },
];

/// Human-readable label for a furniture flag value.
pub fn furniture_label(value: &str) -> &str {
    if value == FURNITURE_NONE.1 {
        return FURNITURE_NONE.0;
    }
    for group in FURNITURE_GROUPS {
        for (label, v) in group.options {
            if *v == value {
                return label;
            }
        }
    }
    value
}
