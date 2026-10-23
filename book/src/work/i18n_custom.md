# Custom i18n at Runtime

Rauthy ships translations for a fixed set of languages, compiled into the
binary and the UI bundle. With `i18n.custom_dir` you can additionally serve
**languages and E-Mail texts from disk at runtime** — no rebuild, and the
files survive image upgrades because they live outside the container:

```toml
[i18n]
custom_dir = '/app/i18n'
```

```
/app/i18n/
├── it/                 # a NEW language: selectable in the UI at runtime
│   ├── common.json     # login / account / register pages
│   ├── admin.json      # Admin UI (optional)
│   └── email.json      # E-Mail texts (optional)
└── de/
    └── email.json      # override the built-in German E-Mail texts
```

Everything is loaded **once at startup**. Invalid files are logged with an
error and skipped — they never prevent Rauthy from starting. Changing files
requires a restart.

## Language codes

Each sub-directory must be named after a **2-letter lowercase code**
(`^[a-z]{2}$`). The code is what ends up in the `locale` cookie, the
`Accept-Language` matching, the `<html lang>` attribute and the user's
stored language. Region variants (`it-CH` in `Accept-Language`) match their
2-letter prefix.

- A code that is **not built-in** (built-ins: `de en es fr ja ko nb nl pt ru uk` +
  `zh`/`zhhans`) registers a new language. It appears in the language
  selector when a `common.json` exists (Admin UI selector: `admin.json`),
  is accepted from the `locale` cookie and `Accept-Language`, is announced
  in `.well-known` `ui_locales_supported`, and can be stored as a user's
  language — mails then use its `email.json`.
- For a **built-in** code, only `email.json` is honored (it overrides the
  compiled E-Mail texts). UI catalogs for built-in languages are compiled
  into the frontend and cannot be overridden at runtime — fix those via a
  normal translation PR instead.

A user whose stored language disappears from the directory silently falls
back to English.

## `common.json` / `admin.json`

The catalogs are the JSON equivalent of the built-in TypeScript catalogs.
The keys mirror, 1:1, the interfaces in

- `frontend/src/i18n/common/interface.ts` for `common.json`
- `frontend/src/i18n/admin/interface.ts` for `admin.json`

The easiest way to create one is to copy `frontend/src/i18n/common/en.ts`,
translate the values, and convert the object literal to JSON. The `lang`
field is overwritten with the directory code by the loader, so you cannot
get it wrong. There is no deep validation: a missing key renders as an
empty value in the UI, so keep the file complete.

The frontend fetches these catalogs from

```
GET /auth/v1/i18n/common/{code}
GET /auth/v1/i18n/admin/{code}
```

(public, cached like `/i18n_config`) — but only for languages that are not
compiled in; built-in languages keep their fully static, synchronous
behavior.

## `email.json`

All sections are optional; a missing section falls back to the built-in
texts (for a new language: to English). Unknown fields are rejected so
typos surface in the log instead of silently falling back.

```json
{
    "tz_fmt": "%d/%m/%Y %T (%Z)",
    "password_new": {
        "subject": "...", "header": "...",
        "click_link": "...", "validity": "...", "expires": "...",
        "button": "...",
        "text": null, "footer": null, "button_text_request_new": null
    },
    "password_reset": { "...": "same fields as password_new" },
    "email_registered_already": {
        "subject": "...", "header": "...", "text": "...",
        "button_text_request_new": "..."
    },
    "change_info_new": {
        "subject": "...", "header": "...", "click_link": "...",
        "validity": "...", "expires": "...", "button_text": "..."
    },
    "change_info_old": {
        "subject": "...", "header": "...", "change_info": "...",
        "click_link": "...", "validity": "...", "expires": "...",
        "button_text": "..."
    },
    "confirm_change": { "subject": "...", "msg": "...", "msg_from_admin": "..." },
    "confirm_change_html": {
        "title": "...", "text_changed": "...", "text_login": "...", "to": "..."
    },
    "login_location": {
        "subject": "...", "unknown_location": "...", "if_invalid": "...",
        "revoke_link": "...", "account_link": "..."
    },
    "reset_info": {
        "subject": "...", "expires_1": "...", "expires_2": "...",
        "update": "...", "button_text": "..."
    }
}
```

`password_new` and `password_reset` require `click_link`, `validity`,
`expires` and `button` — a section missing one of them is rejected at load
time (with an error log) instead of panicking when the first mail is built.
`tz_fmt` is the chrono format for timestamps inside mails, like the
per-language `email.tz_fmt` config.

For the fully custom HTML/text body templates (`[templates]` blocks /
`E-Mail Templates` chapter), the same rules apply as before; `email.json`
covers the *translated strings* level.

## Interaction with `filter_lang_*`

Languages with runtime catalogs are appended to the `i18n_config` response
automatically, so they show up in the selectors without touching
`filter_lang_common` / `filter_lang_admin`. Those filters may also
reference custom codes once the directory provides them.

## For developers

The implementation lives in `src/data/src/i18n_custom.rs` (registry,
loaded once via `i18n_custom::init()` right after the config becomes
static). `Language` carries runtime codes as `Language::Custom(&'static str)`
— codes are interned at startup, which keeps the enum `Copy`; on the wire
and in the database a language is a plain lowercase string, exactly as
before. The API type `rauthy_api_types::generic::Language` accepts unknown
codes as `Custom(String)`; converting to the data-layer `Language` only
succeeds for registered codes (everything else falls back to the default),
so arbitrary API input is never interned.
