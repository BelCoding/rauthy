// Runtime i18n from `i18n.custom_dir`: additional UI languages and E-Mail
// text overrides loaded from disk at startup, without rebuilding Rauthy.
//
// Layout: `<custom_dir>/<code>/{common.json,admin.json,email.json}` where
// `<code>` is a two-letter lowercase language code. A code that is not
// built-in becomes selectable at runtime (its UI catalogs are served to the
// frontend via `GET /auth/v1/i18n/{common|admin}/{code}`); `email.json` also
// works for built-in codes and then overrides the compiled E-Mail texts.
//
// Everything is loaded exactly once at startup (before the HTTP server
// accepts connections) and leaked into `'static` — the amount of data is
// bounded by the directory content and there is no reload at runtime.

use crate::language::Language;
use crate::rauthy_config::{RauthyConfig, VarsTemplate};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::sync::OnceLock;
use tracing::{error, info, warn};

static I18N_CUSTOM: OnceLock<I18nCustom> = OnceLock::new();
static EMPTY: I18nCustom = I18nCustom {
    langs: Vec::new(),
    common_json: None,
    admin_json: None,
    email: None,
};

#[derive(Debug)]
pub struct I18nCustom {
    /// All custom codes found in the directory (any file present).
    /// Only contains codes that are NOT built-in.
    pub langs: Vec<&'static str>,
    /// code -> raw JSON catalog served to the frontend
    common_json: Option<HashMap<&'static str, &'static str>>,
    admin_json: Option<HashMap<&'static str, &'static str>>,
    /// code (custom OR built-in) -> E-Mail text overrides
    email: Option<HashMap<&'static str, &'static EmailOverride>>,
}

impl I18nCustom {
    pub fn common_json(&self, code: &str) -> Option<&'static str> {
        self.common_json.as_ref()?.get(code).copied()
    }

    pub fn admin_json(&self, code: &str) -> Option<&'static str> {
        self.admin_json.as_ref()?.get(code).copied()
    }

    pub fn common_codes(&self) -> Vec<&'static str> {
        let mut res = self
            .common_json
            .as_ref()
            .map(|m| m.keys().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        res.sort_unstable();
        res
    }

    pub fn admin_codes(&self) -> Vec<&'static str> {
        let mut res = self
            .admin_json
            .as_ref()
            .map(|m| m.keys().copied().collect::<Vec<_>>())
            .unwrap_or_default();
        res.sort_unstable();
        res
    }
}

/// E-Mail texts for one language. Every section is optional — a missing
/// section falls back to the built-in texts (custom codes fall back to
/// English). Field names mirror the built-in structs 1:1.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmailOverride {
    /// chrono format string for timestamps in mails, e.g. "%d/%m/%Y %T (%Z)"
    pub tz_fmt: Option<String>,
    pub password_new: Option<TplOverride>,
    pub password_reset: Option<TplOverride>,
    pub email_registered_already: Option<TplOverride>,
    pub change_info_new: Option<ChangeInfoNewOverride>,
    pub change_info_old: Option<ChangeInfoOldOverride>,
    pub confirm_change: Option<ConfirmChangeOverride>,
    pub confirm_change_html: Option<ConfirmChangeHtmlOverride>,
    pub login_location: Option<LoginLocationOverride>,
    pub reset_info: Option<ResetInfoOverride>,
    /// Converted `password_new` / `password_reset` / `email_registered_already`
    #[serde(skip)]
    pub tpl_password_new: Option<VarsTemplate>,
    #[serde(skip)]
    pub tpl_password_reset: Option<VarsTemplate>,
    #[serde(skip)]
    pub tpl_email_registered_already: Option<VarsTemplate>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TplOverride {
    pub subject: String,
    pub header: String,
    pub text: Option<String>,
    pub click_link: Option<String>,
    pub validity: Option<String>,
    pub expires: Option<String>,
    pub button: Option<String>,
    pub footer: Option<String>,
    pub button_text_request_new: Option<String>,
}

impl From<&TplOverride> for VarsTemplate {
    fn from(t: &TplOverride) -> Self {
        Self {
            subject: t.subject.clone().into(),
            header: t.header.clone().into(),
            text: t.text.clone(),
            click_link: t.click_link.clone().map(Into::into),
            validity: t.validity.clone().map(Into::into),
            expires: t.expires.clone().map(Into::into),
            button: t.button.clone().map(Into::into),
            footer: t.footer.clone(),
            button_text_request_new: t.button_text_request_new.clone().map(Into::into),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeInfoNewOverride {
    pub subject: String,
    pub header: String,
    pub click_link: String,
    pub validity: String,
    pub expires: String,
    pub button_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeInfoOldOverride {
    pub subject: String,
    pub header: String,
    pub change_info: String,
    pub click_link: String,
    pub validity: String,
    pub expires: String,
    pub button_text: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmChangeOverride {
    pub subject: String,
    pub msg: String,
    pub msg_from_admin: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmChangeHtmlOverride {
    pub title: String,
    pub text_changed: String,
    pub text_login: String,
    pub to: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginLocationOverride {
    pub subject: String,
    pub unknown_location: String,
    pub if_invalid: String,
    pub revoke_link: String,
    pub account_link: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetInfoOverride {
    pub subject: String,
    pub expires_1: String,
    pub expires_2: String,
    pub update: String,
    pub button_text: String,
}

const BUILT_IN: [&str; 12] = [
    "de", "en", "es", "fr", "ja", "ko", "nb", "nl", "pt", "ru", "uk", "zhhans",
];

#[inline]
pub fn get() -> &'static I18nCustom {
    I18N_CUSTOM.get().unwrap_or(&EMPTY)
}

/// Exact-match lookup of a custom language code; used by
/// `Language::from(&str)`. Also accepts values like `it-CH` by falling back
/// to the first 2 characters.
pub fn find_code(value: &str) -> Option<&'static str> {
    let reg = get();
    if let Some(code) = reg.langs.iter().find(|c| **c == value) {
        return Some(code);
    }
    let prefix = value.get(..2)?.to_lowercase();
    reg.langs.iter().find(|c| **c == prefix).copied()
}

/// E-Mail overrides for this language, if any.
#[inline]
pub fn email(lang: &Language) -> Option<&'static EmailOverride> {
    I18N_CUSTOM
        .get()?
        .email
        .as_ref()?
        .get(lang.as_str())
        .copied()
}

/// Loads `i18n.custom_dir` (if configured). Must be called once at startup,
/// after the config is initialized and before the HTTP server starts.
/// Invalid files are logged and skipped — they never take Rauthy down.
pub fn init() {
    let Some(dir) = RauthyConfig::get().vars.i18n.custom_dir.as_deref() else {
        return;
    };
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            error!(
                dir,
                ?err,
                "cannot read `i18n.custom_dir` - skipping custom i18n"
            );
            return;
        }
    };

    let mut langs = Vec::new();
    let mut common_json = HashMap::new();
    let mut admin_json = HashMap::new();
    let mut email = HashMap::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(code) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if code.len() != 2 || !code.chars().all(|c| c.is_ascii_lowercase()) {
            warn!(
                code,
                "`i18n.custom_dir` entries must be 2 lowercase letters - skipping"
            );
            continue;
        }
        let is_built_in = BUILT_IN.contains(&code) || code == "zh";
        let code: &'static str = Box::leak(code.to_string().into_boxed_str());
        let mut any = false;

        for (file, map, allow_built_in) in [
            ("common.json", &mut common_json, false),
            ("admin.json", &mut admin_json, false),
        ] {
            let f = path.join(file);
            if !f.exists() {
                continue;
            }
            if is_built_in && !allow_built_in {
                warn!(
                    code,
                    file,
                    "UI catalog overrides for built-in languages are not supported - skipping"
                );
                continue;
            }
            match fs::read_to_string(&f)
                .map_err(|e| e.to_string())
                .and_then(|raw| {
                    serde_json::from_str::<serde_json::Value>(&raw).map_err(|e| e.to_string())
                }) {
                Ok(mut value) => {
                    if !value.is_object() {
                        error!(code, file, "catalog must be a JSON object - skipping");
                        continue;
                    }
                    // the frontend reads `lang` from the catalog - enforce it
                    value["lang"] = serde_json::Value::String(code.to_string());
                    let raw: &'static str = Box::leak(value.to_string().into_boxed_str());
                    map.insert(code, raw);
                    any = true;
                }
                Err(err) => error!(code, file, err, "invalid catalog - skipping"),
            }
        }

        let f = path.join("email.json");
        if f.exists() {
            match fs::read_to_string(&f)
                .map_err(|e| e.to_string())
                .and_then(|raw| {
                    serde_json::from_str::<EmailOverride>(&raw).map_err(|e| e.to_string())
                }) {
                Ok(mut o) => {
                    o.tpl_password_new = convert_tpl(code, "password_new", o.password_new.as_ref());
                    o.tpl_password_reset =
                        convert_tpl(code, "password_reset", o.password_reset.as_ref());
                    o.tpl_email_registered_already = o
                        .email_registered_already
                        .as_ref()
                        .map(|t| VarsTemplate::from(t));
                    email.insert(code, &*Box::leak(Box::new(o)));
                    any = true;
                }
                Err(err) => error!(code, err, "invalid email.json - skipping"),
            }
        }

        if any && !is_built_in {
            langs.push(code);
        }
    }

    langs.sort_unstable();
    info!(
        langs = ?langs,
        email_overrides = email.len(),
        "custom i18n loaded from `i18n.custom_dir`"
    );

    let _ = I18N_CUSTOM.set(I18nCustom {
        langs,
        common_json: Some(common_json),
        admin_json: Some(admin_json),
        email: Some(email),
    });
}

/// `password_new` / `password_reset` mails require these optional
/// `VarsTemplate` fields — validate at load time instead of panicking when
/// the first mail is built.
fn convert_tpl(code: &str, name: &str, t: Option<&TplOverride>) -> Option<VarsTemplate> {
    let t = t?;
    if t.click_link.is_none() || t.validity.is_none() || t.expires.is_none() || t.button.is_none() {
        error!(
            code,
            name,
            "`click_link`, `validity`, `expires` and `button` are required for this template - skipping"
        );
        return None;
    }
    Some(t.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_override_parses_partial_files() {
        let o: EmailOverride = serde_json::from_str(
            r#"{
                "tz_fmt": "%d/%m/%Y %T (%Z)",
                "password_reset": {
                    "subject": "s", "header": "h", "click_link": "c",
                    "validity": "v", "expires": "e", "button": "b"
                },
                "confirm_change": { "subject": "s", "msg": "m", "msg_from_admin": "a" }
            }"#,
        )
        .unwrap();
        assert_eq!(o.tz_fmt.as_deref(), Some("%d/%m/%Y %T (%Z)"));
        assert!(o.password_reset.is_some());
        assert!(o.password_new.is_none());
        assert!(o.confirm_change.is_some());

        let tpl = convert_tpl("xx", "password_reset", o.password_reset.as_ref()).unwrap();
        assert_eq!(tpl.subject.as_ref(), "s");
        assert_eq!(tpl.button.as_deref(), Some("b"));
        // required fields missing -> section is dropped, not a panic later
        let bad: EmailOverride =
            serde_json::from_str(r#"{ "password_new": { "subject": "s", "header": "h" } }"#)
                .unwrap();
        assert!(convert_tpl("xx", "password_new", bad.password_new.as_ref()).is_none());
    }

    #[test]
    fn email_override_rejects_unknown_fields() {
        assert!(serde_json::from_str::<EmailOverride>(r#"{ "no_such_mail": {} }"#).is_err());
    }
}
