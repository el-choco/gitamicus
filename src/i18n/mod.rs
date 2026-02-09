use std::collections::HashMap;

#[derive(Clone)]
pub struct I18nService {
    current_lang: String,
    translations: HashMap<String, HashMap<String, String>>,
}

impl I18nService {
    pub fn new(lang: &str) -> Self {
        let mut translations = HashMap::new();

        let de_content = include_str!("../../locales/de-DE/main.ftl");
        let en_content = include_str!("../../locales/en-US/main.ftl");

        translations.insert("de-DE".to_string(), parse_ftl(de_content));
        translations.insert("en-US".to_string(), parse_ftl(en_content));

        I18nService {
            current_lang: lang.to_string(),
            translations,
        }
    }

    pub fn translate(&self, key: &str) -> String {
        if let Some(lang_map) = self.translations.get(&self.current_lang) {
            if let Some(val) = lang_map.get(key) {
                return val.clone();
            }
        }
        
        if let Some(lang_map) = self.translations.get("en-US") {
            if let Some(val) = lang_map.get(key) {
                return val.clone();
            }
        }
        
        key.to_string()
    }
}

fn parse_ftl(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}