import { detectColumnTypes, normalizeDate, normalizeAmount } from './csvParser';
import type { ColumnMapping } from '../types';

/**
 * Auto-detect column mapping from CSV headers and sample data
 */
export function autoDetectColumnMapping(
  headers: string[],
  sampleRows: string[][]
): ColumnMapping {
  const mapping: ColumnMapping = {};
  const detections = detectColumnTypes(headers, sampleRows);

  // Find best matches for each field type
  const dateColumns = headers.filter((h) => detections[h] === 'date');
  const amountColumns = headers.filter((h) => detections[h] === 'amount');
  const descriptionColumns = headers.filter((h) => detections[h] === 'description');
  const vendorColumns = headers.filter((h) => detections[h] === 'vendor');
  const categoryColumns = headers.filter((h) => detections[h] === 'category');

  // Assign first match for each type
  if (dateColumns.length > 0) {
    mapping.date = dateColumns[0];
  }
  if (amountColumns.length > 0) {
    mapping.amount = amountColumns[0];
  }
  if (descriptionColumns.length > 0) {
    mapping.description = descriptionColumns[0];
  }
  if (vendorColumns.length > 0) {
    mapping.vendor = vendorColumns[0];
  } else if (descriptionColumns.length > 0) {
    // Use description as fallback for vendor
    mapping.vendor = descriptionColumns[0];
  }
  if (categoryColumns.length > 0) {
    mapping.category = categoryColumns[0];
  }

  return mapping;
}

/**
 * Match filename pattern (supports wildcards)
 */
export function matchFilenamePattern(filename: string, pattern: string): boolean {
  if (!pattern) return false;

  // Convert pattern to regex
  const regexPattern = pattern
    .replace(/\./g, '\\.')
    .replace(/\*/g, '.*')
    .replace(/\?/g, '.');

  const regex = new RegExp(`^${regexPattern}$`, 'i');
  return regex.test(filename);
}

