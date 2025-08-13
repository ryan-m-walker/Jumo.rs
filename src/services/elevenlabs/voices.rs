pub const FLYNN_VOICE_ID: &str = "OZ5NFxPCh40uGDshxKOi";
pub const KOTA_VOICE_ID: &str = "pvxGJdhknm00gMyYHtET";
pub const ARCHER_VOICE_ID: &str = "Fahco4VZzobUeiPqni1S";
pub const JULES_VOICE_ID: &str = "kIC4kfVqgGXGVwgAx81Z";

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Voice {
    #[default]
    Jules,
    Flynn,
    Kota,
    Archer,
}

pub fn get_voice_id(voice_name: Voice) -> &'static str {
    match voice_name {
        Voice::Flynn => FLYNN_VOICE_ID,
        Voice::Kota => KOTA_VOICE_ID,
        Voice::Archer => ARCHER_VOICE_ID,
        Voice::Jules => JULES_VOICE_ID,
    }
}
