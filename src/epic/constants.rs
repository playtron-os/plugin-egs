use lazy_static::lazy_static;
use std::collections::HashMap;

pub const SUPPORTED_LANGUAGES: [&str; 12] = [
    "brazilian",
    "french",
    "german",
    "italian",
    "polish",
    "russian",
    "spanish",
    "mexican",
    "chinese",
    "japanese",
    "korean",
    "czech",
];

lazy_static! {
    // Convert language names to a config value that can be passed to the legendary --language argument
    pub static ref LANG_CONFIG_VALUES: HashMap<&'static str, &'static str> = HashMap::from([
        ("brazilian", "pt-BR"),
        ("french", "fr"),
        ("german", "de"),
        ("italian", "it"),
        ("polish", "pl"),
        ("russian", "ru"),
        ("spanish", "es-ES"),
        ("mexican", "es-MX"),
        ("chinese", "zh-Hant"),
        ("korean", "ko"),
        ("japanese", "ja"),
        ("czech", "cs"),
    ]);

    // Convert (normalized) language code to language names
    pub static ref LANGUAGE_CODES: HashMap<&'static str, &'static str> = HashMap::from([
        ("br", "brazilian"),
        ("bra", "brazilian"),
        ("pt_br", "brazilian"),
        ("cs", "czech"),
        ("cs_cz", "czech"),
        ("fr", "french"),
        ("fre", "french"),
        ("fr_fr", "french"),
        ("de", "german"),
        ("ger", "german"),
        ("de_de", "german"),
        ("it", "italian"),
        ("ita", "italian"),
        ("it_it", "italian"),
        ("es_mx", "mexican"),
        ("pl", "polish"),
        ("pol", "polish"),
        ("pl_pl", "polish"),
        ("ru", "russian"),
        ("rus", "russian"),
        ("ru_ru", "russian"),
        ("es", "spanish"),
        ("spa", "spanish"),
        ("es_es", "spanish"),
        ("es_419", "mexican"),
        ("mx", "mexican"),
        ("mex", "mexican"),
        ("ko", "korean"),
        ("ko_kr", "korean"),
        ("ja", "japanese"),
        ("ja_jp", "japanese"),
        ("zh_cn", "chinese"),
        ("zh_hans", "chinese"),
        ("zh_hant", "chinese"),
    ]);

}
