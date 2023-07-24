pub struct Config {
    layers: [LayerConfig; 6]
}

pub struct LayerConfig {
    pub name: [u8; 16],
    pub keys: [KeyConfig; 14]
}

pub struct KeyConfig {
    pub on_press: Option<[u8; 8]>,
    pub on_hold: Option<[u8; 8]>,
    pub colors: [u8; 6]
}