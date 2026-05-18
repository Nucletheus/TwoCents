/**
 * Generate color variations from a parent color
 * Creates lighter/darker variations for category grouping
 */
export function generateColorVariations(
  baseColor: string,
  index: number,
  total: number
): string {
  if (!baseColor) return '#95A5A6'; // Default gray

  // Parse hex color
  const hex = baseColor.replace('#', '');
  const r = parseInt(hex.substring(0, 2), 16);
  const g = parseInt(hex.substring(2, 4), 16);
  const b = parseInt(hex.substring(4, 6), 16);

  // Calculate variation factor (0.1 to 0.3 lightness change)
  const variation = (index / total) * 0.3;
  const lightness = 1 - variation;

  // Apply lightness variation
  const newR = Math.round(r * lightness);
  const newG = Math.round(g * lightness);
  const newB = Math.round(b * lightness);

  return `#${[newR, newG, newB].map((x) => x.toString(16).padStart(2, '0')).join('')}`;
}

/**
 * Get category color (uses parent_color variation if available, falls back to color)
 */
export function getCategoryColor(category: {
  color: string | null;
  parent_color: string | null;
  group_name: string | null;
}, groupIndex: number, groupTotal: number): string {
  if (category.parent_color) {
    return generateColorVariations(category.parent_color, groupIndex, groupTotal);
  }
  return category.color || '#95A5A6';
}

/**
 * Normalize category colors so every category in the same `group_name` shares a base `parent_color`.
 * Optionally provide household-level overrides by group name.
 */
export function normalizeCategoryGroupColors<T extends { group_name: string | null; parent_color: string | null; color: string | null }>(
  categories: T[],
  groupColorOverrides?: Record<string, string>
): T[] {
  const meta: Record<string, { parent?: string; color?: string }> = {};

  categories.forEach((cat) => {
    if (!cat.group_name) return;
    if (!meta[cat.group_name]) meta[cat.group_name] = {};
    if (cat.parent_color && !meta[cat.group_name].parent) meta[cat.group_name].parent = cat.parent_color;
    if (cat.color && !meta[cat.group_name].color) meta[cat.group_name].color = cat.color;
  });

  const baseByGroup: Record<string, string> = {};
  Object.keys(meta).forEach((groupName) => {
    baseByGroup[groupName] =
      groupColorOverrides?.[groupName] ||
      meta[groupName].parent ||
      meta[groupName].color ||
      '#95A5A6';
  });

  return categories.map((cat) => {
    if (!cat.group_name) return cat;
    const base = baseByGroup[cat.group_name];
    if (!base) return cat;
    if (cat.parent_color === base) return cat;
    return { ...cat, parent_color: base };
  });
}

