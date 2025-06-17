import he from 'he';
import { describe, expect, it } from 'vitest';
import { extractStringValues, getHighlightChunks } from '../highlight-utils';

describe('extractStringValues', () => {
  it('returns empty array for undefined or non-array', () => {
    expect(extractStringValues(undefined as any)).toEqual([]);
    expect(extractStringValues(null as any)).toEqual([]);
    expect(extractStringValues({} as any)).toEqual([]);
  });

  it('extracts plain string values', () => {
    const inputs = [{ value: 'hello world' }];
    expect(extractStringValues(inputs as any)).toEqual([
      he.encode('hello world'),
    ]);
  });

  it('extracts values from JSON string', () => {
    const inputs = [{ value: '{"a":"foo","b":"bar"}' }];
    expect(extractStringValues(inputs as any)).toEqual(['foo', 'bar']);
  });

  it('splits repeated text by 2+ spaces', () => {
    const inputs = [{ value: 'foo  bar   baz' }];
    expect(extractStringValues(inputs as any)).toEqual([
      he.encode('foo'),
      he.encode('bar'),
      he.encode('baz'),
    ]);
  });

  it('extracts string values from object input', () => {
    const inputs = [{ value: { a: 'foo', b: 2, c: 'bar' } }];
    expect(extractStringValues(inputs as any)).toEqual(['foo', 'bar']);
  });
});

describe('getHighlightChunks', () => {
  const text = 'The quick brown fox jumps over the lazy dog.';

  it('returns only chunks present in text', () => {
    const chunks = ['quick', 'cat', 'dog'];
    expect(getHighlightChunks(text, chunks)).toEqual(['quick', 'dog']);
  });

  it('handles empty or missing chunks/text', () => {
    expect(getHighlightChunks('', ['foo'])).toEqual([]);
    expect(getHighlightChunks(text, [])).toEqual([]);
    expect(getHighlightChunks(text, [''])).toEqual([]);
  });

  it('escapes regex special characters in chunks', () => {
    const special = 'quick.';
    expect(getHighlightChunks(text, [special])).toEqual([]);
    expect(getHighlightChunks(text + ' quick.', [special])).toEqual(['quick.']);
  });

  it('handles unicode and emoji', () => {
    const t = 'Hello 👋 world';
    expect(getHighlightChunks(t, ['👋'])).toEqual(['👋']);
  });
});
