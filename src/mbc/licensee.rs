use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub enum Licensee {
    None,
    Unknown,
    Capcom,
    ElectronicArts,
    HudsonSoft,
    BAI,
    KSS,
    Pow,
    PCMComplete,
    SanX,
    KemcoJapan,
    Seta,
    Viacom,
    Nintendo,
    Bandai,
    OceanAcclaim,
    Konami,
    Taito,
    Banpresto,
    UbiSoft,
    Atlus,
    MalibuInteractive,
    Angel,
    Irem,
    Absolute,
    AcclaimEntertainment,
    Activision,
    AmericanSammy,
    HiTechEntertainment,
    LJN,
    Matchbox,
    Mattel,
    MiltonBradley,
    TitusInteractive,
    LucasfilmGames,
    Infogrames,
    Interplay,
    Broderbund,
    SalesCurveLimited,
    Accolade,
    Lozc,
    TokumaShotenIntermedia,
    TsukudaOriginal,
    VideoSystem,
    Yonezawa,
    Kaneko,
    PackInSoft,
    BottomUp,
    KonamiYuGiOh,
    HOTB,
    Jaleco,
    CoconutsJapan,
    EliteSystems,
    ITCEntertainment,
    Yanoman,
    JapanClary,
    VirginGamesLtd,
    KotobukiSystems,
    HectorSoft,
    Entertainmenti,
    Gremlin,
    SpectrumHoloby,
    USGold,
    GameTek,
    ParkPlace,
    Mindscape,
    Romstar,
    NaxatSoft,
    Tradewest,
    OceanSoftware,
    ElectroBrain,
    SculpturedSoftware,
    THQ,
    TriffixEntertainment,
    Microprose,
    Kemco,
    MisawaEntertainment,
    BulletProofSoftware,
    VicTokai,
    Ape,
    IMax,
    ChunsoftCo,
    TsubarayaProductionsCo,
    Varie,
    Arc,
    NihonBussan,
    Tecmo,
    Imagineer,
    Nova,
    HoriElectric,
    Kawada,
    Takara,
    TechnosJapan,
    ToeiAnimation,
    Toho,
    Namco,
    ASCIICorporation,
    SquareEnix,
    HALLaboratory,
    SNK,
    PonyCanyon,
    CultureBrain,
    Sunsoft,
    SonyImagesoft,
    Sammy,
    Squaresoft,
    DataEast,
    Tonkinhouse,
    Koei,
    UFL,
    Ultra,
    Vap,
    UseCorporation,
    Meldac,
    Sofel,
    Quest,
    SigmaEnterprises,
    ASKKodanshaCo,
    CopyaSystem,
    Tomy,
    NCS,
    Human,
    Altron,
    TowaChiki,
    Yutaka,
    Epoch,
    Athena,
    AsmikACEEntertainment,
    Natsume,
    KingRecords,
    Epic,
    IGS,
    AWave,
    ExtremeEntertainment,
    MTO,
    Kodansha
}

impl Licensee {
    pub fn new_licensee(v: &str) -> Self {
        match v {
            "00" => Licensee::None,
            "01" => Licensee::Nintendo,
            "08" => Licensee::Capcom,
            "13" => Licensee::ElectronicArts,
            "18" => Licensee::HudsonSoft,
            "19" => Licensee::BAI,
            "20" => Licensee::KSS,
            "22" => Licensee::Pow,
            "24" => Licensee::PCMComplete,
            "25" => Licensee::SanX,
            "28" => Licensee::KemcoJapan,
            "29" => Licensee::Seta,
            "30" => Licensee::Viacom,
            "31" => Licensee::Nintendo,
            "32" => Licensee::Bandai,
            "33" => Licensee::OceanAcclaim,
            "34" => Licensee::Konami,
            "35" => Licensee::HectorSoft,
            "37" => Licensee::Taito,
            "38" => Licensee::HudsonSoft,
            "39" => Licensee::Banpresto,
            "41" => Licensee::UbiSoft,
            "42" => Licensee::Atlus,
            "44" => Licensee::MalibuInteractive,
            "46" => Licensee::Angel,
            "47" => Licensee::BulletProofSoftware,
            "49" => Licensee::Irem,
            "50" => Licensee::Absolute,
            "51" => Licensee::AcclaimEntertainment,
            "52" => Licensee::Activision,
            "53" => Licensee::AmericanSammy,
            "54" => Licensee::Konami,
            "55" => Licensee::HiTechEntertainment,
            "56" => Licensee::LJN,
            "57" => Licensee::Matchbox,
            "58" => Licensee::Mattel,
            "59" => Licensee::MiltonBradley,
            "60" => Licensee::TitusInteractive,
            "61" => Licensee::VirginGamesLtd,
            "64" => Licensee::LucasfilmGames,
            "67" => Licensee::OceanSoftware,
            "69" => Licensee::ElectronicArts,
            "70" => Licensee::Infogrames,
            "71" => Licensee::Interplay,
            "72" => Licensee::Broderbund,
            "73" => Licensee::SculpturedSoftware,
            "75" => Licensee::SalesCurveLimited,
            "78" => Licensee::THQ,
            "79" => Licensee::Accolade,
            "80" => Licensee::MisawaEntertainment,
            "83" => Licensee::Lozc,
            "86" => Licensee::TokumaShotenIntermedia,
            "87" => Licensee::TsukudaOriginal,
            "91" => Licensee::ChunsoftCo,
            "92" => Licensee::VideoSystem,
            "93" => Licensee::OceanAcclaim,
            "95" => Licensee::Varie,
            "96" => Licensee::Yonezawa,
            "97" => Licensee::Kaneko,
            "99" => Licensee::PackInSoft,
            "9H" => Licensee::BottomUp,
            "A4" => Licensee::KonamiYuGiOh,
            "BL" => Licensee::MTO,
            "DK" => Licensee::Kodansha,
            _ => Licensee::Unknown,
        }
    }

    pub fn old_licensee(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Licensee::None),
            0x01 => Some(Licensee::Nintendo),
            0x08 => Some(Licensee::Capcom),
            0x09 => Some(Licensee::HOTB),
            0x0A => Some(Licensee::Jaleco),
            0x0B => Some(Licensee::CoconutsJapan),
            0x0C => Some(Licensee::EliteSystems),
            0x13 => Some(Licensee::ElectronicArts),
            0x18 => Some(Licensee::HudsonSoft),
            0x19 => Some(Licensee::ITCEntertainment),
            0x1A => Some(Licensee::Yanoman),
            0x1D => Some(Licensee::JapanClary),
            0x1F => Some(Licensee::VirginGamesLtd),
            0x24 => Some(Licensee::PCMComplete),
            0x25 => Some(Licensee::SanX),
            0x28 => Some(Licensee::KotobukiSystems),
            0x29 => Some(Licensee::Seta),
            0x30 => Some(Licensee::Infogrames),
            0x31 => Some(Licensee::Nintendo),
            0x32 => Some(Licensee::Bandai),
            // Indicates that the new licensee code should be used.
            0x33 => None,
            0x34 => Some(Licensee::Konami),
            0x35 => Some(Licensee::HectorSoft),
            0x38 => Some(Licensee::Capcom),
            0x39 => Some(Licensee::Banpresto),
            0x3C => Some(Licensee::Entertainmenti),
            0x3E => Some(Licensee::Gremlin),
            0x41 => Some(Licensee::UbiSoft),
            0x42 => Some(Licensee::Atlus),
            0x44 => Some(Licensee::MalibuInteractive),
            0x46 => Some(Licensee::Angel),
            0x47 => Some(Licensee::SpectrumHoloby),
            0x49 => Some(Licensee::Irem),
            0x4A => Some(Licensee::VirginGamesLtd),
            0x4D => Some(Licensee::MalibuInteractive),
            0x4F => Some(Licensee::USGold),
            0x50 => Some(Licensee::Absolute),
            0x51 => Some(Licensee::AcclaimEntertainment),
            0x52 => Some(Licensee::Activision),
            0x53 => Some(Licensee::AmericanSammy),
            0x54 => Some(Licensee::GameTek),
            0x55 => Some(Licensee::ParkPlace),
            0x56 => Some(Licensee::LJN),
            0x57 => Some(Licensee::Matchbox),
            0x59 => Some(Licensee::MiltonBradley),
            0x5A => Some(Licensee::Mindscape),
            0x5B => Some(Licensee::Romstar),
            0x5C => Some(Licensee::NaxatSoft),
            0x5D => Some(Licensee::Tradewest),
            0x60 => Some(Licensee::TitusInteractive),
            0x61 => Some(Licensee::VirginGamesLtd),
            0x67 => Some(Licensee::OceanSoftware),
            0x69 => Some(Licensee::ElectronicArts),
            0x6E => Some(Licensee::EliteSystems),
            0x6F => Some(Licensee::ElectroBrain),
            0x70 => Some(Licensee::Infogrames),
            0x71 => Some(Licensee::Interplay),
            0x72 => Some(Licensee::Broderbund),
            0x73 => Some(Licensee::SculpturedSoftware),
            0x75 => Some(Licensee::SalesCurveLimited),
            0x78 => Some(Licensee::THQ),
            0x79 => Some(Licensee::Accolade),
            0x7A => Some(Licensee::TriffixEntertainment),
            0x7C => Some(Licensee::Microprose),
            0x7F => Some(Licensee::Kemco),
            0x80 => Some(Licensee::MisawaEntertainment),
            0x83 => Some(Licensee::Lozc),
            0x86 => Some(Licensee::TokumaShotenIntermedia),
            0x8B => Some(Licensee::BulletProofSoftware),
            0x8C => Some(Licensee::VicTokai),
            0x8E => Some(Licensee::Ape),
            0x8F => Some(Licensee::IMax),
            0x91 => Some(Licensee::ChunsoftCo),
            0x92 => Some(Licensee::VideoSystem),
            0x93 => Some(Licensee::TsubarayaProductionsCo),
            0x95 => Some(Licensee::Varie),
            0x96 => Some(Licensee::Yonezawa),
            0x97 => Some(Licensee::Kaneko),
            0x99 => Some(Licensee::Arc),
            0x9A => Some(Licensee::NihonBussan),
            0x9B => Some(Licensee::Tecmo),
            0x9C => Some(Licensee::Imagineer),
            0x9D => Some(Licensee::Banpresto),
            0x9F => Some(Licensee::Nova),
            0xA1 => Some(Licensee::HoriElectric),
            0xA2 => Some(Licensee::Bandai),
            0xA4 => Some(Licensee::Konami),
            0xA6 => Some(Licensee::Kawada),
            0xA7 => Some(Licensee::Takara),
            0xA9 => Some(Licensee::TechnosJapan),
            0xAA => Some(Licensee::Broderbund),
            0xAC => Some(Licensee::ToeiAnimation),
            0xAD => Some(Licensee::Toho),
            0xAF => Some(Licensee::Namco),
            0xB0 => Some(Licensee::AcclaimEntertainment),
            0xB1 => Some(Licensee::ASCIICorporation),
            0xB2 => Some(Licensee::Bandai),
            0xB4 => Some(Licensee::SquareEnix),
            0xB6 => Some(Licensee::HALLaboratory),
            0xB7 => Some(Licensee::SNK),
            0xB9 => Some(Licensee::PonyCanyon),
            0xBA => Some(Licensee::CultureBrain),
            0xBB => Some(Licensee::Sammy),
            0xBD => Some(Licensee::SonyImagesoft),
            0xBF => Some(Licensee::Sammy),
            0xC0 => Some(Licensee::Taito),
            0xC2 => Some(Licensee::Kemco),
            0xC3 => Some(Licensee::Squaresoft),
            0xC4 => Some(Licensee::TokumaShotenIntermedia),
            0xC5 => Some(Licensee::DataEast),
            0xC6 => Some(Licensee::Tonkinhouse),
            0xC8 => Some(Licensee::Koei),
            0xC9 => Some(Licensee::UFL),
            0xCA => Some(Licensee::Ultra),
            0xCB => Some(Licensee::Vap),
            0xCC => Some(Licensee::UseCorporation),
            0xCD => Some(Licensee::Meldac),
            0xCE => Some(Licensee::PonyCanyon),
            0xCF => Some(Licensee::Angel),
            0xD0 => Some(Licensee::Taito),
            0xD1 => Some(Licensee::Sofel),
            0xD2 => Some(Licensee::Quest),
            0xD3 => Some(Licensee::SigmaEnterprises),
            0xD4 => Some(Licensee::ASKKodanshaCo),
            0xD6 => Some(Licensee::NaxatSoft),
            0xD7 => Some(Licensee::CopyaSystem),
            0xD9 => Some(Licensee::Banpresto),
            0xDA => Some(Licensee::Tomy),
            0xDB => Some(Licensee::LJN),
            0xDD => Some(Licensee::NCS),
            0xDE => Some(Licensee::Human),
            0xDF => Some(Licensee::Altron),
            0xE0 => Some(Licensee::Jaleco),
            0xE1 => Some(Licensee::TowaChiki),
            0xE2 => Some(Licensee::Yutaka),
            0xE3 => Some(Licensee::Varie),
            0xE5 => Some(Licensee::Epoch),
            0xE7 => Some(Licensee::Athena),
            0xE8 => Some(Licensee::AsmikACEEntertainment),
            0xE9 => Some(Licensee::Natsume),
            0xEA => Some(Licensee::KingRecords),
            0xEB => Some(Licensee::Atlus),
            0xEC => Some(Licensee::Epic),
            0xEE => Some(Licensee::IGS),
            0xF0 => Some(Licensee::AWave),
            0xF3 => Some(Licensee::ExtremeEntertainment),
            0xFF => Some(Licensee::LJN),
            _ => Some(Licensee::Unknown)
        }
    }
}

impl Display for Licensee {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Licensee::None => "None",
            Licensee::Unknown => "Unknown",
            Licensee::Capcom => "Capcom",
            Licensee::ElectronicArts => "EA (Electronic Arts)",
            Licensee::HudsonSoft => "Hudson Soft",
            Licensee::BAI => "B-AI",
            Licensee::KSS => "KSS",
            Licensee::Pow => "Pow",
            Licensee::PCMComplete => "PCM Complete",
            Licensee::SanX => "San-X",
            Licensee::KemcoJapan => "Kemco Japan",
            Licensee::Seta => "seta",
            Licensee::Viacom => "Viacom",
            Licensee::Nintendo => "Nintendo",
            Licensee::Bandai => "Bandai",
            Licensee::OceanAcclaim => "Ocean/Acclaim",
            Licensee::Konami => "Konami",
            Licensee::Taito => "Taito",
            Licensee::Banpresto => "Banpresto",
            Licensee::UbiSoft => "Ubi Soft",
            Licensee::Atlus => "Atlus",
            Licensee::MalibuInteractive => "Mailbu Interactive",
            Licensee::Angel => "Angel",
            Licensee::Irem => "Irem",
            Licensee::Absolute => "Absolute",
            Licensee::AcclaimEntertainment => "Acclaim Entertainment",
            Licensee::Activision => "Activision",
            Licensee::AmericanSammy => "American Sammy",
            Licensee::HiTechEntertainment => "Hi Tech Entertainment",
            Licensee::LJN => "LJN",
            Licensee::Matchbox => "Matchbox",
            Licensee::Mattel => "Mattel",
            Licensee::MiltonBradley => "Milton Bradley",
            Licensee::TitusInteractive => "Titus",
            Licensee::LucasfilmGames => "Lucasfilm Games",
            Licensee::Infogrames => "Infogrames",
            Licensee::Interplay => "Interplay",
            Licensee::Broderbund => "Broderbund",
            Licensee::SalesCurveLimited => "The Sales Curve Limited",
            Licensee::THQ => "THQ",
            Licensee::Accolade => "Accolade",
            Licensee::Lozc => "lozc",
            Licensee::TokumaShotenIntermedia => "Tokuma Shoten Intermedia",
            Licensee::TsukudaOriginal => "Tsukunda Original",
            Licensee::VideoSystem => "Video System",
            Licensee::Yonezawa => "Yonezawa",
            Licensee::Kaneko => "Kaneko",
            Licensee::PackInSoft => "Pack-In-Soft",
            Licensee::BottomUp => "Bottom Up",
            Licensee::KonamiYuGiOh => "Konami (Yu-Gi-Oh!)",
            Licensee::HOTB => "HOT-B",
            Licensee::Jaleco => "Jaleco",
            Licensee::CoconutsJapan => "Coconuts Japan",
            Licensee::EliteSystems => "Elite Systems",
            Licensee::ITCEntertainment => "ITC Entertainment",
            Licensee::Yanoman => "Yanoman",
            Licensee::JapanClary => "Japan Clary",
            Licensee::VirginGamesLtd => "Virgin Games Ltd.",
            Licensee::KotobukiSystems => "Kotobuki Systems",
            Licensee::HectorSoft => "HectorSoft",
            Licensee::Entertainmenti => "Entertainment i",
            Licensee::Gremlin => "Gremlin",
            Licensee::SpectrumHoloby => "Spectrum Holoby",
            Licensee::USGold => "U.S. Gold",
            Licensee::GameTek => "GameTek",
            Licensee::ParkPlace => "Park Place",
            Licensee::Mindscape => "Mindscape",
            Licensee::Romstar => "Romstar",
            Licensee::NaxatSoft => "Naxat Soft",
            Licensee::Tradewest => "Tradewest",
            Licensee::OceanSoftware => "Ocean Software",
            Licensee::ElectroBrain => "Electro Brain",
            Licensee::SculpturedSoftware => "Sculptured Software",
            Licensee::TriffixEntertainment => "Triffix Entertainment",
            Licensee::Microprose => "Microprose",
            Licensee::Kemco => "Kemco",
            Licensee::MisawaEntertainment => "Misawa Entertainment",
            Licensee::BulletProofSoftware => "Bullet-Proof Software",
            Licensee::VicTokai => "Vic Tokai",
            Licensee::Ape => "Ape",
            Licensee::IMax => "I'Max",
            Licensee::ChunsoftCo => "Chunsoft Co.",
            Licensee::TsubarayaProductionsCo => "Tsubaraya Productions Co.",
            Licensee::Varie => "Varie",
            Licensee::Arc => "Arc",
            Licensee::NihonBussan => "Nihon Bussan",
            Licensee::Tecmo => "Tecmo",
            Licensee::Imagineer => "Imagineer",
            Licensee::Nova => "Nova",
            Licensee::HoriElectric => "Hori Electric",
            Licensee::Kawada => "Kawada",
            Licensee::Takara => "Takara",
            Licensee::TechnosJapan => "Technos Japan",
            Licensee::ToeiAnimation => "Toei Animation",
            Licensee::Toho => "Toho",
            Licensee::Namco => "Namco",
            Licensee::ASCIICorporation => "ASCII Corporation",
            Licensee::SquareEnix => "Square Enix",
            Licensee::HALLaboratory => "HAL Laboratory",
            Licensee::SNK => "SNK",
            Licensee::PonyCanyon => "Pony Canyon",
            Licensee::CultureBrain => "Culture Brain",
            Licensee::Sunsoft => "Sunsoft",
            Licensee::SonyImagesoft => "Sony Imagesoft",
            Licensee::Sammy => "Sammy",
            Licensee::Squaresoft => "Squaresoft",
            Licensee::DataEast => "Data East",
            Licensee::Tonkinhouse => "Tonkinhouse",
            Licensee::Koei => "Koei",
            Licensee::UFL => "UFL",
            Licensee::Ultra => "Ultra",
            Licensee::Vap => "Vap",
            Licensee::UseCorporation => "Use Corporation",
            Licensee::Meldac => "Meldac",
            Licensee::Sofel => "Sofel",
            Licensee::Quest => "Quest",
            Licensee::SigmaEnterprises => "Sigma Enterprises",
            Licensee::ASKKodanshaCo => "ASK Kodansha Co.",
            Licensee::CopyaSystem => "Copya System",
            Licensee::Tomy => "Tomy",
            Licensee::NCS => "NCS",
            Licensee::Human => "Human",
            Licensee::Altron => "Atron",
            Licensee::TowaChiki => "Towa Chiki",
            Licensee::Yutaka => "Yutaka",
            Licensee::Epoch => "Epoch",
            Licensee::Athena => "Athena",
            Licensee::AsmikACEEntertainment => "Asmik ACE Entertainment",
            Licensee::Natsume => "Natsume",
            Licensee::KingRecords => "King Records",
            Licensee::Epic => "Epic",
            Licensee::IGS => "IGS",
            Licensee::AWave => "A Wave",
            Licensee::ExtremeEntertainment => "Extreme Entertainment",
            Licensee::MTO => "MTO",
            Licensee::Kodansha => "Kodansha"
        };

        write!(f, "{}", name)?;
        Ok(())
    }
}