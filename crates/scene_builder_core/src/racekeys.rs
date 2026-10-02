use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum RaceKey {
    Human = 0,
    AshHopper,
    Bear,
    Boar,
    BoarMounted,
    BoarSingle,
    Canine,
    Chaurus,
    ChaurusHunter,
    ChaurusReaper,
    Chicken,
    Cow,
    Deer,
    Dog,
    Dragon,
    DragonPriest,
    Draugr,
    DwarvenBallista,
    DwarvenCenturion,
    DwarvenSphere,
    DwarvenSpider,
    Falmer,
    FlameAtronach,
    Fox,
    FrostAtronach,
    Gargoyle,
    Giant,
    GiantSpider,
    Goat,
    Hagraven,
    Hare,
    Horker,
    Horse,
    IceWraith,
    LargeSpider,
    Lurker,
    Mammoth,
    Mudcrab,
    Netch,
    Riekling,
    Sabrecat,
    Seeker,
    Skeever,
    Slaughterfish,
    Spider,
    Spriggan,
    StormAtronach,
    Troll,
    VampireLord,
    Werewolf,
    Wisp,
    Wispmother,
    Wolf,
}

const LEGACY_RACEKEY_PAIRS: &[(&str, &str)] = &[
    ("Humans", "Human"),
    ("Ashhoppers", "Ash Hopper"),
    ("Bears", "Bear"),
    ("BoarsAny", "Boar"),
    ("BoarsMounted", "Boar (Any)"),
    ("Boars", "Boar (Mounted)"),
    ("Canines", "Canine"),
    ("Chaurus", "Chaurus"),
    ("ChaurusHunters", "Chaurus Hunter"),
    ("ChaurusReapers", "Chaurus Reaper"),
    ("Chickens", "Chicken"),
    ("Cows", "Cow"),
    ("Deers", "Deer"),
    ("Dogs", "Dog"),
    ("Dragons", "Dragon"),
    ("DragonPriests", "Dragon Priest"),
    ("Draugrs", "Draugr"),
    ("DwarvenBallistas", "Dwarven Ballista"),
    ("DwarvenCenturions", "Dwarven Centurion"),
    ("DwarvenSpheres", "Dwarven Sphere"),
    ("DwarvenSpiders", "Dwarven Spider"),
    ("Falmers", "Falmer"),
    ("FlameAtronach", "Flame Atronach"),
    ("Foxes", "Fox"),
    ("FrostAtronach", "Frost Atronach"),
    ("Gargoyles", "Gargoyle"),
    ("Giants", "Giant"),
    ("GiantSpiders", "Giant Spider"),
    ("Goats", "Goat"),
    ("Hagravens", "Hagraven"),
    ("Rabbits", "Rabbit"),
    ("Horkers", "Horker"),
    ("Horses", "Horse"),
    ("IceWraiths", "Ice Wraith"),
    ("LargeSpiders", "Large Spider"),
    ("Lurkers", "Lurker"),
    ("Mammoths", "Mammoth"),
    ("Mudcrabs", "Mudcrab"),
    ("Netches", "Netch"),
    ("Rieklings", "Riekling"),
    ("Sabrecats", "Sabrecat"),
    ("Seekers", "Seeker"),
    ("Skeevers", "Skeever"),
    ("Slaughterfishes", "Slaughterfish"),
    ("Spiders", "Spider"),
    ("Spriggans", "Spriggan"),
    ("StormAtronach", "Storm Atronach"),
    ("Trolls", "Troll"),
    ("VampireLords", "Vampire Lord"),
    ("Werewolves", "Werewolf"),
    ("Wisps", "Wisp"),
    ("Wispmothers", "Wispmother"),
    ("Wolves", "Wolf"),
];

pub fn map_legacy_to_racekey(legacykey: &str) -> Result<String, String> {
    let key = legacykey.to_lowercase();
    LEGACY_RACEKEY_PAIRS
        .iter()
        .find(|(legacy, _)| legacy.to_lowercase() == key)
        .map(|(_, racekey)| (*racekey).into())
        .ok_or_else(|| format!("Unrecognized legacy key: {}", legacykey))
}

/// Reverse of [`map_legacy_to_racekey`].
pub fn map_racekey_to_legacy(racekey: &str) -> Result<String, String> {
    LEGACY_RACEKEY_PAIRS
        .iter()
        .find(|(_, rk)| *rk == racekey)
        .map(|(legacy, _)| (*legacy).into())
        .ok_or_else(|| format!("Unrecognized race key for SLAL export: {}", racekey))
}

fn get_race_map() -> HashMap<String, RaceKey> {
    HashMap::from([
        ("Human".into(), RaceKey::Human),
        ("Ash Hopper".into(), RaceKey::AshHopper),
        ("Bear".into(), RaceKey::Bear),
        ("Boar".into(), RaceKey::BoarSingle),
        ("Boar (Any)".into(), RaceKey::Boar),
        ("Boar (Mounted)".into(), RaceKey::BoarMounted),
        ("Canine".into(), RaceKey::Canine),
        ("Chaurus".into(), RaceKey::Chaurus),
        ("Chaurus Hunter".into(), RaceKey::ChaurusHunter),
        ("Chaurus Reaper".into(), RaceKey::ChaurusReaper),
        ("Chicken".into(), RaceKey::Chicken),
        ("Cow".into(), RaceKey::Cow),
        ("Deer".into(), RaceKey::Deer),
        ("Dog".into(), RaceKey::Dog),
        ("Dragon Priest".into(), RaceKey::DragonPriest),
        ("Dragon".into(), RaceKey::Dragon),
        ("Draugr".into(), RaceKey::Draugr),
        ("Dwarven Ballista".into(), RaceKey::DwarvenBallista),
        ("Dwarven Centurion".into(), RaceKey::DwarvenCenturion),
        ("Dwarven Sphere".into(), RaceKey::DwarvenSphere),
        ("Dwarven Spider".into(), RaceKey::DwarvenSpider),
        ("Falmer".into(), RaceKey::Falmer),
        ("Flame Atronach".into(), RaceKey::FlameAtronach),
        ("Fox".into(), RaceKey::Fox),
        ("Frost Atronach".into(), RaceKey::FrostAtronach),
        ("Gargoyle".into(), RaceKey::Gargoyle),
        ("Giant".into(), RaceKey::Giant),
        ("Goat".into(), RaceKey::Goat),
        ("Hagraven".into(), RaceKey::Hagraven),
        ("Horker".into(), RaceKey::Horker),
        ("Horse".into(), RaceKey::Horse),
        ("Ice Wraith".into(), RaceKey::IceWraith),
        ("Lurker".into(), RaceKey::Lurker),
        ("Mammoth".into(), RaceKey::Mammoth),
        ("Mudcrab".into(), RaceKey::Mudcrab),
        ("Netch".into(), RaceKey::Netch),
        ("Rabbit".into(), RaceKey::Hare),
        ("Riekling".into(), RaceKey::Riekling),
        ("Sabrecat".into(), RaceKey::Sabrecat),
        ("Seeker".into(), RaceKey::Seeker),
        ("Skeever".into(), RaceKey::Skeever),
        ("Slaughterfish".into(), RaceKey::Slaughterfish),
        ("Storm Atronach".into(), RaceKey::StormAtronach),
        ("Spider".into(), RaceKey::Spider),
        ("Large Spider".into(), RaceKey::LargeSpider),
        ("Giant Spider".into(), RaceKey::GiantSpider),
        ("Spriggan".into(), RaceKey::Spriggan),
        ("Troll".into(), RaceKey::Troll),
        ("Vampire Lord".into(), RaceKey::VampireLord),
        ("Werewolf".into(), RaceKey::Werewolf),
        ("Wispmother".into(), RaceKey::Wispmother),
        ("Wisp".into(), RaceKey::Wisp),
        ("Wolf".into(), RaceKey::Wolf),
    ])
}

pub fn get_race_keys_string() -> Vec<String> {
    get_race_map()
        .iter()
        .map(|(key, _)| key.clone())
        .collect()
}

pub fn get_race_key_bytes(race: &str) -> Option<u8> {
    get_race_map()
        .get(race)
        .map(|&key| key as u8)
}
