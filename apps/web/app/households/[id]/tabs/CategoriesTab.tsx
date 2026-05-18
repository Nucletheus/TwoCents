'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { SupabaseClient } from '@supabase/supabase-js';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../../../components/ui/Card';
import Input from '../../../components/ui/Input';
import Button from '../../../components/ui/Button';
import { Plus, Trash2, Tag, ChevronDown, ChevronRight, PiggyBank, DollarSign, X } from 'lucide-react';

interface Category {
  id: string;
  name: string;
  icon: string | null;
  color: string | null;
  group_name: string | null;
  parent_color: string | null;
  household_id: string | null;
  is_default: boolean; // template-only (households use their own copied categories)
  is_savings_category?: boolean;
  is_hidden?: boolean;
  exclude_from_calculations?: boolean;
  is_income_category?: boolean;
}

interface CategoriesTabProps {
  householdId: string;
  supabase: SupabaseClient;
}

const DEFAULT_COLORS = [
  '#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4', '#FFEAA7',
  '#DDA0DD', '#98D8C8', '#F7DC6F', '#BB8FCE', '#85C1E2',
];

// Convert hex to HSL
function hexToHsl(hex: string): { h: number; s: number; l: number } {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!result) return { h: 0, s: 0, l: 50 };

  let r = parseInt(result[1], 16) / 255;
  let g = parseInt(result[2], 16) / 255;
  let b = parseInt(result[3], 16) / 255;

  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r: h = ((g - b) / d + (g < b ? 6 : 0)) / 6; break;
      case g: h = ((b - r) / d + 2) / 6; break;
      case b: h = ((r - g) / d + 4) / 6; break;
    }
  }

  return { h: h * 360, s: s * 100, l: l * 100 };
}

// Convert HSL to hex
function hslToHex(h: number, s: number, l: number): string {
  s /= 100;
  l /= 100;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    const color = l - a * Math.max(Math.min(k - 3, 9 - k, 1), -1);
    return Math.round(255 * color).toString(16).padStart(2, '0');
  };
  return `#${f(0)}${f(8)}${f(4)}`;
}

// Generate gradient shades from a parent color
function generateGradientShades(parentColor: string, count: number): string[] {
  const hsl = hexToHsl(parentColor);
  const shades: string[] = [];
  
  const minL = Math.max(hsl.l - 10, 25);
  const maxL = Math.min(hsl.l + 30, 85);
  const range = maxL - minL;
  
  for (let i = 0; i < count; i++) {
    const ratio = count > 1 ? i / (count - 1) : 0.5;
    const lightness = minL + (range * ratio);
    const saturation = Math.max(hsl.s - (ratio * 15), 20);
    shades.push(hslToHex(hsl.h, saturation, lightness));
  }
  
  return shades;
}

// Individual category row with local state for editing
function CategoryRow({
  category,
  supabase,
  householdId,
  onUpdated,
  onDeleted,
}: {
  category: Category;
  supabase: SupabaseClient;
  householdId: string;
  onUpdated: () => void;
  onDeleted: (id: string) => void;
}) {
  const [localName, setLocalName] = useState(category.name);
  const [localColor, setLocalColor] = useState(category.color || '#95A5A6');
  const [localIsSavings, setLocalIsSavings] = useState(category.is_savings_category || false);
  const [localExcludeFromCalc, setLocalExcludeFromCalc] = useState(category.exclude_from_calculations || false);
  const [localIsIncome, setLocalIsIncome] = useState(category.is_income_category || false);
  const [saving, setSaving] = useState(false);
  const initialColorRef = useRef(category.color || '#95A5A6');

  useEffect(() => {
    setLocalName(category.name);
    setLocalColor(category.color || '#95A5A6');
    setLocalIsSavings(category.is_savings_category || false);
    setLocalExcludeFromCalc(category.exclude_from_calculations || false);
    setLocalIsIncome(category.is_income_category || false);
    initialColorRef.current = category.color || '#95A5A6';
  }, [category.name, category.color, category.is_savings_category, category.exclude_from_calculations, category.is_income_category]);

  const saveChanges = useCallback(async (updates: { name?: string; color?: string; is_savings_category?: boolean; exclude_from_calculations?: boolean; is_income_category?: boolean }) => {
    if (Object.keys(updates).length === 0) return;

    setSaving(true);
    try {
      const { error } = await supabase
        .from('categories')
        .update(updates)
        .eq('id', category.id);
      if (error) throw error;
      onUpdated();
    } catch (error) {
      console.error('Error updating category:', error);
      alert('Failed to update category');
    } finally {
      setSaving(false);
    }
  }, [supabase, category, householdId, onUpdated]);

  const handleNameBlur = () => {
    if (localName !== category.name && localName.trim()) {
      saveChanges({ name: localName.trim() });
    }
  };

  const handleColorBlur = () => {
    if (localColor !== initialColorRef.current) {
      saveChanges({ color: localColor });
      initialColorRef.current = localColor;
    }
  };

  const handleSavingsToggle = () => {
    const newValue = !localIsSavings;
    setLocalIsSavings(newValue);
    saveChanges({ is_savings_category: newValue });
  };

  const handleExcludeToggle = () => {
    const newValue = !localExcludeFromCalc;
    setLocalExcludeFromCalc(newValue);
    saveChanges({ exclude_from_calculations: newValue });
  };

  const handleIncomeToggle = () => {
    const newValue = !localIsIncome;
    setLocalIsIncome(newValue);
    saveChanges({ is_income_category: newValue });
  };

  return (
    <div className="flex items-center gap-1.5 group py-0.5">
      <input
        type="color"
        value={localColor}
        onChange={(e) => setLocalColor(e.target.value)}
        onBlur={handleColorBlur}
        disabled={saving}
        className="h-5 w-5 cursor-pointer rounded border-0 bg-transparent p-0 disabled:opacity-50 shrink-0"
      />
      <Input
        value={localName}
        onChange={(e) => setLocalName(e.target.value)}
        onBlur={handleNameBlur}
        disabled={saving}
        className="h-6 text-xs flex-1 border-transparent bg-transparent hover:border-border focus:border-border px-1"
      />
      <button
        onClick={handleSavingsToggle}
        disabled={saving}
        title={localIsSavings ? 'Marked as savings category' : 'Mark as savings category'}
        className={`h-5 w-5 p-0.5 rounded transition-colors shrink-0 ${
          localIsSavings 
            ? 'text-green-600 bg-green-100 dark:bg-green-900/30' 
            : 'text-muted-foreground opacity-0 group-hover:opacity-100 hover:text-green-600'
        }`}
      >
        <PiggyBank className="h-full w-full" />
      </button>
      <button
        onClick={handleIncomeToggle}
        disabled={saving}
        title={localIsIncome ? 'Marked as income category' : 'Mark as income category'}
        className={`h-5 w-5 p-0.5 rounded transition-colors shrink-0 ${
          localIsIncome 
            ? 'text-blue-600 bg-blue-100 dark:bg-blue-900/30' 
            : 'text-muted-foreground opacity-0 group-hover:opacity-100 hover:text-blue-600'
        }`}
      >
        <DollarSign className="h-full w-full" />
      </button>
      <button
        onClick={handleExcludeToggle}
        disabled={saving}
        title={localExcludeFromCalc ? 'Excluded from calculations' : 'Exclude from calculations'}
        className={`h-5 w-5 p-0.5 rounded transition-colors shrink-0 ${
          localExcludeFromCalc 
            ? 'text-orange-600 bg-orange-100 dark:bg-orange-900/30' 
            : 'text-muted-foreground opacity-0 group-hover:opacity-100 hover:text-orange-600'
        }`}
      >
        <X className="h-full w-full" />
      </button>
      <Button
        variant="ghost"
        size="sm"
        onClick={() => onDeleted(category.id)}
        disabled={saving}
        className="opacity-0 group-hover:opacity-100 h-5 w-5 p-0 text-muted-foreground hover:text-red-600 shrink-0"
      >
        <Trash2 className="h-3 w-3" />
      </Button>
    </div>
  );
}

// Group header component with color picker and rename/delete
function GroupHeader({
  groupName,
  groupColor,
  categoryCount,
  expanded,
  onToggle,
  onRename,
  onColorChange,
  onDelete,
  saving,
}: {
  groupName: string;
  groupColor: string;
  categoryCount: number;
  expanded: boolean;
  onToggle: () => void;
  onRename: (newName: string) => void;
  onColorChange: (newColor: string) => void;
  onDelete: () => void;
  saving: boolean;
}) {
  const [localName, setLocalName] = useState(groupName);
  const [localColor, setLocalColor] = useState(groupColor);
  const [editing, setEditing] = useState(false);
  const initialColorRef = useRef(groupColor);

  useEffect(() => {
    setLocalName(groupName);
    setLocalColor(groupColor);
    initialColorRef.current = groupColor;
  }, [groupName, groupColor]);

  const handleSave = () => {
    if (localName.trim() === groupName || !localName.trim()) {
      setEditing(false);
      setLocalName(groupName);
      return;
    }
    onRename(localName.trim());
    setEditing(false);
  };

  const handleColorBlur = () => {
    if (localColor !== initialColorRef.current) {
      onColorChange(localColor);
      initialColorRef.current = localColor;
    }
  };

  return (
    <div className="flex items-center gap-1.5 mb-1 group/header">
      <button
        onClick={onToggle}
        className="p-0.5 text-muted-foreground hover:text-foreground shrink-0"
      >
        {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
      </button>
      <input
        type="color"
        value={localColor}
        onChange={(e) => setLocalColor(e.target.value)}
        onBlur={handleColorBlur}
        disabled={saving}
        className="h-4 w-4 cursor-pointer rounded border-0 bg-transparent p-0 disabled:opacity-50 shrink-0"
        title="Change group color (applies gradient to all categories)"
      />
      {editing ? (
        <Input
          value={localName}
          onChange={(e) => setLocalName(e.target.value)}
          onBlur={handleSave}
          onKeyDown={(e) => e.key === 'Enter' && handleSave()}
          disabled={saving}
          autoFocus
          className="h-6 text-xs font-medium flex-1"
        />
      ) : (
        <button
          onClick={() => setEditing(true)}
          className="text-xs font-medium hover:underline text-left flex-1 truncate"
          disabled={saving}
        >
          {groupName}
        </button>
      )}
      <span className="text-[10px] text-muted-foreground shrink-0">({categoryCount})</span>
      <Button
        variant="ghost"
        size="sm"
        onClick={onDelete}
        disabled={saving}
        className="opacity-0 group-hover/header:opacity-100 h-5 w-5 p-0 text-muted-foreground hover:text-red-600 shrink-0"
        title="Delete entire group"
      >
        <Trash2 className="h-3 w-3" />
      </Button>
    </div>
  );
}

export default function CategoriesTab({ householdId, supabase }: CategoriesTabProps) {
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedGroups, setExpandedGroups] = useState<Set<string>>(new Set());
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);
  const [confirmDeleteGroup, setConfirmDeleteGroup] = useState<string | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [savingGroup, setSavingGroup] = useState<string | null>(null);
  const hasLoadedRef = useRef(false);

  const [showNewGroup, setShowNewGroup] = useState(false);
  const [newGroupName, setNewGroupName] = useState('');
  const [newGroupColor, setNewGroupColor] = useState(DEFAULT_COLORS[0]);
  const [creatingGroup, setCreatingGroup] = useState(false);
  
  const [showNewSubCategory, setShowNewSubCategory] = useState<string | null>(null);
  const [newSubCategoryName, setNewSubCategoryName] = useState('');
  const [creatingSubCategory, setCreatingSubCategory] = useState(false);

  const fetchCategories = useCallback(async () => {
    // Avoid "full page reload" feeling on every tiny toggle/save.
    // Only show the big loading state on the initial fetch.
    if (!hasLoadedRef.current) setLoading(true);
    setError(null);
    try {
      const { data: householdCats, error: householdError } = await supabase
        .from('categories')
        .select('*')
        .eq('household_id', householdId)
        .neq('is_hidden', true)
        .order('group_name')
        .order('name');

      if (householdError) throw householdError;

      const merged = (householdCats || []) as Category[];
      setCategories(merged);

      const groups = new Set(merged.map((c) => c.group_name || 'Uncategorized'));
      setExpandedGroups(groups);
    } catch (err: any) {
      console.error('Error fetching categories:', err);
      setError('Unable to load categories.');
    } finally {
      hasLoadedRef.current = true;
      setLoading(false);
    }
  }, [supabase, householdId]);

  useEffect(() => {
    fetchCategories();
  }, [fetchCategories]);

  const toggleGroup = (groupName: string) => {
    setExpandedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(groupName)) {
        next.delete(groupName);
      } else {
        next.add(groupName);
      }
      return next;
    });
  };

  const handleGroupColorChange = async (groupName: string, newColor: string, cats: Category[]) => {
    setSavingGroup(groupName);
    setError(null);
    
    try {
      const shades = generateGradientShades(newColor, cats.length);
      
      for (let i = 0; i < cats.length; i++) {
        const cat = cats[i];
        const shade = shades[i];
        
        await supabase
          .from('categories')
          .update({ color: shade, parent_color: newColor })
          .eq('id', cat.id);
      }
      
      await fetchCategories();
    } catch (err: any) {
      console.error('Error updating group color:', err);
      setError('Unable to update group color.');
    } finally {
      setSavingGroup(null);
    }
  };

  const handleGroupRename = async (oldName: string, newName: string, cats: Category[]) => {
    if (oldName === newName) return;
    
    setSavingGroup(oldName);
    setError(null);
    
    try {
      for (const cat of cats) {
        await supabase
          .from('categories')
          .update({ group_name: newName })
          .eq('id', cat.id);
      }
      
      setExpandedGroups((prev) => {
        const next = new Set(prev);
        if (next.has(oldName)) {
          next.delete(oldName);
          next.add(newName);
        }
        return next;
      });
      
      await fetchCategories();
    } catch (err: any) {
      console.error('Error renaming group:', err);
      setError('Unable to rename group.');
    } finally {
      setSavingGroup(null);
    }
  };

  const deleteGroup = async () => {
    if (!confirmDeleteGroup) return;
    
    setDeleting(true);
    try {
      const groupCats = groupedCategories[confirmDeleteGroup] || [];
      
      for (const cat of groupCats) {
        await supabase.from('categories').delete().eq('id', cat.id);
      }
      
      setExpandedGroups((prev) => {
        const next = new Set(prev);
        next.delete(confirmDeleteGroup);
        return next;
      });
      
      await fetchCategories();
      setConfirmDeleteGroup(null);
    } catch (err: any) {
      console.error('Error deleting group:', err);
      setError('Unable to delete group.');
    } finally {
      setDeleting(false);
    }
  };

  const createGroup = async () => {
    if (!newGroupName.trim()) return;

    setCreatingGroup(true);
    try {
      // Create a new group by creating a category with the group name
      // The first category in the group will have the group name as its name
      const { error } = await supabase.from('categories').insert({
        name: newGroupName.trim(),
        color: newGroupColor,
        group_name: newGroupName.trim(),
        parent_color: newGroupColor,
        household_id: householdId,
        is_default: false,
      });

      if (error) throw error;
      
      // Expand the new group
      setExpandedGroups((prev) => {
        const next = new Set(prev);
        next.add(newGroupName.trim());
        return next;
      });
      
      await fetchCategories();
      setShowNewGroup(false);
      setNewGroupName('');
      setNewGroupColor(DEFAULT_COLORS[Math.floor(Math.random() * DEFAULT_COLORS.length)]);
    } catch (err: any) {
      console.error('Error creating group:', err);
      setError('Unable to create group.');
    } finally {
      setCreatingGroup(false);
    }
  };

  const createSubCategory = async (groupName: string) => {
    if (!newSubCategoryName.trim()) return;

    setCreatingSubCategory(true);
    try {
      const groupCats = groupedCategories[groupName] || [];
      const groupColor = getGroupColor(groupCats);
      
      // Generate a color variation for the new sub-category
      const shades = generateGradientShades(groupColor, groupCats.length + 1);
      const newColor = shades[groupCats.length];

      const { error } = await supabase.from('categories').insert({
        name: newSubCategoryName.trim(),
        color: newColor,
        group_name: groupName,
        parent_color: groupColor,
        household_id: householdId,
        is_default: false,
      });

      if (error) throw error;
      await fetchCategories();
      setShowNewSubCategory(null);
      setNewSubCategoryName('');
    } catch (err: any) {
      console.error('Error creating sub-category:', err);
      setError('Unable to create sub-category.');
    } finally {
      setCreatingSubCategory(false);
    }
  };

  const deleteCategory = async () => {
    if (!confirmDeleteId) return;

    setDeleting(true);
    try {
      const category = categories.find((c) => c.id === confirmDeleteId);
      if (!category) return;

      const { error } = await supabase.from('categories').delete().eq('id', confirmDeleteId);
      if (error) throw error;
      await fetchCategories();
      setConfirmDeleteId(null);
    } catch (err: any) {
      console.error('Error deleting category:', err);
      setError('Unable to delete category. It may be in use by expenses.');
    } finally {
      setDeleting(false);
    }
  };

  const groupedCategories = categories.reduce((acc, cat) => {
    const group = cat.group_name || 'Uncategorized';
    if (!acc[group]) acc[group] = [];
    acc[group].push(cat);
    return acc;
  }, {} as Record<string, Category[]>);

  const sortedGroups = Object.keys(groupedCategories).sort((a, b) => {
    if (a === 'Uncategorized') return 1;
    if (b === 'Uncategorized') return -1;
    return a.localeCompare(b);
  });

  const getGroupColor = (cats: Category[]): string => {
    const first = cats[0];
    return first?.parent_color || first?.color || '#95A5A6';
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-24">
        <div className="text-muted-foreground">Loading categories...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {error && (
        <Card variant="outlined" className="border-red-500">
          <CardContent className="py-3">
            <p className="text-sm text-red-600">{error}</p>
            <Button variant="ghost" size="sm" onClick={() => setError(null)} className="mt-1">
              Dismiss
            </Button>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="flex items-center gap-2 text-base">
                <Tag className="h-4 w-4" />
                Categories
              </CardTitle>
              <CardDescription className="text-xs">
                {categories.length} categories in {sortedGroups.length} groups
              </CardDescription>
            </div>
            <Button onClick={() => setShowNewGroup(true)} size="sm">
              <Plus className="mr-1 h-3.5 w-3.5" />
              Add Group
            </Button>
          </div>
        </CardHeader>
        <CardContent className="pt-0">
          {/* New Group Form */}
          {showNewGroup && (
            <div className="rounded-notion border border-accent bg-accent/5 p-3 mb-3 space-y-2">
              <div className="grid gap-2 sm:grid-cols-2">
                <div>
                  <label className="text-[10px] font-medium text-muted-foreground">Group Name</label>
                  <Input
                    value={newGroupName}
                    onChange={(e) => setNewGroupName(e.target.value)}
                    placeholder="Group name"
                    className="mt-0.5 h-8 text-sm"
                    onKeyDown={(e) => e.key === 'Enter' && newGroupName.trim() && createGroup()}
                  />
                </div>
                <div>
                  <label className="text-[10px] font-medium text-muted-foreground">Color</label>
                  <div className="mt-0.5 flex items-center gap-1.5">
                    <input
                      type="color"
                      value={newGroupColor}
                      onChange={(e) => setNewGroupColor(e.target.value)}
                      className="h-8 w-10 cursor-pointer rounded border border-border bg-transparent p-0.5"
                    />
                    <div className="flex gap-0.5 flex-wrap flex-1">
                      {DEFAULT_COLORS.slice(0, 5).map((color) => (
                        <button
                          key={color}
                          onClick={() => setNewGroupColor(color)}
                          className={`h-5 w-5 rounded-full border-2 ${newGroupColor === color ? 'border-foreground' : 'border-transparent'}`}
                          style={{ backgroundColor: color }}
                        />
                      ))}
                    </div>
                  </div>
                </div>
              </div>
              <div className="flex gap-2 justify-end">
                <Button variant="ghost" size="sm" onClick={() => setShowNewGroup(false)} disabled={creatingGroup}>
                  Cancel
                </Button>
                <Button size="sm" onClick={createGroup} disabled={creatingGroup || !newGroupName.trim()}>
                  {creatingGroup ? 'Creating...' : 'Create Group'}
                </Button>
              </div>
            </div>
          )}

          {/* Categories Grid - Responsive columns */}
          <div className="grid gap-2 grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6">
            {sortedGroups.map((groupName) => {
              const cats = groupedCategories[groupName];
              const isExpanded = expandedGroups.has(groupName);
              const groupColor = getGroupColor(cats);
              const isSaving = savingGroup === groupName;

              return (
                <div key={groupName} className="rounded border border-border p-2 bg-card/50">
                  <GroupHeader
                    groupName={groupName}
                    groupColor={groupColor}
                    categoryCount={cats.length}
                    expanded={isExpanded}
                    onToggle={() => toggleGroup(groupName)}
                    onRename={(newName) => handleGroupRename(groupName, newName, cats)}
                    onColorChange={(newColor) => handleGroupColorChange(groupName, newColor, cats)}
                    onDelete={() => setConfirmDeleteGroup(groupName)}
                    saving={isSaving}
                  />
                  {isExpanded && (
                    <div className="pl-4 space-y-0">
                      {cats.map((cat) => (
                        <CategoryRow
                          key={cat.id}
                          category={cat}
                          supabase={supabase}
                          householdId={householdId}
                          onUpdated={fetchCategories}
                          onDeleted={setConfirmDeleteId}
                        />
                      ))}
                      {/* Add Sub-Category Button */}
                      {showNewSubCategory === groupName ? (
                        <div className="mt-2 p-2 rounded border border-border bg-card/30 space-y-2">
                          <Input
                            value={newSubCategoryName}
                            onChange={(e) => setNewSubCategoryName(e.target.value)}
                            placeholder="Sub-category name"
                            className="h-7 text-xs"
                            onKeyDown={(e) => {
                              if (e.key === 'Enter' && newSubCategoryName.trim()) {
                                createSubCategory(groupName);
                              } else if (e.key === 'Escape') {
                                setShowNewSubCategory(null);
                                setNewSubCategoryName('');
                              }
                            }}
                            autoFocus
                          />
                          <div className="flex gap-1 justify-end">
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => {
                                setShowNewSubCategory(null);
                                setNewSubCategoryName('');
                              }}
                              disabled={creatingSubCategory}
                              className="h-6 px-2 text-xs"
                            >
                              Cancel
                            </Button>
                            <Button
                              size="sm"
                              onClick={() => createSubCategory(groupName)}
                              disabled={creatingSubCategory || !newSubCategoryName.trim()}
                              className="h-6 px-2 text-xs"
                            >
                              {creatingSubCategory ? 'Adding...' : 'Add'}
                            </Button>
                          </div>
                        </div>
                      ) : (
                        <button
                          onClick={() => {
                            setShowNewSubCategory(groupName);
                            setNewSubCategoryName('');
                          }}
                          className="mt-2 w-full flex items-center justify-center gap-1.5 py-1.5 text-xs text-muted-foreground hover:text-foreground border border-dashed border-border rounded hover:border-accent/50 transition-colors"
                          title="Add sub-category to this group"
                        >
                          <Plus className="h-3 w-3" />
                          <span>Add Sub-Category</span>
                        </button>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </CardContent>
      </Card>

      {/* Delete Category Confirmation Dialog */}
      {confirmDeleteId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <Card className="w-full max-w-md">
            <CardHeader>
              <CardTitle>Delete Category</CardTitle>
              <CardDescription>
                Are you sure you want to delete this category? Expenses using this category will have their category unset.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex gap-2 justify-end">
                <Button variant="ghost" onClick={() => setConfirmDeleteId(null)} disabled={deleting}>
                  Cancel
                </Button>
                <Button variant="danger" onClick={deleteCategory} disabled={deleting}>
                  {deleting ? 'Deleting...' : 'Delete'}
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Delete Group Confirmation Dialog */}
      {confirmDeleteGroup && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <Card className="w-full max-w-md">
            <CardHeader>
              <CardTitle>Delete Group</CardTitle>
              <CardDescription>
                Are you sure you want to delete the "{confirmDeleteGroup}" group and all {groupedCategories[confirmDeleteGroup]?.length || 0} categories in it? 
                This will delete the categories for this household.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <div className="flex gap-2 justify-end">
                <Button variant="ghost" onClick={() => setConfirmDeleteGroup(null)} disabled={deleting}>
                  Cancel
                </Button>
                <Button variant="danger" onClick={deleteGroup} disabled={deleting}>
                  {deleting ? 'Deleting...' : 'Delete Group'}
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
