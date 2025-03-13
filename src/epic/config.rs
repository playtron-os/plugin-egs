use parking_lot::Mutex;

use super::common::get_legendary_command;

lazy_static::lazy_static! {
    static ref LEGENDARY_CONFIG: Mutex<LegendaryConfig> = Mutex::new(LegendaryConfig::default());
}

#[derive(Clone)]
pub struct LegendaryConfig {
    pub legendary_command: String,
}

impl LegendaryConfig {
    pub fn default() -> LegendaryConfig {
        LegendaryConfig {
            legendary_command: get_legendary_command(),
        }
    }

    pub fn new() -> LegendaryConfig {
        LEGENDARY_CONFIG.lock().clone()
    }
}
