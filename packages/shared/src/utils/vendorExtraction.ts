/**
 * Extract vendor name from expense description
 * Handles common patterns like:
 * - "STARBUCKS #1234" -> "STARBUCKS"
 * - "AMAZON.COM*ABC123" -> "AMAZON.COM"
 * - "Uber Eats - Restaurant" -> "Uber Eats"
 * - "Gas Station #1 | Location" -> "Gas Station"
 */
export function extractVendor(description: string | null): string | null {
  if (!description) return null;

  let vendor = description.trim();

  // Remove common suffixes/patterns
  // Remove transaction IDs, reference numbers (e.g., #1234, *ABC123)
  vendor = vendor.replace(/\s*[#*]\s*[A-Z0-9]+.*$/i, '');

  // Remove common separators and everything after
  const separators = ['|', '-', '•', '·', '—', '–'];
  for (const sep of separators) {
    const index = vendor.indexOf(sep);
    if (index > 0) {
      vendor = vendor.substring(0, index).trim();
      break;
    }
  }

  // Remove common prefixes
  vendor = vendor.replace(/^(PAYMENT|PAY|PURCHASE|TRANSACTION)\s+/i, '');

  // Remove trailing location info (e.g., "STORE #123 CITY")
  vendor = vendor.replace(/\s+#\d+.*$/i, '');

  // Clean up whitespace
  vendor = vendor.trim();

  // If result is too short or empty, return original
  if (vendor.length < 2) {
    return description.trim();
  }

  return vendor;
}

/**
 * Normalize vendor name for consistent matching
 */
export function normalizeVendor(vendor: string | null): string {
  if (!vendor) return '';
  
  return vendor
    .toUpperCase()
    .trim()
    .replace(/\s+/g, ' ') // Normalize whitespace
    .replace(/[^\w\s]/g, ''); // Remove special characters
}

