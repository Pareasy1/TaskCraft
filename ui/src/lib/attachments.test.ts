import { describe, expect, it } from 'vitest';
import { formatSize, isWebUrl, notAddedMessage, refuseFileDrop, urlHost } from './attachments';

describe('formatSize', () => {
  it('uses bytes below 1 KB', () => {
    expect(formatSize(0, 'en')).toBe('0 B');
    expect(formatSize(999, 'en')).toBe('999 B');
  });
  it('scales with 1024 steps', () => {
    expect(formatSize(1024, 'en')).toBe('1 KB');
    expect(formatSize(104_857_600, 'en')).toBe('100 MB');
  });
  it('uses one decimal and the language separator', () => {
    expect(formatSize(1_250_000, 'en')).toBe('1.2 MB');
    expect(formatSize(1_250_000, 'it')).toBe('1,2 MB');
  });
});

describe('urlHost', () => {
  it('drops www and the path', () => {
    expect(urlHost('https://www.x.io/a?b')).toBe('x.io');
    expect(urlHost('http://docs.x.io:8080/')).toBe('docs.x.io');
  });
  it('returns invalid input as is', () => {
    expect(urlHost('not a url')).toBe('not a url');
  });
});

describe('isWebUrl', () => {
  it('accepts http and https only', () => {
    expect(isWebUrl('https://a.io')).toBe(true);
    expect(isWebUrl(' http://a.io/x ')).toBe(true);
    expect(isWebUrl('ftp://a.io')).toBe(false);
    expect(isWebUrl('javascript:alert(1)')).toBe(false);
    expect(isWebUrl('a.io')).toBe(false);
    expect(isWebUrl('')).toBe(false);
  });
});

describe('notAddedMessage', () => {
  const reason = (code: string) => `<${code}>`;
  const fmt = (names: string, why: string) => `${names}: ${why}`;
  it('is null when every file was added', () => {
    expect(notAddedMessage([], reason, fmt)).toBeNull();
  });
  it('lists the names that share a reason once', () => {
    expect(notAddedMessage([{ name: 'a.mov', code: 'attachment_too_large' }, { name: 'b.iso', code: 'attachment_too_large' }], reason, fmt))
      .toBe('a.mov, b.iso: <attachment_too_large>');
  });
  it('keeps one sentence per reason, in the order they first appear', () => {
    expect(notAddedMessage([
      { name: 'a.mov', code: 'attachment_too_large' }, { name: 'gone.txt', code: 'not_found' }, { name: 'b.iso', code: 'attachment_too_large' },
    ], reason, fmt)).toBe('a.mov, b.iso: <attachment_too_large> gone.txt: <not_found>');
  });
  it('uses the generic reason for codes without a message of their own (io, db…)', () => {
    const known = (code: string) => (code === 'attachment_too_large' ? '<too large>' : null);
    expect(notAddedMessage([
      { name: 'x.bin', code: 'io' }, { name: 'y.bin', code: 'db' }, { name: 'a.mov', code: 'attachment_too_large' },
    ], known, fmt, '<not copied>')).toBe('x.bin, y.bin: <not copied> a.mov: <too large>');
  });
});

describe('refuseFileDrop', () => {
  const ev = (types: string[]) => {
    const dataTransfer = { types, dropEffect: 'copy' as DataTransfer['dropEffect'] };
    let prevented = false;
    return {
      e: { dataTransfer, preventDefault: () => { prevented = true; } } as unknown as DragEvent,
      dataTransfer,
      prevented: () => prevented,
    };
  };
  it('blocks a drag carrying OS files so the webview never opens the file', () => {
    const x = ev(['Files']);
    expect(refuseFileDrop(x.e)).toBe(true);
    expect(x.prevented()).toBe(true);
    expect(x.dataTransfer.dropEffect).toBe('none');
  });
  it('leaves an in-app row drag alone', () => {
    const x = ev(['text/plain']);
    expect(refuseFileDrop(x.e)).toBe(false);
    expect(x.prevented()).toBe(false);
    expect(x.dataTransfer.dropEffect).toBe('copy');
  });
  it('copes with no dataTransfer', () => {
    expect(refuseFileDrop({ preventDefault() {} } as unknown as DragEvent)).toBe(false);
  });
});
