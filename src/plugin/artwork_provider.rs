use zbus_macros::interface;

use crate::types::app::ArtworkMetadata;

pub struct ArtworkProvider {}

#[interface(name = "one.playtron.plugin.ArtworkProvider")]
impl ArtworkProvider {
    /// Returns a list of ArtworkMetadata.
    fn get_artwork_metadata(&mut self, item_id: String) -> Vec<ArtworkMetadata> {
        dbg!(item_id);
        vec![]
    }
}
