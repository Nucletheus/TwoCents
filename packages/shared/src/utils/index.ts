import { format, parseISO, startOfMonth, endOfMonth, startOfYear, endOfYear } from 'date-fns';

export const formatCurrency = (amount: number, currency: string = 'USD'): string => {
  return new Intl.NumberFormat('en-US', {
    style: 'currency',
    currency,
  }).format(amount);
};

export const formatDate = (date: string | Date, formatStr: string = 'MMM dd, yyyy'): string => {
  const dateObj = typeof date === 'string' ? parseISO(date) : date;
  return format(dateObj, formatStr);
};

export const getMonthRange = (date: Date = new Date()) => {
  return {
    start: startOfMonth(date),
    end: endOfMonth(date),
  };
};

export const getYearRange = (date: Date = new Date()) => {
  return {
    start: startOfYear(date),
    end: endOfYear(date),
  };
};

export const calculateSplitAmount = (
  totalAmount: number,
  splitType: 'equal' | 'percentage' | 'custom',
  participants: number,
  percentages?: number[],
  customAmounts?: number[]
): number[] => {
  if (splitType === 'equal') {
    const amountPerPerson = totalAmount / participants;
    return Array(participants).fill(amountPerPerson);
  }

  if (splitType === 'percentage' && percentages) {
    return percentages.map((pct) => (totalAmount * pct) / 100);
  }

  if (splitType === 'custom' && customAmounts) {
    return customAmounts;
  }

  return [];
};

// Export transaction rules utilities
export { applyTransactionRules, testRuleAgainstExpense } from './transactionRules';
export { extractVendor, normalizeVendor } from './vendorExtraction';

// Export CSV utilities
export {
  parseCSV,
  detectColumnTypes,
  normalizeDate,
  normalizeAmount,
} from './csvParser';
export { autoDetectColumnMapping, matchFilenamePattern } from './columnDetector';

// Export color utilities
export { generateColorVariations, getCategoryColor, normalizeCategoryGroupColors } from './colorUtils';

// Settlements utilities
export {
  calculateSharedSettlements,
  buildEvenSplit,
  type SharedSplitMap,
} from './settlements';

// Savings processing utilities
export { processSavingsFromImport, processExpenseForSavings, isSavingsCategory } from './savingsProcessor';

// Duplicate detection utilities
export { checkForDuplicates, getVendorCategories } from './duplicateDetection';

