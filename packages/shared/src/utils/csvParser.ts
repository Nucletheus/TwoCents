/**
 * Parse CSV file content into rows and columns
 * Handles quoted fields, escaped quotes, and various line endings
 */
export function parseCSV(content: string): string[][] {
  if (!content || typeof content !== 'string') {
    throw new Error('Invalid CSV content: content must be a non-empty string');
  }

  const rows: string[][] = [];
  let currentRow: string[] = [];
  let currentCell = '';
  let inQuotes = false;

  // Normalize line endings to \n
  const normalizedContent = content.replace(/\r\n/g, '\n').replace(/\r/g, '\n');

  for (let i = 0; i < normalizedContent.length; i++) {
    const char = normalizedContent[i];
    const nextChar = normalizedContent[i + 1];

    if (char === '"') {
      if (inQuotes && nextChar === '"') {
        // Escaped quote (double quote)
        currentCell += '"';
        i++; // Skip next quote
      } else if (inQuotes && (nextChar === ',' || nextChar === '\n' || nextChar === undefined)) {
        // End of quoted field
        inQuotes = false;
      } else {
        // Start of quoted field
        inQuotes = !inQuotes;
      }
    } else if (char === ',' && !inQuotes) {
      // End of cell
      currentRow.push(currentCell);
      currentCell = '';
    } else if (char === '\n' && !inQuotes) {
      // End of row
      currentRow.push(currentCell);
      if (currentRow.length > 0 && currentRow.some(cell => cell.trim().length > 0)) {
        rows.push(currentRow);
      }
      currentRow = [];
      currentCell = '';
    } else {
      currentCell += char;
    }
  }

  // Add last cell and row if there's remaining content
  if (currentCell.trim() || currentRow.length > 0) {
    currentRow.push(currentCell);
    if (currentRow.length > 0 && currentRow.some(cell => cell.trim().length > 0)) {
      rows.push(currentRow);
    }
  }

  // Ensure all rows have the same number of columns (pad with empty strings)
  if (rows.length > 0) {
    const maxColumns = Math.max(...rows.map(row => row.length));
    rows.forEach(row => {
      while (row.length < maxColumns) {
        row.push('');
      }
    });
  }

  return rows;
}

/**
 * Detect column types from CSV headers and sample data
 */
export function detectColumnTypes(
  headers: string[],
  sampleRows: string[][]
): Record<string, 'date' | 'amount' | 'description' | 'vendor' | 'category' | 'unknown'> {
  const detections: Record<string, 'date' | 'amount' | 'description' | 'vendor' | 'category' | 'unknown'> = {};

  headers.forEach((header, index) => {
    const headerLower = header.toLowerCase().trim();
    const sampleValues = sampleRows.slice(0, 10).map((row) => row[index]?.toLowerCase().trim() || '');

    // Check header keywords
    if (
      headerLower.includes('date') ||
      headerLower.includes('transaction date') ||
      headerLower.includes('posted date')
    ) {
      detections[header] = 'date';
      return;
    }

    if (
      headerLower.includes('amount') ||
      headerLower.includes('total') ||
      headerLower.includes('price') ||
      headerLower.includes('cost')
    ) {
      detections[header] = 'amount';
      return;
    }

    if (
      headerLower.includes('description') ||
      headerLower.includes('memo') ||
      headerLower.includes('note') ||
      headerLower.includes('details') ||
      headerLower.includes('transaction') ||
      headerLower.includes('item') ||
      headerLower.includes('payee')
    ) {
      detections[header] = 'description';
      return;
    }

    if (headerLower.includes('vendor') || headerLower.includes('merchant') || headerLower.includes('store')) {
      detections[header] = 'vendor';
      return;
    }

    if (headerLower.includes('category')) {
      detections[header] = 'category';
      return;
    }

    // Try to detect from sample values
    if (sampleValues.length > 0) {
      // Check if looks like date
      const datePattern = /^\d{1,2}[-\/]\d{1,2}[-\/]\d{2,4}$/;
      if (sampleValues.some((v) => datePattern.test(v) || !isNaN(Date.parse(v)))) {
        detections[header] = 'date';
        return;
      }

      // Check if looks like amount
      const amountPattern = /^-?\$?\d+\.?\d*$/;
      if (sampleValues.some((v) => amountPattern.test(v.replace(/[,\s]/g, '')))) {
        detections[header] = 'amount';
        return;
      }
    }

    detections[header] = 'unknown';
  });

  return detections;
}

/**
 * Normalize date string to YYYY-MM-DD format
 */
export function normalizeDate(dateStr: string): string | null {
  if (!dateStr) return null;

  // Try common formats
  const formats = [
    /(\d{4})-(\d{2})-(\d{2})/, // YYYY-MM-DD
    /(\d{2})\/(\d{2})\/(\d{4})/, // MM/DD/YYYY
    /(\d{2})-(\d{2})-(\d{4})/, // MM-DD-YYYY
    /(\d{4})\/(\d{2})\/(\d{2})/, // YYYY/MM/DD
  ];

  for (const format of formats) {
    const match = dateStr.match(format);
    if (match) {
      if (format === formats[0] || format === formats[3]) {
        // Already YYYY-MM-DD or YYYY/MM/DD
        return dateStr.replace(/\//g, '-');
      } else if (format === formats[1]) {
        // MM/DD/YYYY
        return `${match[3]}-${match[1]}-${match[2]}`;
      } else if (format === formats[2]) {
        // MM-DD-YYYY
        return `${match[3]}-${match[1]}-${match[2]}`;
      }
    }
  }

  // Try parsing with Date
  const parsed = new Date(dateStr);
  if (!isNaN(parsed.getTime())) {
    const year = parsed.getFullYear();
    const month = String(parsed.getMonth() + 1).padStart(2, '0');
    const day = String(parsed.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  return null;
}

/**
 * Normalize amount string to number
 */
export function normalizeAmount(amountStr: string): number | null {
  if (!amountStr) return null;

  // Remove currency symbols, commas, spaces
  const cleaned = amountStr.replace(/[$,\s]/g, '');

  // Handle negative amounts (parentheses or minus sign)
  const isNegative = cleaned.startsWith('-') || cleaned.startsWith('(');
  // Remove parentheses and leading minus sign before parsing
  // If it starts with '-', remove the '-'. If it starts with '(', remove both '(' and ')'
  let numericStr = cleaned;
  if (cleaned.startsWith('-')) {
    numericStr = cleaned.substring(1); // Remove leading minus
  } else if (cleaned.startsWith('(')) {
    numericStr = cleaned.replace(/[()]/g, ''); // Remove parentheses
  }

  const amount = parseFloat(numericStr);
  if (isNaN(amount)) return null;

  return isNegative ? -amount : amount;
}

