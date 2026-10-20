// Tests run in English whatever the Mac's language is (see lib/i18n.ts).
Object.defineProperty(globalThis.navigator, 'languages', { value: ['en-US'], configurable: true });
Object.defineProperty(globalThis.navigator, 'language', { value: 'en-US', configurable: true });
