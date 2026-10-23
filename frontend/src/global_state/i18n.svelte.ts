import { useLang } from '$state/language.svelte';
import { type I18n } from '../i18n/common/interface';
import { I18nEn } from '../i18n/common/en';
import { I18nEs } from '../i18n/common/es';
import { I18nDe } from '../i18n/common/de';
import { I18nFr } from '../i18n/common/fr';
import { I18nJa } from '../i18n/common/ja';
import { I18nKo } from '../i18n/common/ko';
import { I18nNb } from '../i18n/common/nb';
import { I18nRu } from '../i18n/common/ru';
import { I18nNl } from '../i18n/common/nl';
import { I18nPt } from '../i18n/common/pt';
import { I18nUk } from '../i18n/common/uk';
import { I18nZh } from '../i18n/common/zh';

// This hack makes typescript happy and is fine as long as
// we `initI18n()` as the very first thing in +layout
let _i18n: I18n = undefined as any as I18n;

// IMPORTANT:
//
// It is not possible to set a new value to the returned rune without a page reload!
// Such an API is usually a very bad idea because you can easily mess it up.
// However, in this case we want it like that on purpose.
//
// We only ever set the i18n_email in the initial routing or inside the <LangSelector>.
// A manual page reload is only necessary in the <LangSelector> though which is a
// small price when we never need a `get()` in so many other places where we just read
// the value.
//
// For the same reason, we don't even need a rune here.
export function useI18n(): I18n {
    return _i18n;
}

const i18nMap: Record<string, I18n> = {
    de: I18nDe,
    en: I18nEn,
    es: I18nEs,
    fr: I18nFr,
    ja: I18nJa,
    ko: I18nKo,
    nb: I18nNb,
    nl: I18nNl,
    pt: I18nPt,
    ru: I18nRu,
    uk: I18nUk,
    zh: I18nZh,
};

/**
 * Sync (and returns nothing) for every compiled language. For a language
 * served from the backend's `i18n.custom_dir`, the catalog is fetched and a
 * Promise is returned — the layout delays rendering its children until it
 * resolves, so every component still reads a final, stable value.
 */
export function initI18n(): void | Promise<void> {
    let lang = useLang();
    let compiled = i18nMap[lang];
    if (compiled) {
        _i18n = compiled;
        return;
    }
    // fallback in case the fetch fails - never render without a catalog
    _i18n = I18nEn;
    return fetch(`/auth/v1/i18n/common/${lang}`)
        .then(async res => {
            if (res.ok) {
                _i18n = (await res.json()) as I18n;
            }
        })
        .catch(() => {});
}
