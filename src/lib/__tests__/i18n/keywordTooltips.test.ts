import { describe, it, expect } from 'vitest';
import * as de from '../../i18n/de.json';
import * as en from '../../i18n/en.json';

const localeEntries = [
  { name: 'de', data: de },
  { name: 'en', data: en },
];

const requiredKeys = [
  'extractionMethodsLabel',
  'extractionMethodsNote',
  'typePerson',
  'typeOrganization',
  'typeLocation',
  'typeAcronym',
  'typeConcept',
  'openNetwork',
  'sourceAi',
  'sourceStatistical',
  'sourceManual',
  'multiConfirmed',
  'confidenceLabel',
  'confidenceNote',
  'qualityLabel',
  'qualityNote',
  'showNeighbors',
  'remove',
  'similarityLabel',
  'similarityNote',
  'cooccurrenceLabel',
  'cooccurrenceNote',
  'tfidfLabel',
  'tfidfNote',
  'semanticLabel',
  'semanticNote',
];

describe('keywordTooltips translations', () => {
  localeEntries.forEach(({ name, data }) => {
    it(`${name} includes keywordTooltips keys`, () => {
      const tooltips = data.keywordTooltips as Record<string, string> | undefined;
      requiredKeys.forEach((key) => {
        const value = tooltips?.[key];
        expect(typeof value).toBe('string');
        expect(value?.length).toBeGreaterThan(0);
      });
    });
  });
});
