import { describe, expect, it } from 'vitest';
import { en, zhCN } from './locales';
import { getCatalog, resolveMessage, translate } from './registry';
import type { TranslationKey } from './types';

describe('i18n registry', () => {
  it('returns the English catalog', () => {
    expect(getCatalog('en')).toBe(en);
  });

  it('returns the Simplified Chinese catalog', () => {
    expect(getCatalog('zh-CN')).toBe(zhCN);
  });

  it('keeps both catalogs on the same key set', () => {
    expect(Object.keys(zhCN).sort()).toEqual(Object.keys(en).sort());
  });

  it('translates messages in both locales', () => {
    expect(translate('en', 'language.simplifiedChinese')).toBe(
      'Simplified Chinese',
    );
    expect(translate('zh-CN', 'language.simplifiedChinese')).toBe('简体中文');
  });

  it('interpolates string and number variables', () => {
    expect(
      translate('en', 'language.current', {
        name: 'English',
        unused: 'ignored',
      }),
    ).toBe('Language: English');
    expect(translate('en', 'language.current', { name: 42 })).toBe(
      'Language: 42',
    );
  });

  it('leaves missing interpolation variables visible', () => {
    expect(translate('en', 'language.current')).toBe('Language: {name}');
  });

  it('falls back to English when the active catalog lacks a key', () => {
    expect(resolveMessage({}, 'language.english')).toBe('English');
  });

  it('falls back to the key when no catalog contains it', () => {
    const missingKey = 'missing.key' as TranslationKey;
    expect(resolveMessage({}, missingKey)).toBe('missing.key');
  });
});

describe('bootstrap, login, and register catalogs', () => {
  const keys = [
    'bootstrap.title',
    'bootstrap.step.health',
    'bootstrap.step.loginStatus',
    'bootstrap.log.healthCheck.start',
    'bootstrap.log.healthCheck.ok',
    'bootstrap.log.healthCheck.failed',
    'bootstrap.log.loginStatus.start',
    'bootstrap.log.loginStatus.ok',
    'bootstrap.log.loginStatus.notLoggedIn',
    'bootstrap.log.loginStatus.failed',
    'login.title',
    'login.step.authenticate',
    'login.method.account',
    'login.method.domain',
    'login.field.code',
    'login.field.password',
    'login.action.login',
    'login.action.register',
    'login.hint.notFound',
    'login.hint.inactive',
    'login.log.method.selected',
    'login.log.login.start',
    'login.log.login.ok',
    'login.log.login.failed',
    'login.log.login.notFound',
    'login.log.login.inactive',
    'register.title',
    'register.field.userCode',
    'register.field.domainName',
    'register.field.hostname',
    'register.field.sid',
    'register.field.userName',
    'register.field.password',
    'register.action.register',
    'register.hint.contactAdmin',
    'register.log.identity.start',
    'register.log.identity.ok',
    'register.log.identity.failed',
    'register.log.register.start',
    'register.log.register.ok',
    'register.log.register.failed',
  ] as const;

  it.each(keys)(
    'has a non-empty en and zh-CN message for %s',
    (key) => {
      expect(translate('en', key)).not.toBe(key);
      expect(translate('en', key).length).toBeGreaterThan(0);
      expect(translate('zh-CN', key)).not.toBe(key);
      expect(translate('zh-CN', key).length).toBeGreaterThan(0);
    },
  );

  it('interpolates the message variable in a failure log line', () => {
    expect(
      translate('en', 'login.log.login.failed', { message: 'boom' }),
    ).toContain('boom');
    expect(
      translate('zh-CN', 'login.log.login.failed', { message: 'boom' }),
    ).toContain('boom');
  });
});

describe('project dashboard catalog', () => {
  const keys = [
    'workspace.dashboard.crfStatusTitle',
    'workspace.dashboard.noCrfVersions',
    'workspace.dashboard.noCrfForms',
  ] as const;

  it.each(keys)(
    'has a non-empty en and zh-CN message for %s',
    (key) => {
      expect(translate('en', key)).not.toBe(key);
      expect(translate('en', key).length).toBeGreaterThan(0);
      expect(translate('zh-CN', key)).not.toBe(key);
      expect(translate('zh-CN', key).length).toBeGreaterThan(0);
    },
  );
});

describe('templates catalog', () => {
  const keys = [
    'templates.structure.update',
    'templates.structure.rootpath',
    'templates.path.browse',
  ] as const;

  it.each(keys)(
    'has a non-empty en and zh-CN message for %s',
    (key) => {
      expect(translate('en', key)).not.toBe(key);
      expect(translate('en', key).length).toBeGreaterThan(0);
      expect(translate('zh-CN', key)).not.toBe(key);
      expect(translate('zh-CN', key).length).toBeGreaterThan(0);
    },
  );
});