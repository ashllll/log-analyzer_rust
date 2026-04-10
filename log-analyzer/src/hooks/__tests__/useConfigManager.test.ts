import { computeConfigFingerprint } from '../useConfigManager';
import type { KeywordGroup } from '../../types/common';

describe('computeConfigFingerprint', () => {
  const baseGroup: KeywordGroup = {
    id: 'g1',
    name: 'Errors',
    color: 'red',
    patterns: [{ regex: 'error', comment: '' }],
    enabled: true,
  };

  it('detects change when a keyword group name is edited', () => {
    const before = computeConfigFingerprint([baseGroup], []);
    const after = computeConfigFingerprint(
      [{ ...baseGroup, name: 'Critical Errors' }],
      [],
    );
    expect(after).not.toBe(before);
  });

  it('detects change when a keyword group color is edited', () => {
    const before = computeConfigFingerprint([baseGroup], []);
    const after = computeConfigFingerprint(
      [{ ...baseGroup, color: 'blue' }],
      [],
    );
    expect(after).not.toBe(before);
  });

  it('detects change when a keyword group pattern is added', () => {
    const before = computeConfigFingerprint([baseGroup], []);
    const after = computeConfigFingerprint(
      [{ ...baseGroup, patterns: [...baseGroup.patterns, { regex: 'timeout', comment: '' }] }],
      [],
    );
    expect(after).not.toBe(before);
  });

  it('detects change when a keyword group pattern is removed', () => {
    const group: KeywordGroup = {
      ...baseGroup,
      patterns: [
        { regex: 'error', comment: '' },
        { regex: 'timeout', comment: '' },
      ],
    };
    const before = computeConfigFingerprint([group], []);
    const after = computeConfigFingerprint(
      [{ ...group, patterns: [{ regex: 'error', comment: '' }] }],
      [],
    );
    expect(after).not.toBe(before);
  });

  it('detects change when a keyword group pattern regex is edited', () => {
    const before = computeConfigFingerprint([baseGroup], []);
    const after = computeConfigFingerprint(
      [{ ...baseGroup, patterns: [{ regex: 'fatal', comment: '' }] }],
      [],
    );
    expect(after).not.toBe(before);
  });

  it('detects change when a keyword group enabled state changes', () => {
    const before = computeConfigFingerprint([baseGroup], []);
    const after = computeConfigFingerprint(
      [{ ...baseGroup, enabled: false }],
      [],
    );
    expect(after).not.toBe(before);
  });

  it('detects change when a keyword group is deleted', () => {
    const before = computeConfigFingerprint([baseGroup], []);
    const after = computeConfigFingerprint([], []);
    expect(after).not.toBe(before);
  });

  it('produces the same fingerprint when nothing changed', () => {
    const fp1 = computeConfigFingerprint([baseGroup], []);
    const fp2 = computeConfigFingerprint([baseGroup], []);
    expect(fp2).toBe(fp1);
  });
});
