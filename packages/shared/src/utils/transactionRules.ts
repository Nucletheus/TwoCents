import type { TransactionRule, RuleMatchPattern, Expense } from '../types';
import { normalizeVendor, extractVendor } from './vendorExtraction';

/**
 * Check if a date matches a date pattern
 */
function matchesDatePattern(date: string, pattern: string): boolean {
  const dateObj = new Date(date);
  const dayOfWeek = dateObj.getDay(); // 0 = Sunday, 6 = Saturday
  const dayOfMonth = dateObj.getDate();
  const month = dateObj.getMonth() + 1; // 1-12

  switch (pattern.toLowerCase()) {
    case 'weekend':
      return dayOfWeek === 0 || dayOfWeek === 6;
    case 'weekday':
      return dayOfWeek >= 1 && dayOfWeek <= 5;
    case 'first-of-month':
      return dayOfMonth === 1;
    case 'last-of-month':
      // Approximate - check if it's in the last 3 days
      const lastDay = new Date(dateObj.getFullYear(), month, 0).getDate();
      return dayOfMonth >= lastDay - 2;
    case 'monthly':
      // Match if it's the same day of month as a reference (would need more context)
      return true; // Simplified - would need reference date
    default:
      return false;
  }
}

/**
 * Check if a string matches a pattern (string or RegExp)
 * Null/empty values match each other
 */
function matchesPattern(text: string | null, pattern: string | RegExp | undefined): boolean {
  if (!pattern) return true; // No pattern means match anything
  
  // Handle null/empty matching: null and empty string should match each other
  const textIsEmpty = !text || text.trim() === '';
  const patternIsEmpty = typeof pattern === 'string' && (!pattern || pattern.trim() === '');
  
  if (textIsEmpty && patternIsEmpty) return true;
  if (textIsEmpty || patternIsEmpty) return false;

  if (pattern instanceof RegExp) {
    return pattern.test(text);
  }

  // Case-insensitive string matching
  const normalizedText = text.toLowerCase();
  const normalizedPattern = pattern.toLowerCase();
  
  return normalizedText.includes(normalizedPattern);
}

/**
 * Check if an amount is within a range
 */
function matchesAmountRange(amount: number, range: { min?: number; max?: number } | undefined): boolean {
  if (!range) return true;

  if (range.min !== undefined && amount < range.min) {
    return false;
  }
  if (range.max !== undefined && amount > range.max) {
    return false;
  }

  return true;
}

/**
 * Check if an expense matches a single rule pattern
 */
function matchesRulePattern(expense: Expense, pattern: RuleMatchPattern): boolean {
  const vendor = normalizeVendor(expense.vendor || extractVendor(expense.description));
  const description = expense.description || '';
  const subDescription = expense.sub_description || '';

  // Check individual conditions
  const vendorMatch = pattern.vendor
    ? matchesPattern(vendor, pattern.vendor)
    : true;

  const descriptionMatch = pattern.description
    ? matchesPattern(description, pattern.description)
    : true;

  const subDescriptionMatch = pattern.sub_description
    ? matchesPattern(subDescription, pattern.sub_description)
    : true;

  const amountMatch = pattern.amountRange
    ? matchesAmountRange(expense.amount, pattern.amountRange)
    : true;

  const dateMatch = pattern.datePattern
    ? matchesDatePattern(expense.date, pattern.datePattern)
    : true;

  // Combine based on combination type
  if (pattern.combination === 'OR') {
    return vendorMatch || descriptionMatch || subDescriptionMatch || amountMatch || dateMatch;
  }

  // Default to AND
  return vendorMatch && descriptionMatch && subDescriptionMatch && amountMatch && dateMatch;
}

/**
 * Apply transaction rules to an expense and return matching category
 * Rules are applied in priority order (higher priority first)
 * First matching rule wins
 */
export function applyTransactionRules(
  expense: Expense,
  rules: TransactionRule[]
): string | null {
  // Filter active rules and sort by priority (descending)
  const activeRules = rules
    .filter((rule) => rule.is_active)
    .sort((a, b) => b.priority - a.priority);

  // Try each rule in priority order
  for (const rule of activeRules) {
    // Handle different match types
    let matches = false;

    switch (rule.match_type) {
      case 'vendor': {
        const vendor = normalizeVendor(expense.vendor || extractVendor(expense.description));
        matches = matchesPattern(vendor, rule.match_pattern.vendor);
        break;
      }

      case 'description': {
        matches = matchesPattern(expense.description, rule.match_pattern.description);
        break;
      }

      case 'amount_range': {
        matches = matchesAmountRange(expense.amount, rule.match_pattern.amountRange);
        break;
      }

      case 'date_pattern': {
        matches = matchesDatePattern(expense.date, rule.match_pattern.datePattern || '');
        break;
      }

      case 'combination': {
        matches = matchesRulePattern(expense, rule.match_pattern);
        break;
      }
    }

    if (matches && rule.category_id) {
      return rule.category_id;
    }
  }

  return null;
}

/**
 * Test a rule against an expense (for preview/testing)
 */
export function testRuleAgainstExpense(
  expense: Expense,
  rule: TransactionRule
): boolean {
  if (!rule.is_active) return false;

  switch (rule.match_type) {
    case 'vendor': {
      const vendor = normalizeVendor(expense.vendor || extractVendor(expense.description));
      return matchesPattern(vendor, rule.match_pattern.vendor);
    }
    case 'description': {
      return matchesPattern(expense.description, rule.match_pattern.description);
    }
    case 'amount_range': {
      return matchesAmountRange(expense.amount, rule.match_pattern.amountRange);
    }
    case 'date_pattern': {
      return matchesDatePattern(expense.date, rule.match_pattern.datePattern || '');
    }
    case 'combination': {
      return matchesRulePattern(expense, rule.match_pattern);
    }
    default:
      return false;
  }
}

