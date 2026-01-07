use lazy_static::lazy_static;
use std::collections::HashMap;

pub const SUPPORTED_LANGUAGES: &[&str] = &[
    "pt-BR", "fr-FR", "de-DE", "it-IT", "pl-PL", "ru-RU", "es-ES", "es-MX", "zh-CN", "zh-TW",
    "ja-JP", "ko-KR", "cs-CZ",
];

lazy_static! {
    // Convert language names to a config value that can be passed to the legendary --language argument
    // List of Epic supported languages: https://dev.epicgames.com/docs/epic-games-store/store-presence/localize/supported-languages
    pub static ref LANG_CONFIG_VALUES: HashMap<&'static str, &'static str> = HashMap::from([
        ("pt-BR", "pt-BR"),
        ("pt-PT", "pt"),
        ("fr-FR", "fr"),
        ("de-DE", "de"),
        ("it-IT", "it"),
        ("pl-PL", "pl"),
        ("ru-RU", "ru"),
        ("es-ES", "es-ES"),
        ("es-MX", "es"),
        ("zh-TW", "zh-Hant"),
        ("zh-CN", "zh-CN"),
        ("ja-JP", "ja"),
        ("cs-CZ", "cs"),
        ("ko-KR", "ko")
    ]);

    // Convert (normalized) language code to language names
    pub static ref LANGUAGE_CODES: HashMap<&'static str, &'static str> = HashMap::from([
        ("br", "pt-BR"),
        ("bra", "pt-BR"),
        ("pt_br", "pt-BR"),
        ("cs", "cs-CZ"),
        ("cs_cz", "cs-CZ"),
        ("fr", "fr-FR"),
        ("fre", "fr-FR"),
        ("fr_fr", "fr-FR"),
        ("de", "de-DE"),
        ("ger", "de-DE"),
        ("de_de", "de-DE"),
        ("it", "it-IT"),
        ("ita", "it-IT"),
        ("it_it", "it-IT"),
        ("es_mx", "es-MX"),
        ("pl", "pl-PL"),
        ("pol", "pl-PL"),
        ("pl_pl", "pl-PL"),
        ("ru", "ru-RU"),
        ("rus", "ru-RU"),
        ("ru_ru", "ru-RU"),
        ("es", "es-ES"),
        ("spa", "es-ES"),
        ("es_es", "es-ES"),
        ("es_419", "es-MX"),
        ("mx", "es-MX"),
        ("mex", "es-MX"),
        ("ko", "ko-KR"),
        ("ko_kr", "ko-KR"),
        ("ja", "ja-JP"),
        ("ja_jp", "ja-JP"),
        ("zh_cn", "zh-CN"),
        ("zh_hans", "zh-CN"),
        ("zh_hant", "zh-TW"),
    ]);

}
