import { describe, expect, it } from 'vitest';
import { formatSize, isWebUrl, urlHost } from './attachments';

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
