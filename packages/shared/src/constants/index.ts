export const DEFAULT_CATEGORIES = [
  { name: 'Food & Dining', icon: '🍽️', color: '#FF6B6B' },
  { name: 'Shopping', icon: '🛍️', color: '#4ECDC4' },
  { name: 'Transportation', icon: '🚗', color: '#45B7D1' },
  { name: 'Bills & Utilities', icon: '💡', color: '#FFA07A' },
  { name: 'Entertainment', icon: '🎬', color: '#98D8C8' },
  { name: 'Healthcare', icon: '🏥', color: '#F7DC6F' },
  { name: 'Education', icon: '📚', color: '#BB8FCE' },
  { name: 'Travel', icon: '✈️', color: '#85C1E2' },
  { name: 'Personal Care', icon: '💅', color: '#F1948A' },
  { name: 'Other', icon: '📦', color: '#95A5A6' },
] as const;

export const CURRENCY_SYMBOLS: Record<string, string> = {
  USD: '$',
  EUR: '€',
  GBP: '£',
  CAD: 'C$',
  AUD: 'A$',
};

