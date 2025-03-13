pub async fn get_app_from_provider_metadata(
    &self,
    provider_app_id: &str,
) -> ResultWithError<PlaytronApp> {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .or_else(|_err| std::env::var("HOME").map(|var| format!("{var}/.config")))?;
    let metadata_path: PathBuf =
        format!("{config_home}/legendary/metadata/{provider_app_id}.json").parse()?;

    if !metadata_path.exists() {
        return Err(ApiError::AppNotFound.into());
    }

    let game_info_raw = tokio::fs::read_to_string(&metadata_path).await?;
    let game_info: LegendaryGameListInfo = serde_json::from_str(&game_info_raw)?;

    let store_id = game_info.metadata.id.to_owned();
    let namespace = game_info.metadata.namespace.to_owned();
    let developer = game_info.metadata.developer.to_owned();
    let images: Vec<PlaytronImage> = game_info
        .metadata
        .key_images
        .into_iter()
        .map(|epic_image| {
            let image_type = match epic_image.image_type.as_str() {
                "DieselGameBox" => "OfferImageWide",
                "DieselGameBoxTall" => "OfferImageTall",
                _ => &epic_image.image_type,
            }
            .to_owned();
            PlaytronImage {
                url: epic_image.url,
                image_type,
                source: Provider::EpicGames,
                alt: epic_image.alt,
            }
        })
        .collect();

    let mut app_type: Option<PlaytronAppType> = None;
    for category in game_info.metadata.categories {
        match category.path.as_str() {
            "games" => {
                app_type = Some(PlaytronAppType::Game);
                break;
            }
            "addons" => {
                app_type = Some(PlaytronAppType::Dlc);
                break;
            }
            _ => (),
        }
    }
    let app_type = app_type.unwrap_or(PlaytronAppType::None);
    Ok(PlaytronApp {
        images,
        app_type,
        id: format!("playtronlocal-{provider_app_id}"),
        name: game_info.app_title,
        parent_id: None,
        free_to_play: false,
        fetched_at: None,
        sales: vec![],
        coming_soon: false,
        providers: vec![PlaytronProvider {
            namespace,
            store_id,
            provider_app_id: provider_app_id.to_owned(),
            provider: Provider::EpicGames,
            // It's not op to get the slug for the game for the detailed store link
            // we fill store.epicgames.com here as grid is checking if sotre link is specified
            // in order to allow users to even install the game
            product_store_link: "https://store.epicgames.com".to_owned(),
            parent_store_id: None,
            last_imported_timestamp: None,
            known_dlc_store_ids: vec![],
        }],
        developers: vec![developer],
        release_date: None,
        ..Default::default()
    })
}
