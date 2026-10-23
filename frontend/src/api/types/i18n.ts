// Built-in languages, plus any 2-letter code served at runtime from the
// backend's `i18n.custom_dir` (the union keeps autocompletion for built-ins).
export type Language =
    | 'en'
    | 'de'
    | 'es'
    | 'fr'
    | 'ja'
    | 'ko'
    | 'nb'
    | 'nl'
    | 'pt'
    | 'ru'
    | 'uk'
    | 'zh'
    | (string & {});

export interface I18nConfigResponse {
    common: Language[];
    admin: Language[];
}
